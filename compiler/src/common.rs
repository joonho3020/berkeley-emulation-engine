use crate::primitives::*;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use serde_json::to_string;


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
