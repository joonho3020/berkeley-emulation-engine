use crate::common::primitive::*;
use crate::common::config::PlatformConfig;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;
use bitvec::vec::BitVec;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Operand {
    /// index into data memory
    pub rs: Bits,

    /// ldm or sdm?
    pub local: bool,

    /// for luts, which input does this operand correspond to
    pub idx: Bits,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SwitchInfo {
    /// Set when the `local` field has been already set (for correctness checks)
    pub local_set: bool,

    /// Set when the `fwd` field has been already set (for correctness checks)
    pub fwd_set: bool,

    /// proc to receive bit from
    pub idx: Bits,

    /// Receive from local switch
    pub local: bool,

    /// forward the incomming bit
    pub fwd: bool
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Instruction {
    /// This instruction is performing something
    pub valid: bool,

    /// Opcode
    pub opcode: Opcode,

    /// LUT table
    pub lut: u64,

    /// Index into LDM or SDM
    pub operands: Vec<Operand>,

    /// Information related to switching
    pub sinfo: SwitchInfo,

    /// This is a memory op (SRAM input bit)
    /// When set, use operands[1:] to indicate which IO bit this is
    pub mem: bool
}

impl Instruction {
    pub fn new(nops: u32) -> Self {
        Instruction {
            valid: false,
            opcode: Opcode::NOP,
            lut: 0,
            operands: Vec::with_capacity(nops as usize),
            sinfo: SwitchInfo::default(),
            mem: false,
        }
    }

    pub fn to_bits(self: &Self, cfg: &PlatformConfig) -> BitVec {
        let mut bit_vec = BitVec::new();

        let opcode = self.opcode as u32;
        for i in 0..cfg.opcode_bits() {
            let sl = cfg.opcode_bits() - i - 1;
            bit_vec.push((opcode >> sl) & 1 == 1);
        }

        for i in 0..cfg.lut_bits() {
            let sl = cfg.lut_bits() - i - 1;
            bit_vec.push((self.lut >> sl) & 1 == 1);
        }

        for opidx in (0..cfg.lut_inputs).rev() {
            let (rs, local) = match self.operands.get(opidx as usize) {
                Some(op) => {
                    (op.rs, op.local)
                }
                None => {
                    (0, true)
                }
            };
            for i in 0..cfg.index_bits() {
                let sl = cfg.index_bits() - i - 1;
                bit_vec.push((rs >> sl) & 1 == 1);
            }
            bit_vec.push(local);
        }

        for i in 0..cfg.switch_bits() {
            let sl = cfg.switch_bits() - i - 1;
            bit_vec.push((self.sinfo.idx >> sl) & 1 == 1);
        }
        bit_vec.push(self.sinfo.local);
        bit_vec.push(self.sinfo.fwd  );
        bit_vec.push(self.mem        );
        return bit_vec;
    }

    pub fn ports_used(self: &Self) -> (Option<u32>, Option<u32>) {
        (self.ldm_ports_used(), self.sdm_ports_used())
    }

    fn ldm_ports_used(self: &Self) -> Option<u32> {
        self.operands.iter()
            .map(|x| if x.local { 1 } else { 0 })
            .reduce(|a, b| a + b)
    }

    fn sdm_ports_used(self: &Self) -> Option<u32> {
        self.operands.iter()
            .map(|x| if x.local { 0 } else { 1 })
            .reduce(|a, b| a + b)
    }
}
