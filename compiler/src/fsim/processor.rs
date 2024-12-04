use crate::common::{config::*, primitive::*, instruction::*};
use crate::fsim::memory::*;
use crate::fsim::sram::ProcessorSRAMPort;
use std::fmt::Debug;

#[derive(Default, Clone, Debug)]
struct ProcessorSwitchPort {
    ip: Bit,
    op: Bit,
}

#[derive(Clone)]
pub struct Processor {
    /// Processor id
    pub processor_id: u32,
    pub cfg: PlatformConfig,
    pub host_steps: Bits,
    pub target_cycle: Cycle,

    /// Program counter
    pub pc: Bits,

    /// Instruction memory
    pub imem: AbstractMemory<Instruction>,

    /// Local data memory
    pub ldm: AbstractMemory<Bit>,

    /// Switch data memory
    pub sdm: AbstractMemory<Bit>,

    /// For pipelining instructions
    pipeline: Vec<Instruction>,

    /// IO input port
    io_i: Bit,

    /// IO output port
    io_o: Bit,

    /// Local switch port (within a `Module`)
    sw_loc: ProcessorSwitchPort,

    /// Receive bit from `sin_idx` from Local switch
    sin_idx: u32,

    /// Global switch port (within a `Board`)
    sw_glb: ProcessorSwitchPort,

    /// true when receiving from `sw_loc` instead of `sw_glb` & vice versa
    sin_local: bool,

    /// Bit to send when we need to forward the bit to the network instead of
    /// the computed bit. Used when inst.sinfo.fwd is true
    sin_fwd_bit: Bit,

    sram_port: ProcessorSRAMPort,


    pub dbg_ldm_wbit: Bit,
    pub dbg_sdm_wbit: Bit
}

impl Processor {
    pub fn new(id_: u32, host_steps_: Bits, cfg: &PlatformConfig) -> Self {
        Processor {
            cfg: cfg.clone(),
            host_steps: host_steps_,
            imem: AbstractMemory::new(cfg.max_steps as u32, cfg.imem_lat,    1,              cfg.imem_lat,    1),
            ldm:  AbstractMemory::new(cfg.max_steps as u32, cfg.dmem_rd_lat, cfg.lut_inputs, cfg.dmem_wr_lat, 1),
            sdm:  AbstractMemory::new(cfg.max_steps as u32, cfg.dmem_rd_lat, cfg.lut_inputs, cfg.dmem_wr_lat, 1),
            pipeline: vec![Instruction::default(); cfg.dmem_rd_lat as usize],
            io_i: 0,
            io_o: 0,
            pc: 0,
            target_cycle: 0,
            sw_loc: ProcessorSwitchPort::default(),
            sw_glb: ProcessorSwitchPort::default(),
            sin_local: false,
            sin_fwd_bit: 0,
            sin_idx: 0,
            sram_port: ProcessorSRAMPort::default(),
            processor_id: id_,
            dbg_ldm_wbit: 0,
            dbg_sdm_wbit: 0
        }
    }

    pub fn set_inst(self: &mut Self, inst: Instruction, step: usize) {
        assert!(step < self.imem.data.len());
        self.imem[step] = inst;
    }

    /// Processor pipeline
    /// 1. Fetch
    /// 2. Read LDM & SDM
    /// 3. Various things
    ///    - Get LDM & SDM outputs
    ///    - Compute output
    ///    - Ship output to switch
    ///    - Writeback to LDM
    ///    - Recv from switch and writeback to SDM
    pub fn fetch(self: &mut Self) {
        assert!(self.pipeline.len() as Cycle == self.cfg.dmem_rd_lat);

        // ---------------------  Fetch  ---------------------------------
        // Send imem req
        self.imem.get_rport(0).submit_req(ReadReq{ addr: self.pc });

        // --------------------- Read DMem ---------------------------------
        // Get instruction for imem
        self.imem.update_rd_ports();
        let opt_fd_inst = self.imem.get_rport(0).cur_resp();
        let fd_inst = match opt_fd_inst {
            Some(resp) => resp.data,
            None => Instruction::default()
        };

        // Submit read requests to the data memory using fetched instruction
        for i in 0..self.cfg.lut_inputs {
            let rs = match &fd_inst.operands.get(i as usize) {
                Some(op) => op.rs,
                None => 0
            };
            self.ldm.get_rport(i).submit_req(ReadReq{ addr: rs as Bits });
            self.sdm.get_rport(i).submit_req(ReadReq{ addr: rs as Bits });
        }

        self.pipeline.push(fd_inst.clone());
    }

