use crate::common::{
    config::*, mapping::{SRAMMapping, SRAMPortType}, primitive::{Bit, Bits, Primitive}
};
use crate::fsim::memory::*;
use std::fmt::Debug;


#[derive(Default, Clone)]
pub struct SRAMEntry {
    pub bits: Vec<Bit>
}

impl SRAMEntry {
    pub fn new(width: u32) -> Self {
        SRAMEntry {
            bits: vec![0; width as usize]
        }
    }

    pub fn to_u64_vec(self: &Self) -> Vec<u64> {
        let mut result = Vec::new();
        let mut current = 0u64;

        for (i, bit) in self.bits.iter().enumerate() {
            if *bit > 0{
                current |= 1 << (i % 64);
            }
            if i % 64 == 63 || i == self.bits.len() - 1 {
                result.push(current);
                current = 0;
            }
        }
        result
    }

    pub fn bit(self: &Self, idx: u32) -> Bit {
        self.bits.get(idx as usize).unwrap().clone()
    }
}

impl Debug for SRAMEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_u64_vec())
    }
}

#[derive(Default, Debug, Clone)]
pub struct SRAMInputs {
    pub rd_en: Bit,
    pub wr_en: Bit,
    pub rd_addr: Bits,
    pub wr_addr: Bits,
    pub wr_data: Vec<Bit>,
    pub wr_mask: Vec<Bit>,
}

impl SRAMInputs {
    pub fn new(width_bits: u32) -> Self {
        SRAMInputs {
            rd_en: 0,
            wr_en: 0,
            rd_addr: 0,
            wr_addr: 0,
            wr_data: vec![0; width_bits as usize],
            wr_mask: vec![0; width_bits as usize]
        }
    }

