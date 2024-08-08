use crate::primitives::*;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fmt::Debug;

pub type Bit = u8;
pub type Bits32 = u32;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FourStateBit {
    #[default]
    ZERO,
    ONE,
    X,
    Z,
}

impl FourStateBit {
    pub fn from_char(c: char) -> Self {
        match c {
            '0' => Self::ZERO,
            '1' => Self::ONE,
            'x' => Self::X,
            'z' => Self::Z,
            _ => Self::X,
        }
    }

    pub fn to_bit(self: &Self) -> Option<Bit> {
        match self {
            Self::ZERO => Some(0),
            Self::ONE => Some(1),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Operand {
    pub rs: u32,     // index into data memory
    pub local: bool, // ldm or sdm?
    pub idx: u32,    // for luts, which input does this operand correspond to
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwitchIn {
    pub idx: u32, // proc to receive bit from
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Instruction {
    pub valid: bool,
    pub opcode: Primitives,
    pub lut: u64,
    pub operands: Vec<Operand>,
    pub sin: SwitchIn,
}

impl Instruction {
    pub fn to_bytes(self: &Self, cfg: &Configuration) -> BitBuf {
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
        ret.push_bits(self.sin.idx as u64, cfg.switch_bits());
        return ret;
    }
}

#[derive(Debug, Default, Clone)]
pub struct BitBuf {
    pub bytes: Vec<u8>,
    pub offset: u32,
    pub size: u32,
}

impl BitBuf {
    pub fn push_bits(self: &mut Self, input: u64, nbits: u32) {
        let mut left = nbits;
        while left > 0 {
            if self.offset == 0 {
                self.bytes.push(0);
            }
            let cur_input = (input >> (nbits - left)) as u8;
            let free_bits = 8 - self.offset;
            let consume_bits = min(free_bits, left);

            let last = self.bytes.last_mut().unwrap();
            *last |= (cur_input << self.offset) as u8;

            self.offset = (self.offset + consume_bits) % 8;
            left -= consume_bits;
        }
        self.size += nbits;
    }
}
