use crate::common::network::*;
use crate::common::primitive::*;
use petgraph::graph::{Graph, NodeIndex};
use serde::Serialize;
use strum_macros::EnumCount as EnumCountMacro;
use serde::ser::SerializeStruct;
use serde::Serializer;
use std::fmt::Debug;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NodeCheckState {
    #[default]
    Unknown = 0,
    Match,
    Mismatch
}

#[derive(Debug, Clone, Default)]
pub struct DebugInfo {
    pub check: NodeCheckState
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct RankInfo {
    /// Rank of the node assigned during forward pass topo sort
    pub asap: u32,

    /// Rank of the node assigned during backward pass topo sort
    pub alap: u32,

    /// Mobility of this node
    pub mob:  u32,
}

/// # Metadata attached to each `HWGraph` node
#[derive(Debug, Clone, Default)]
pub struct NodeInfo {
    /// Module and processor id that this node is mapped to
    pub coord: Coordinate,

    /// rank order index
    pub rank: RankInfo,

    /// true if a imem slot has been allocated for this instruction
    pub scheduled: bool,

    /// index to the allocated imem slot
    pub pc: u32,

    /// register group that this node is in
    pub reggrp: u32,

    /// Debug information
    pub debug: DebugInfo
}

impl Serialize for NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Node", 4)?;
        state.serialize_field("module", &self.coord.module)?;
        state.serialize_field("proc", &self.coord.proc)?;
        state.serialize_field("rank.asap", &self.rank.asap)?;
        state.serialize_field("rank.alap", &self.rank.alap)?;
        state.serialize_field("scheduled", &self.scheduled)?;
        state.serialize_field("pc", &self.pc)?;
        state.end()
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeMapInfo {
    pub info: NodeInfo,
    pub idx: NodeIndex,
}

impl Serialize for NodeMapInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("NodeMapInfo", 1)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct HWNode {
    /// CircuitPrimitive from the blif_parser
    pub prim: CircuitPrimitive,

    /// Information that we will fill in or use during the compiler passes
    pub info: NodeInfo
}

/// # Interface for accessing/manipulating the underlying node in `HWGraph`
impl HWNode {
    pub fn new(prim: CircuitPrimitive) -> Self {
        HWNode {
            prim: prim,
            info: NodeInfo::default()
        }
    }

    /// # Returns the `Primitive` enum so that we can check for types
    pub fn is(&self) -> Primitive {
        Primitive::from(&self.prim)
    }

    pub fn info(&self) -> &NodeInfo {
        &self.info
    }

    pub fn info_mut(&mut self) -> &mut NodeInfo {
        &mut self.info
    }

    pub fn name(&self) -> &str {
        match &self.prim {
            CircuitPrimitive::Lut { inputs: _, output, .. }  => &output,
            CircuitPrimitive::ConstLut { val:_, output }     => &output,
            CircuitPrimitive::Gate { c: _, d: _, q, .. }     => &q,
            CircuitPrimitive::Input { name }                 => &name,
            CircuitPrimitive::Output { name }                => &name,
            CircuitPrimitive::Latch { input: _, output, .. } => &output,
            CircuitPrimitive::SRAMRdEn { name }              => &name,
            CircuitPrimitive::SRAMWrEn { name }              => &name,
            CircuitPrimitive::SRAMRdAddr { name, .. }        => &name,
            CircuitPrimitive::SRAMRdData { name, .. }        => &name,
            CircuitPrimitive::SRAMWrAddr { name, .. }        => &name,
            CircuitPrimitive::SRAMWrMask { name, .. }        => &name,
            CircuitPrimitive::SRAMWrData { name, .. }        => &name,
            _ => ""
        }
    }
}

pub type HWGraph = Graph<HWNode, HWEdge>;

#[derive(Clone, Default, PartialEq, Serialize, EnumCountMacro)]
#[repr(u32)]
pub enum SignalType {
    #[default]
    NOP = 0,
    Wire         { name: String },
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

impl Debug for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SignalType::NOP                          => "",
            SignalType::Wire         { name        } => name,
            SignalType::SRAMRdEn     { name        } => name,
            SignalType::SRAMWrEn     { name        } => name,
            SignalType::SRAMRdAddr   { name, idx:_ } => name,
            SignalType::SRAMRdData   { name, idx:_ } => name,
            SignalType::SRAMWrAddr   { name, idx:_ } => name,
            SignalType::SRAMWrMask   { name, idx:_ } => name,
            SignalType::SRAMWrData   { name, idx:_ } => name,
            SignalType::SRAMRdWrEn   { name        } => name,
            SignalType::SRAMRdWrAddr { name, idx:_ } => name,
            SignalType::SRAMRdWrMode { name        } => name,
        };
        write!(f, "{}", name)
    }
}

/// # Metadata attached to each `HWGraph` edge
#[derive(Debug, Clone, Default)]
pub struct HWEdge {
    /// Type of signal
    pub signal: SignalType,

    /// For inter-module communication, set to describe how the bit is routed
    /// over the global network.
    /// For communication that happens within a module, this is set to None.
    pub route: Option<NetworkRoute>,
}

impl HWEdge {
    pub fn new(s: SignalType) -> Self {
        HWEdge {
            signal: s,
            route: None,
        }
    }

    pub fn set_routing(self: &mut Self, route: NetworkRoute) {
        self.route = Some(route);
    }
}