    pub fn init(self: &mut Self) {
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

    pub fn set_rd_en(self: &mut Self, ibit: Bit) {
        self.rd_en = ibit;
    }

    pub fn set_wr_en(self: &mut Self, ibit: Bit) {
        self.wr_en = ibit;
    }

    pub fn set_rd_addr(self: &mut Self, ibit: Bit, idx: u32) {
        self.rd_addr |= (ibit as u32) << idx;
    }

    pub fn set_wr_addr(self: &mut Self, ibit: Bit, idx: u32) {
        self.wr_addr |= (ibit as u32) << idx;
    }

    pub fn set_wr_data(self: &mut Self, ibit: Bit, idx: u32) {
        let bit = self.wr_data.get_mut(idx as usize).unwrap();
        *bit = ibit;
    }

    pub fn set_wr_mask(self: &mut Self, ibit: Bit, idx: u32) {
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
    mapping: SRAMMapping,
    cur: u32,
    inputs: Vec<SRAMInputs>,
    prev_input: SRAMInputs,
    cur_rd_data: SRAMEntry,
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
            mapping: SRAMMapping::default(),
            inputs: vec![SRAMInputs::new(cfg.sram_width); 2],
            prev_input: SRAMInputs::new(cfg.sram_width),
            cur_rd_data: SRAMEntry::new(cfg.sram_width),
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

    pub fn print(self: &Self) {
        if self.mapping.width_bits > 0 {
            println!("{:?}", self.sram);
        }
    }

    pub fn set_sram_mapping(self: &mut Self, map: &SRAMMapping) {
        println!("Emulating SRAM {:?} with {} wmask bits, {} bits per entry",
            map.port_type, map.wmask_bits, map.width_bits);

        self.mapping = map.clone();
    }

    fn recv_input_idx(self: &Self) -> u32 {
        (self.cur + 1) % 2
    }

    fn use_input_idx(self: &Self) -> u32 {
        self.cur as u32
    }

    fn masked_write_data(self: &Self, sram_input: &SRAMInputs, rd_data: &SRAMEntry) -> Vec<Bit> {
        assert!(sram_input.wr_data.len() == self.pcfg.sram_width as usize,
            "Number of wr_data bits {} != sram width {}",
            sram_input.wr_data.len(), self.pcfg.sram_width);

        assert!(sram_input.wr_data.len() == rd_data.bits.len(),
            "Number of wr_data bits {} != rd_data.bits bits {}",
            sram_input.wr_data.len(), rd_data.bits.len());

        assert!(sram_input.wr_mask.len() == rd_data.bits.len(),
            "Number of wr_mask bits {} != rd_data.bits bits {}",
            sram_input.wr_mask.len(), rd_data.bits.len());

        assert!(self.mapping.wmask_bits > 0,
            "masked_write_data should only be called for SRAMs w/ wmask bits");

        let num_bits_per_mask = self.mapping.width_bits / self.mapping.wmask_bits;
        let mut mask = vec![0u8; self.pcfg.sram_width as usize];
        for i in 0..self.mapping.wmask_bits {
            let mask_value = sram_input.wr_mask.get(i as usize).unwrap();
            for j in 0..num_bits_per_mask {
                let idx = i * num_bits_per_mask + j;
                *mask.get_mut(idx as usize).unwrap() = *mask_value;
            }
        }

        assert!(mask.len() == sram_input.wr_data.len(),
            "mask {:?}, expected length: {}, num_masks: {} num_bits_per_mask: {}",
            mask, sram_input.wr_data.len(), self.mapping.wmask_bits, num_bits_per_mask);

        let mut ret = vec![];
        for ((m, w), r) in mask.iter().zip(sram_input.wr_data.iter()).zip(rd_data.bits.iter()) {
            if *m == 0 {
                ret.push(*r);
            } else {
                ret.push(*w);
            }
        }

        assert!(ret.len() == sram_input.wr_data.len(),
            "masked write data: {:?}, expected length: {}",
            ret, sram_input.wr_data.len());

        return ret;
    }

    // - update output ports
    pub fn set_sram_out(self: &mut Self) {
        // Get value from the current read data port
        self.sram.update_rd_ports();
        let rd_resp = match self.sram.get_rport(0).cur_resp() {
            Some(resp) => resp.data,
            None => SRAMEntry::new(self.pcfg.sram_width),
        };

        self.cur_rd_data = rd_resp.clone();

        // Set the output port
        for p in self.ports.iter_mut() {
            let bit_pos = p.idx as usize;
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
        // Receive inputs and update input regs
        for p in self.ports.iter() {
            if p.val != 0 {
                let (prim, bit_pos) = self.pcfg.index_to_sram_input_type(p.idx);
                let ridx = self.recv_input_idx() as usize;
                let input = self.inputs.get_mut(ridx).unwrap();
                match prim {
                    Primitive::SRAMRdEn     => { input.set_rd_en(p.ip); }
                    Primitive::SRAMWrEn     => { input.set_wr_en(p.ip); }
                    Primitive::SRAMRdAddr   => { input.set_rd_addr(p.ip, bit_pos); }
                    Primitive::SRAMWrAddr   => { input.set_wr_addr(p.ip, bit_pos); }
                    Primitive::SRAMWrData   => { input.set_wr_data(p.ip, bit_pos); }
                    Primitive::SRAMWrMask   => { input.set_wr_mask(p.ip, bit_pos); }
                    Primitive::SRAMRdWrEn   => { input.set_rd_en(p.ip); }
                    Primitive::SRAMRdWrMode => { input.set_wr_en(p.ip); }
                    Primitive::SRAMRdWrAddr => { input.set_rd_addr(p.ip, bit_pos); }
                    _ => {}
                }
            }
        }

        let uidx = self.use_input_idx() as usize;
        let cur_input = self.inputs.get(uidx).unwrap();

        // Check if this request should be a read or a write
        let (wen, waddr) = match self.mapping.port_type {
            SRAMPortType::OneRdOneWrPortSRAM => {
                (cur_input.wr_en != 0, cur_input.wr_addr)
            }
            SRAMPortType::SinglePortSRAM => {
                (cur_input.wr_en != 0 && cur_input.rd_en != 0, cur_input.rd_addr)
            }
        };

        let ren = match self.mapping.port_type {
            SRAMPortType::OneRdOneWrPortSRAM => {
                cur_input.rd_en != 0
            }
            SRAMPortType::SinglePortSRAM => {
                cur_input.wr_en == 0 && cur_input.rd_en != 0
            }
        };

        // If read enable is high, read from the current input
        // otherwise, use the address from the previous input
        let raddr = if ren {
            cur_input.rd_addr
        } else {
            self.prev_input.rd_addr
        };

        if wen && self.pc == 0 {
            // Write request, need to read the current value in the write address
            // to emulate write mask behavior
            self.sram.get_rport(0).submit_req(ReadReq {
                addr: waddr
            });
        } else {
            // Send out SRAM read request
            self.sram.get_rport(0).submit_req(ReadReq {
                addr: raddr
            });
        }

        // Send out SRAM write request
        if wen && self.pc == self.pcfg.sram_wr_lat {
            let wdata = if self.mapping.wmask_bits == 0 {
                // No write mask for this SRAM
                cur_input.wr_data.clone()
            } else {
                // Write mask for this SRAM
                self.masked_write_data(cur_input, &self.cur_rd_data)
            };
            self.sram.get_wport(0).submit_req(WriteReq {
                addr: waddr,
                data: SRAMEntry { bits: wdata }
            });
        }

        // Update SRAM state
        self.sram.run_cycle();

        // Update PC
        if self.pc == self.host_steps - 1 {
            self.pc = 0;
            if ren {
                self.prev_input = cur_input.clone();
            }
            self.inputs.get_mut(self.cur as usize).unwrap().init();
            self.cur = (self.cur + 1) % 2;
        } else {
            self.pc += 1;
        }
    }
}
