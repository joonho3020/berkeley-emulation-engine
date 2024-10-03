use crate::common::config::PlatformConfig;
use serde::{Deserialize, Serialize};
use strum_macros::EnumCount as EnumCountMacro;
use std::{
    fmt::Debug, collections::LinkedList,
};

#[derive(Serialize, Debug, Clone, Default, Eq, Hash, PartialEq, Copy)]
pub struct Coordinate {
    /// module id
    pub module: u32,

    /// processor id
    pub proc: u32
}

impl Coordinate {
    /// Unique ID of this Coordinate in the emulation platform
    pub fn id(self: &Self, pcfg: &PlatformConfig) -> u32 {
        self.module * pcfg.num_procs + self.proc
    }
}

/// Types of communication possible in the emulation platform
#[derive(PartialEq, Debug, Copy, Clone, Default, Deserialize, Serialize, EnumCountMacro)]
pub enum PathTypes {
    #[default]
    ProcessorInternal = 0,
    InterProcessor,
    InterModule,
}

/// Communication path between a parent and child node in the emulation platform
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct NetworkPath {
    pub src: Coordinate,
    pub dst: Coordinate,
    pub tpe: PathTypes
}

impl NetworkPath {
    pub fn new(src: Coordinate, dst: Coordinate) -> Self {
        let tpe = if src == dst {
            PathTypes::ProcessorInternal
        } else if src.module == dst.module {
            PathTypes::InterProcessor
        } else {
            PathTypes::InterModule
        };
        NetworkPath {
            src: src,
            dst: dst,
            tpe: tpe
        }
    }
}

/// List of `NetworkPath` from one processor to another
pub type NetworkRoute = LinkedList<NetworkPath>;
