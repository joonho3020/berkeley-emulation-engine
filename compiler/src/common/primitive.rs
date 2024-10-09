use crate::common::config::*;
use crate::common::hwgraph::SignalType;
use serde::Serialize;
use strum_macros::EnumCount as EnumCountMacro;
use indexmap::IndexMap;
use std::fmt::Debug;
use blif_parser::primitives::*;

pub type Bit = u8;
pub type Bits = u32;
pub type Cycle = u32;

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

/// Opcodes for the emulator instructions
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, EnumCountMacro)]
#[repr(u32)]
pub enum Opcode {
    #[default]
    NOP = 0,
    Input,
    Output,
    Lut,
    ConstLut,
    Gate,
    Latch,
    SRAMIn,
    SRAMOut,
}

impl From<&Primitive> for Opcode {
    fn from(value: &Primitive) -> Self {
        match value {
            Primitive::NOP          => Opcode::NOP,
            Primitive::Input        => Opcode::Input,
            Primitive::Output       => Opcode::Output,
            Primitive::Lut          => Opcode::Lut,
            Primitive::ConstLut     => Opcode::Lut,
            Primitive::Gate         => Opcode::Gate,
            Primitive::Latch        => Opcode::Latch,
            Primitive::SRAMNode     => Opcode::NOP,
            Primitive::SRAMRdEn     => Opcode::SRAMIn,
            Primitive::SRAMWrEn     => Opcode::SRAMIn,
            Primitive::SRAMRdAddr   => Opcode::SRAMIn,
            Primitive::SRAMWrAddr   => Opcode::SRAMIn,
            Primitive::SRAMWrMask   => Opcode::SRAMIn,
            Primitive::SRAMWrData   => Opcode::SRAMIn,
            Primitive::SRAMRdWrEn   => Opcode::SRAMIn,
            Primitive::SRAMRdWrMode => Opcode::SRAMIn,
            Primitive::SRAMRdWrAddr => Opcode::SRAMIn,
            Primitive::SRAMRdData   => Opcode::SRAMOut
        }
    }
}

/// Same as CircuitPrimitive, except that is doesn't contain fields
/// This is purely to make the compiler code cleaner
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, EnumCountMacro)]
#[repr(u32)]
pub enum Primitive {
    #[default]
    NOP = 0,
    Input,
    Output,
    Lut,
    ConstLut,
    Gate,
    Latch,
    SRAMNode,
    SRAMRdEn,
    SRAMWrEn,
    SRAMRdAddr,
    SRAMRdData,
    SRAMWrAddr,
    SRAMWrMask,
    SRAMWrData,
    SRAMRdWrEn,
    SRAMRdWrMode,
    SRAMRdWrAddr,
}

/// Represents a node in the gate level netlist
#[derive(Debug, Clone, Default, PartialEq, Serialize, EnumCountMacro)]
#[repr(u32)]
pub enum CircuitPrimitive {
    #[default]
    NOP = 0,
    Input        { name: String },
    Output       { name: String },
    Lut          { inputs: Vec<String>, output: String, table: Vec<Vec<u8>> },
    ConstLut     { val: Bit, output: String },
    Gate         { c: String, d: String, q: String, r: Option<String>, e: Option<String> },
    Latch        { input: String, output: String, control: String, init: LatchInit },
    SRAMNode     { name: String, conns: IndexMap<String, String> },
    SRAMRdEn     { name: String },
    SRAMWrEn     { name: String },
    SRAMRdAddr   { name: String, idx: u32 },
    SRAMRdData   { name: String, idx: u32 },
    SRAMWrAddr   { name: String, idx: u32 },
    SRAMWrMask   { name: String, idx: u32 },
    SRAMWrData   { name: String, idx: u32 },
    SRAMRdWrEn   { name: String },
    SRAMRdWrMode { name: String },
    SRAMRdWrAddr { name: String, idx: u32 }
}

impl From<&CircuitPrimitive> for Primitive {
    fn from(value: &CircuitPrimitive) -> Self {
        match value {
            CircuitPrimitive::NOP                 => Primitive::NOP,
            CircuitPrimitive::Input        { .. } => Primitive::Input,
            CircuitPrimitive::Output       { .. } => Primitive::Output,
            CircuitPrimitive::Lut          { .. } => Primitive::Lut,
            CircuitPrimitive::ConstLut     { .. } => Primitive::ConstLut,
            CircuitPrimitive::Gate         { .. } => Primitive::Gate,
            CircuitPrimitive::Latch        { .. } => Primitive::Latch,
            CircuitPrimitive::SRAMNode     { .. } => Primitive::SRAMNode,
            CircuitPrimitive::SRAMRdEn     { .. } => Primitive::SRAMRdEn,
            CircuitPrimitive::SRAMWrEn     { .. } => Primitive::SRAMWrEn,
            CircuitPrimitive::SRAMRdAddr   { .. } => Primitive::SRAMRdAddr,
            CircuitPrimitive::SRAMRdData   { .. } => Primitive::SRAMRdData,
            CircuitPrimitive::SRAMWrAddr   { .. } => Primitive::SRAMWrAddr,
            CircuitPrimitive::SRAMWrMask   { .. } => Primitive::SRAMWrMask,
            CircuitPrimitive::SRAMWrData   { .. } => Primitive::SRAMWrData,
            CircuitPrimitive::SRAMRdWrEn   { .. } => Primitive::SRAMRdWrEn,
            CircuitPrimitive::SRAMRdWrMode { .. } => Primitive::SRAMRdWrMode,
            CircuitPrimitive::SRAMRdWrAddr { .. } => Primitive::SRAMRdWrAddr,
        }
    }
}