    pub fn compute(self: &mut Self) {

        // --------------------- Compute ---------------------------------
        let de_inst = self.pipeline.remove(0);

        // Read the operands from the LDM and SDM
        let mut operands: Vec<Bit> = Vec::new();
        self.ldm.update_rd_ports();
        self.sdm.update_rd_ports();
        for i in 0..self.cfg.lut_inputs {
            let ldm_resp = match self.ldm.get_rport(i as u32).cur_resp() {
                Some(resp) => resp.data,
                None       => 0
            };
            let sdm_resp = match self.sdm.get_rport(i as u32).cur_resp() {
                Some(resp) => resp.data,
                None       => 0
            };
            let bit = match de_inst.operands.get(i as usize) {
                Some(op) => if op.local { ldm_resp } else { sdm_resp },
                None     => ldm_resp
            };
            operands.push(bit);
        }

        // Set sin_idx to receive from switch
        self.sin_idx = de_inst.sinfo.idx;
        self.sin_local = de_inst.sinfo.local;

        // LUT lookup
        let f_out = match &de_inst.opcode {
            Opcode::NOP => 0,
            Opcode::Input => self.io_i,
            Opcode::Lut => {
                let mut entry = 0;
                for (i, bit) in operands.iter().enumerate() {
                    entry = entry + (bit << i);
                }
                ((de_inst.lut >> entry) & 1) as u8
            }
            Opcode::Output => {
                let bit = *operands.get(0).unwrap();
                self.io_o = bit;
                bit
            }
            Opcode::Gate | Opcode::Latch => {
                *operands.get(0).unwrap()
            }
            Opcode::SRAMOut => {
                self.sram_port.op
            }
            Opcode::SRAMIn => {
                *operands.get(0).unwrap()
            }
            _ => 0,
        };

        self.sram_port.ip = f_out;

        // Write to LDM
        if self.pc as Cycle >= self.cfg.fetch_decode_lat() {
            self.ldm.get_wport(0).submit_req(WriteReq {
                addr: self.pc - (self.cfg.fetch_decode_lat() as Bits),
                data: f_out
            });
        }

        // Set switch out
        if de_inst.sinfo.fwd {
            self.sw_loc.op  = self.sin_fwd_bit;
            self.sw_glb.op = self.sin_fwd_bit;
        } else {
            self.sw_loc.op  = f_out;
            self.sw_glb.op = f_out;
        }

        self.ldm.run_cycle();
        self.imem.run_cycle();

        self.dbg_ldm_wbit = if self.pc >= self.cfg.fetch_decode_lat() { f_out } else { 0 };
    }

    pub fn update_sram_in(self: &mut Self) {
        let de_inst = self.pipeline.get(0).unwrap();

        // Collect rs values that is sent to the SRAM processor
        let mut sram_idx = 0;
        for i in 1..self.cfg.lut_inputs {
            let rs = match de_inst.operands.get(i as usize) {
                Some(op) => op.rs,
                None => 0
            };
            let sr = self.cfg.index_bits() * (i - 1);
            sram_idx |= rs << sr;

            assert!(sr < 32 || rs == 0,
                "Cannot represent sram_idx using u32, sr: {} rs: {}", sr, rs);
        }

        // Set SRAM port values
        self.sram_port.val = de_inst.mem as Bit;
        self.sram_port.idx = sram_idx;
    }

    pub fn update_sdm_and_pc(self: &mut Self) {
        let sdm_store_bit = if self.sin_local {
            self.sw_loc.ip
        } else {
            self.sw_glb.ip
        };

        // Update SDM
        if self.pc as u32 >= self.cfg.fetch_decode_lat() {
            self.sdm.get_wport(0).submit_req(WriteReq {
                addr: (self.pc as u32) - self.cfg.fetch_decode_lat(),
                data: sdm_store_bit
            });
        }

        self.sin_fwd_bit = sdm_store_bit;

        self.sdm.run_cycle();

        // Increment step
        if self.pc == self.host_steps - 1 {
            self.target_cycle += 1;
            self.pc = 0;
        } else {
            self.pc += 1;
        }

        self.dbg_sdm_wbit = if self.pc >= self.cfg.fetch_decode_lat() { sdm_store_bit } else { 0 };
    }

    pub fn get_switch_in_id(self: &Self) -> Bits {
        self.sin_idx
    }

    pub fn set_local_switch_in(self: &mut Self, b: Bit) {
        self.sw_loc.ip = b;
    }

    pub fn get_local_switch_out(self: &mut Self) -> Bit {
        self.sw_loc.op
    }

    pub fn set_global_switch_in(self: &mut Self, b: Bit) {
        self.sw_glb.ip = b;
    }

    pub fn get_global_switch_out(self: &mut Self) -> Bit {
        self.sw_glb.op
    }

    pub fn set_io_i(self: &mut Self, x: Bit) {
        self.io_i = x
    }

    pub fn get_io_o(self: &mut Self) -> Bit {
        self.io_o
    }

    pub fn set_sram_out(self: &mut Self, b: Bit) {
        self.sram_port.op = b;
    }

    pub fn get_sram_in_val(self: &Self) -> Bit {
        self.sram_port.val
    }

    pub fn get_sram_in_idx(self: &Self) -> Bits {
        self.sram_port.idx
    }

    pub fn get_sram_in_ip(self: &Self) -> Bit {
        self.sram_port.ip
    }

    fn print_bitvec(self: &Self, bitvec: &Vec<Bit>) {
        let mut hex_bits = vec![];
        for chunk in bitvec.chunks(64) {
            let mut hex: u64 = 0;
            for (i, b) in chunk.iter().enumerate() {
                hex |= (*b as u64) << i;
            }
            hex_bits.push(hex);
        }
        print!("0x");
        for h in hex_bits.iter().rev() {
            print!("{:x},", h);
        }
        print!("\n");
    }

    pub fn print_ldm(self: &Self) {
        self.print_bitvec(&self.ldm.data)
    }

    pub fn print_sdm(self: &Self) {
        self.print_bitvec(&self.sdm.data)
    }
}

impl Debug for Processor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  ldm: ")?;
        for chunk in self.ldm.data.chunks(16) {
            write!(f, "\t{:x?}\n", chunk)?;
        }

        write!(f, "  sdm: ")?;
        for chunk in self.sdm.data.chunks(16) {
            write!(f, "\t{:x?}\n", chunk)?;
        }
        Ok(())
    }
}
