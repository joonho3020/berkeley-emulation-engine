use crate::common::{
    primitive::{Primitive, Bit, Bits},
    config::*
};
use crate::fsim::memory::*;
use std::fmt::Debug;


#[derive(Default, Debug, Clone)]
struct SRAMEntry {
    pub bits: Vec<Bit>
}

impl SRAMEntry {
    fn new(width: u32) -> Self {
        SRAMEntry {
            bits: vec![0; width as usize]
        }
    }
}

#[derive(Default, Debug, Clone)]
struct SRAMInputs {
    pub rd_en: Bit,
    pub wr_en: Bit,
    pub rd_addr: Bits,
    pub wr_addr: Bits,
    pub wr_data: Vec<Bit>,
    pub wr_mask: Vec<Bit>,
}

impl SRAMInputs {
    fn new(width_bits: u32) -> Self {
        SRAMInputs {
            rd_en: 0,
            wr_en: 0,
            rd_addr: 0,
            wr_addr: 0,
            wr_data: vec![0; width_bits as usize],
            wr_mask: vec![0; width_bits as usize]
        }
    }

    fn init(self: &mut Self) {
        self.rd_en = 0;
        self.wr_en = 0;
        self.rd_addr = 0;
        self.wr_addr = 0;
        for e in &mut self.wr_data {
            *e = 0;
        }
        for e in &mut self.wr_mask {
            *e = 0;
        }
    }

    fn set_rd_en(self: &mut Self, ibit: Bit) {
        self.rd_en = ibit;
    }

    fn set_wr_en(self: &mut Self, ibit: Bit) {
        self.wr_en = ibit;
    }

    fn set_rd_addr(self: &mut Self, ibit: Bit, idx: u32) {
        self.rd_addr |= (ibit as u32) << idx;
    }

    fn set_wr_addr(self: &mut Self, ibit: Bit, idx: u32) {
        self.wr_addr |= (ibit as u32) << idx;
    }

    fn set_wr_data(self: &mut Self, ibit: Bit, idx: u32) {
        let bit = self.wr_data.get_mut(idx as usize).unwrap();
        *bit = ibit;
    }

    fn set_wr_mask(self: &mut Self, ibit: Bit, idx: u32) {
        let bit = self.wr_mask.get_mut(idx as usize).unwrap();
        *bit = ibit;
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProcessorSRAMPort {
    pub val: Bit,
    pub idx: Bits,
    pub ip: Bit,
    pub op: Bit
}

pub struct SRAMProcessor {
    pub id: u32,
    pub pc: u32,
    pub host_steps: u32,
    pub pcfg: PlatformConfig,
    pub ports: Vec<ProcessorSRAMPort>,
    cur: u32,
    inputs: Vec<SRAMInputs>,
    sram: AbstractMemory<SRAMEntry>
}

impl SRAMProcessor {
    pub fn new(id_: u32, host_steps_: u32, cfg: &PlatformConfig) -> Self {
        assert!(cfg.sram_rd_ports == 1, "Currently only support single rd ported SRAM");
        assert!(cfg.sram_wr_ports == 1, "Currently only support single wr ported SRAM");

        let mut ret = SRAMProcessor {
            id: id_,
            pc: 0,
            host_steps: host_steps_,
            pcfg: cfg.clone(),
            cur: 0,
            ports: vec![ProcessorSRAMPort::default(); cfg.num_procs as usize],
            inputs: vec![SRAMInputs::new(cfg.sram_width); 2],
            sram: AbstractMemory::new(
                cfg.sram_entries,
                cfg.sram_rd_lat, cfg.sram_rd_ports,
                cfg.sram_wr_lat, cfg.sram_wr_ports)
        };
        for addr in 0..cfg.sram_entries {
            ret.sram[addr as usize] = SRAMEntry::new(cfg.sram_width);
        }
        return ret;
    }

    fn recv_input_idx(self: &Self) -> u32 {
        (self.cur + 1) % 2
    }

    fn use_input_idx(self: &Self) -> u32 {
        self.cur as u32
    }

    // - update output ports
    pub fn set_sram_out(self: &mut Self) {
        // Get value from the current read data port
        self.sram.update_rd_ports();
        let rd_resp = match self.sram.get_rport(0).cur_resp() {
            Some(resp) => resp.data,
            None => SRAMEntry::new(self.pcfg.sram_width)
        };
        println!("rd_resp: {:?}", rd_resp);

        // Set the output port
        for p in self.ports.iter_mut() {
            let bit_pos = p.idx as usize;
            println!("bit_pos: {}", bit_pos);
            match rd_resp.bits.get(bit_pos) {
                Some(bit) => { p.op = *bit; }
                None => { }
            }
        }
    }

    // - update input ports
    // - send out SRAM Rd/Wr request
    // - run_cycle
    pub fn run_cycle(self: &mut Self) {
        println!("SRAM {} run cycle", self.id);

        // Receive inputs and update input regs
        for p in self.ports.iter() {
            if p.val != 0 {
                let (prim, bit_pos) = self.pcfg.index_to_sram_input_type(p.idx);
                println!("SRAM port: {:?} prim: {:?} bit_pos: {:?}",
                    p, prim, bit_pos);
                let ridx = self.recv_input_idx() as usize;
                let input = self.inputs.get_mut(ridx).unwrap();
                match prim {
                    Primitive::SRAMRdEn   => { input.set_rd_en(p.ip); }
                    Primitive::SRAMWrEn   => { input.set_wr_en(p.ip); }
                    Primitive::SRAMRdAddr => { input.set_rd_addr(p.ip, bit_pos); }
                    Primitive::SRAMWrAddr => { input.set_wr_addr(p.ip, bit_pos); }
                    Primitive::SRAMWrData => { input.set_wr_data(p.ip, bit_pos); }
                    Primitive::SRAMWrMask => { input.set_wr_mask(p.ip, bit_pos); }
                    _ => {}
                }
            }
        }

        let uidx = self.use_input_idx() as usize;
        let cur_input = self.inputs.get(uidx).unwrap();

        // Send out SRAM read request
        self.sram.get_rport(0).submit_req(ReadReq {
            addr: cur_input.rd_addr
        });

        println!("SRAM Read Req: addr: {}", cur_input.rd_addr);

        // Send out SRAM write request
        if cur_input.wr_en != 0 {
            // TODO: Write mask?
            self.sram.get_wport(0).submit_req(WriteReq {
                addr: cur_input.wr_addr,
                data: SRAMEntry { bits: cur_input.wr_data.clone() }
            });
            println!("SRAM Write Req: addr: {} data: {:?}",
                cur_input.wr_addr, cur_input.wr_data);
        }

        // Update SRAM state
        self.sram.run_cycle();

        println!("SRAM Data {:?}", self.sram.data);

        // Update PC
        if self.pc == self.host_steps - 1 {
            self.pc = 0;
            self.inputs.get_mut(self.cur as usize).unwrap().init();
            self.cur = (self.cur + 1) % 2;
        } else {
            self.pc += 1;
        }
    }
}
