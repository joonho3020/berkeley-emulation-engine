use crate::common::primitive::*;
use crate::common::config::PlatformConfig;
use crate::common::bitbuf::BitBuf;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

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
    pub opcode: Primitive,

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
            opcode: Primitive::NOP,
            lut: 0,
            operands: Vec::with_capacity(nops as usize),
            sinfo: SwitchInfo::default(),
            mem: false,
        }
    }

    pub fn to_bytes(self: &Self, cfg: &PlatformConfig) -> BitBuf {
        let mut ret = BitBuf::default();
        ret.push_bits(self.opcode as u64, cfg.opcode_bits());
        ret.push_bits(self.lut as u64, cfg.lut_bits());
        for opidx in 0..cfg.lut_inputs {
            match self.operands.get(opidx as usize) {
                Some(op) => {
                    ret.push_bits(op.rs as u64, cfg.index_bits());
                    ret.push_bits(op.local as u64, 1); // local
                }
                None => {
                    ret.push_bits(0, cfg.index_bits()); // rs
                    ret.push_bits(0, 1); // local
                }
            }
        }
        ret.push_bits(self.sinfo.idx as u64, cfg.switch_bits());
        return ret;
    }
}