impl From<&ParsedPrimitive> for CircuitPrimitive {
    fn from(value: &ParsedPrimitive) -> Self {
        match value {
            ParsedPrimitive::NOP => {
                Self::NOP
            }
            ParsedPrimitive::Module { .. } => {
                Self::NOP
            }
            ParsedPrimitive::Input { name } => {
                Self::Input { name: name.to_string() }
            }
            ParsedPrimitive::Output { name } => {
                Self::Output { name: name.to_string() }
            }
            ParsedPrimitive::Lut { inputs, output, table } => {
                if output == "$true" {
                    Self::ConstLut {
                        val: 1,
                        output: output.to_string()
                    }
                } else if output == "$false" {
                    Self::ConstLut {
                        val: 0,
                        output: output.to_string()
                    }
                } else {
                    Self::Lut {
                        inputs: inputs.to_vec(),
                        output: output.to_string(),
                        table: table.to_vec()
                    }
                }
            }
            ParsedPrimitive::Gate { c, d, q, r, e } => {
                Self::Gate {
                    c: c.clone(),
                    d: d.clone(),
                    q: q.clone(),
                    r: r.clone(),
                    e: e.clone()
                }
            }
            ParsedPrimitive::Latch { input, output, control, init } => {
                Self::Latch {
                    input: input.clone(),
                    output: output.clone(),
                    control: control.clone(),
                    init: init.clone()
                }
            }
            ParsedPrimitive::Subckt { name, conns } => {
                Self::SRAMNode { name: name.clone(), conns: conns.clone() }
            }
        }
    }
}

impl From<&SignalType> for CircuitPrimitive {
    fn from(value: &SignalType) -> Self {
        match value {
            SignalType::NOP =>
                Self::NOP,
            SignalType::Wire { .. } =>
                Self::NOP,
            SignalType::SRAMRdEn { name } =>
                Self::SRAMRdEn { name: name.to_string() },
            SignalType::SRAMWrEn { name } =>
                Self::SRAMWrEn { name: name.to_string() },
            SignalType::SRAMRdAddr { name, idx } =>
                Self::SRAMRdAddr { name: name.to_string(), idx: idx.clone() },
            SignalType::SRAMRdData { name, idx } =>
                Self::SRAMRdData { name: name.to_string(), idx: idx.clone() },
            SignalType::SRAMWrData { name, idx } =>
                Self::SRAMWrData { name: name.to_string(), idx: idx.clone() },
            SignalType::SRAMWrMask { name, idx } =>
                Self::SRAMWrMask { name: name.to_string(), idx: idx.clone() },
            SignalType::SRAMWrAddr { name, idx } =>
                Self::SRAMWrAddr { name: name.to_string(), idx: idx.clone() },
            SignalType::SRAMRdWrEn { name } =>
                Self::SRAMRdWrEn { name: name.to_string() },
            SignalType::SRAMRdWrMode { name } =>
                Self::SRAMRdWrMode { name: name.to_string() },
            SignalType::SRAMRdWrAddr { name, idx } =>
                Self::SRAMRdWrAddr { name: name.to_string(), idx: idx.clone() }
        }
    }
}

impl CircuitPrimitive {
    pub fn unique_sram_input_offset(self: &Self, pcfg: &PlatformConfig) -> u32 {
        match self {
            Self::SRAMRdEn     { .. } => pcfg.sram_rd_en_offset(),
            Self::SRAMWrEn     { .. } => pcfg.sram_wr_en_offset(),
            Self::SRAMRdAddr   { .. } => pcfg.sram_rd_addr_offset(),
            Self::SRAMWrAddr   { .. } => pcfg.sram_wr_addr_offset(),
            Self::SRAMWrData   { .. } => pcfg.sram_wr_data_offset(),
            Self::SRAMWrMask   { .. } => pcfg.sram_wr_mask_offset(),
            Self::SRAMRdWrEn   { .. } => pcfg.sram_rdwr_en_offset(),
            Self::SRAMRdWrMode { .. } => pcfg.sram_rdwr_mode_offset(),
            Self::SRAMRdWrAddr { .. } => pcfg.sram_rdwr_addr_offset(),
            _ => pcfg.sram_other_offset()
        }
    }

    pub fn unique_sram_input_idx(self: &Self, pcfg: &PlatformConfig) -> u32 {
        let offset = self.unique_sram_input_offset(pcfg);
        match self {
            Self::SRAMRdEn     { name:_ }      => offset,
            Self::SRAMWrEn     { name:_ }      => offset,
            Self::SRAMRdAddr   { name:_, idx } => offset + idx,
            Self::SRAMWrAddr   { name:_, idx } => offset + idx,
            Self::SRAMWrMask   { name:_, idx } => offset + idx,
            Self::SRAMWrData   { name:_, idx } => offset + idx,
            Self::SRAMRdWrEn   { name:_ }      => offset,
            Self::SRAMRdWrMode { name:_ }      => offset,
            Self::SRAMRdWrAddr { name:_, idx } => offset + idx,
            _ => offset
        }
    }

    pub fn unique_sram_output_idx(self: &Self, pcfg: &PlatformConfig) -> u32 {
        let w = pcfg.sram_width;
        match self {
            Self::SRAMRdData { name:_, idx } => *idx,
            _ => w
        }
    }
}
