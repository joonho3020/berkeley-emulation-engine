use crate::primitives::*;

#[derive(Debug, Clone, Default)]
pub struct Operand {
    pub rs: u32,     // index into data memory
    pub local: bool, // ldm or sdm?
    pub idx: u32,    // for luts, which input does this operand correspond to
}

#[derive(Debug, Clone, Default)]
pub struct SwitchIn {
    pub valid: bool, // valid
    pub idx: u32,    // proc to receive bit from
}

#[derive(Debug, Clone, Default)]
pub struct Instruction {
    pub valid: bool,
    pub opcode: Primitives,
    pub lut: u64,
    pub operands: Vec<Operand>,
    pub sin: SwitchIn,
}
