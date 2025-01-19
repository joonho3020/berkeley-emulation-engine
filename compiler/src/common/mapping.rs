use crate::common::instruction::*;
use crate::common::hwgraph::*;
use serde::Serialize;
use indexmap::IndexMap;

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular emulator processor
#[derive(Serialize, Debug, Default, Clone)]
pub struct ProcessorMapping {
    /// Generated instructions for this module
    pub instructions: Vec<Instruction>,

    /// Signal mapping info
    pub signal_map: IndexMap<String, NodeMapInfo>,
}

/// Supported SRAM port types
#[derive(Serialize, Debug, Default, Clone)]
pub enum SRAMPortType {
    #[default]
    OneRdOneWrPortSRAM = 0,
    SinglePortSRAM
}

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular sram processor
#[derive(Serialize, Debug, Default, Clone)]
pub struct SRAMMapping {
    /// Type of SRAM
    pub port_type: SRAMPortType,

    /// Number of bits used in write mask field
    pub wmask_bits: u32,

    /// Number of bits per target SRAM entry
    pub width_bits: u32
}

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular emulator module
#[derive(Serialize, Debug, Default, Clone)]
pub struct ModuleMapping {
    /// Per processor emulation mapping information
    pub proc_mappings: IndexMap<u32, ProcessorMapping>,

    /// SRAM processor mapping information
    pub sram_mapping: SRAMMapping,
}

/// # MappingInfo
/// - Contains fields specific to the emulator hardware
#[derive(Serialize, Debug, Default, Clone)]
pub struct EmulatorMapping {
    /// Number of host steps to emulate a single cycle
    pub host_steps: u32,

    /// Maximum rank of this design
    pub max_rank: u32,

    /// Maximum slack (ALAP - ASAP) of this design
    pub max_slack: u32,

    /// Per module emulation mapping information
    pub module_mappings: IndexMap<u32, ModuleMapping>
}
