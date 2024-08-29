use crate::common::*;
use crate::fsim::module::Module as EmulModule;
use crate::utils::write_string_to_file;
use indexmap::IndexMap;
use petgraph::{
    graph::{Graph, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt::Debug,
};
use strum::EnumCount;
use strum_macros::EnumCount as EnumCountMacro;

pub type HWGraph = Graph<Box<dyn HWNode>, String>;

/// # Metadata attached to each `HWGraph` node
/// - proc: the processor id that this node is mapped to
/// - rank: rank order index
/// - scheduled: true if a imem slot has been allocated for this instruction
/// - pc: index to the allocated imem slot
#[derive(Debug, Clone, Default)]
pub struct NodeInfo {
    pub proc: u32,
    pub rank: u32,
    pub scheduled: bool,
    pub pc: u32,
}

impl Serialize for NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("NodeMapInfo", 4)?;
        state.serialize_field("proc", &self.proc)?;
        state.serialize_field("rank", &self.rank)?;
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

#[derive(PartialEq, Debug, Copy, Clone, Default, Deserialize, Serialize, EnumCountMacro)]
pub enum Primitives {
    #[default]
    NOP = 0,
    Input,
    Output,
    Lut,
    Gate,
    Latch,
    Subckt,
    Module,
}

/// # Interface for accessing/manipulating the underlying node in `HWGraph`
pub trait HWNode: Debug {
    fn box_clone(&self) -> Box<dyn HWNode>;

    /// # Returns the `Primitives` enum so that we can check for types
    fn is(&self) -> Primitives;
    fn set_info(&mut self, info: NodeInfo);
    fn get_info(&self) -> NodeInfo;
    fn get_lut(&self) -> Option<Lut>;
    fn name(&self) -> &str;
}

impl Clone for Box<dyn HWNode> {
    fn clone(&self) -> Box<dyn HWNode> {
        self.box_clone()
    }
}

impl PartialEq for Box<dyn HWNode> {
    fn eq(&self, other: &Self) -> bool {
        if self.is() == other.is() {
            self.get_info().rank == other.get_info().rank
        } else {
            false
        }
    }
}

impl Eq for Box<dyn HWNode> {}

impl Ord for Box<dyn HWNode> {
    fn cmp(&self, other: &Self) -> Ordering {
        if (self.is() == Primitives::Input) && (other.is() == Primitives::Input)
            || (self.is() != Primitives::Input) && (other.is() != Primitives::Input)
        {
            return self.get_info().rank.cmp(&other.get_info().rank);
        } else if self.is() == Primitives::Input {
            return Ordering::Less;
        } else {
            return Ordering::Greater;
        }
    }
}

impl PartialOrd for Box<dyn HWNode> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct Input {
    pub name: String,
    pub info: NodeInfo,
}

impl HWNode for Input {
    fn is(&self) -> Primitives {
        return Primitives::Input;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Input {} {:?}", self.name, self.info)
    }
}

#[derive(Clone)]
pub struct Output {
    pub name: String,
    pub info: NodeInfo,
}

impl HWNode for Output {
    fn is(&self) -> Primitives {
        return Primitives::Output;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Output {} {:?}", self.name, self.info)
    }
}

#[derive(Clone)]
pub struct Lut {
    pub inputs: Vec<String>,
    pub output: String,
    pub table: Vec<Vec<u8>>,
    pub info: NodeInfo,
}

impl HWNode for Lut {
    fn is(&self) -> Primitives {
        return Primitives::Lut;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        Some(self.clone())
    }

    fn name(&self) -> &str {
        &self.output
    }
}

impl Debug for Lut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut table: u64 = 0;

        if self.table.len() > 0 {
            assert!(
                self.table[0].len() <= 6,
                "Can support up to 6 operands with u64 Lut {} Table {:?}",
                self.name(),
                self.table
            );
        }

        for entry in self.table.iter() {
            let mut x = 0;
            for (i, e) in entry.iter().enumerate() {
                x = x + (e << i);
            }
            table = table | (1 << x);
        }
        write!(f, "Lut {} 0x{:x} {:?}", &self.name(), table, self.info)
    }
}

#[derive(Debug, Clone)]
pub struct Subckt {
    pub name: String,
    pub conns: IndexMap<String, String>,
    pub info: NodeInfo,
}

impl HWNode for Subckt {
    fn is(&self) -> Primitives {
        return Primitives::Subckt;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone)]
pub struct Gate {
    pub c: String,
    pub d: String,
    pub q: String,
    pub r: Option<String>,
    pub e: Option<String>,

    pub info: NodeInfo,
}

impl Default for Gate {
    fn default() -> Gate {
        Gate {
            c: "".to_string(),
            d: "".to_string(),
            q: "".to_string(),
            r: None,
            e: None,
            info: NodeInfo::default(),
        }
    }
}

impl HWNode for Gate {
    fn is(&self) -> Primitives {
        return Primitives::Gate;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.q
    }
}

impl Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gate {:?}", self.info)
    }
}

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum LatchInit {
    /// Defined in Yosys spec
    ZER0 = 0,
    ONE = 1,
    DONTCARE = 2,
    UNKNOWN = 3,
}

impl LatchInit {
    pub fn to_enum(i: &str) -> LatchInit {
        match i {
            "0" => LatchInit::ZER0,
            "1" => LatchInit::ONE,
            "2" => LatchInit::DONTCARE,
            _ => LatchInit::UNKNOWN,
        }
    }
}

#[derive(Clone)]
pub struct Latch {
    pub input: String,
    pub output: String,
    pub control: String,
    pub init: LatchInit,
    pub info: NodeInfo,
}

impl Default for Latch {
    fn default() -> Latch {
        Latch {
            input: "".to_string(),
            output: "".to_string(),
            control: "".to_string(),
            init: LatchInit::UNKNOWN,
            info: NodeInfo::default(),
        }
    }
}

impl HWNode for Latch {
    fn is(&self) -> Primitives {
        return Primitives::Latch;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.output
    }
}

impl Debug for Latch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Latch {} {:?}", self.output, self.info)
    }
}

#[derive(Clone)]
pub struct Module {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub luts: Vec<Lut>,
    pub subckts: Vec<Subckt>,
    pub gates: Vec<Gate>,
    pub latches: Vec<Latch>,
    pub info: NodeInfo,
}

impl HWNode for Module {
    fn is(&self) -> Primitives {
        return Primitives::Module;
    }

    fn box_clone(&self) -> Box<dyn HWNode> {
        Box::new((*self).clone())
    }

    fn set_info(&mut self, info: NodeInfo) {
        self.info = info;
    }

    fn get_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn get_lut(&self) -> Option<Lut> {
        None
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module {}\n", self.name)?;

        write!(f, "  inputs: ")?;
        for i in self.inputs.iter() {
            write!(f, "  {} ", i)?;
        }
        write!(f, "\n")?;

        write!(f, "  outputs: ")?;
        for i in self.outputs.iter() {
            write!(f, "  {} ", i)?;
        }
        write!(f, "\n")?;

        for i in self.luts.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        for i in self.subckts.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        for i in self.gates.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        write!(f, "")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KaMinParConfig {
    pub seed: u64,
    pub epsilon: f64,
    pub nthreads: u32,
}

impl Default for KaMinParConfig {
    fn default() -> Self {
        KaMinParConfig {
            seed: 123,
            epsilon: 0.03,
            nthreads: 16
        }
    }
}

/// # Context
/// - Configuration of the underlying hardware emulation platform
#[derive(Debug, Clone, Serialize)]
pub struct Configuration {
    /// Maximum host steps that can be run
    pub max_steps: u32,

    /// Number of processor in a module
    pub module_sz: u32,

    /// Number of lut inputs
    pub lut_inputs: Cycle,

    /// Latency of the switch network
    pub network_lat: Cycle,

    /// Number of cycles to access i-mem
    pub imem_lat: Cycle,

    /// Number of cycles to read d-mem
    pub dmem_rd_lat: Cycle,

    /// Number of cycles to write d-mem
    pub dmem_wr_lat: Cycle,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            max_steps: 128,
            module_sz: 64,
            lut_inputs: 3,
            network_lat: 0,
            imem_lat: 0,
            dmem_rd_lat: 0,
            dmem_wr_lat: 1,
        }
    }
}

impl Configuration {
    fn power_of_2(self: &Self, v: u32) -> bool {
        return v & (v - 1) == 0;
    }

    fn log2ceil(self: &Self, v: u32) -> u32 {
        let log2x = u32::BITS - v.leading_zeros();
        if self.power_of_2(v) {
            log2x - 1
        } else {
            log2x
        }
    }

    /// log2Ceil(self.max_steps)
    pub fn index_bits(self: &Self) -> u32 {
        self.log2ceil(self.max_steps)
    }

    /// log2Ceil(self.module_sz)
    pub fn switch_bits(self: &Self) -> u32 {
        self.log2ceil(self.module_sz)
    }

    /// log2Ceil(number of Primitives)
    pub fn opcode_bits(self: &Self) -> u32 {
        // FIXME: Currently subtracting 2 to exclude Subckt and Module
        let num_prims: u32 = Primitives::COUNT as u32 - 2;
        self.log2ceil(num_prims)
    }

    /// number of bits for the LUT
    pub fn lut_bits(self: &Self) -> u32 {
        1 << self.lut_inputs
    }

    /// - I can start using bits computed from a local processor at
    /// `local.pc + local_dep_lat`
    ///   <me> | read imem | read dmem | compute + write dmem |
    ///   <me>                                    | read imem | read dmem | compute
    pub fn local_dep_lat(self: &Self) -> Cycle {
        self.dmem_rd_lat + self.dmem_wr_lat
    }

    /// - I can start using bits computed from a remote processor at
    /// `remote.pc + remote_dep_lat`.
    ///   <other> | read imem | read dmem | lut + network | write dmem |
    ///   <me>                                             | read imem | read dmem | compute |
    pub fn remote_dep_lat(self: &Self) -> Cycle {
        self.dmem_rd_lat + self.network_lat + self.dmem_wr_lat
    }

    /// - I have to receive a incoming bit from a remote processor at
    /// `remote.pc + remote_sin_lat`
    /// <other> | read imem | read dmem | compute | network |
    /// <me>                        | read imem | read dmem | compute + write sdm |
    pub fn remote_sin_lat(self: &Self) -> Cycle {
        self.network_lat
    }

    /// If the current pc is `X`, store the current local compute result in
    /// `X - pc_ldm_offset`
    pub fn pc_ldm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat
    }

    /// If the current pc is `X`, store the current switch compute result in
    /// `X - pc_sdm_offset`
    pub fn pc_sdm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat + self.network_lat
    }
}

/// # EmulatorInfo
/// - Contains fields specific to the emulator hardware
#[derive(Serialize, Debug, Default, Clone)]
pub struct EmulatorInfo {
    /// Configuration of the emulation HW
    pub cfg: Configuration,
    pub kaminpar: KaMinParConfig,
    pub host_steps: u32,
    pub used_procs: u32,
    pub instructions: Vec<Vec<Instruction>>,
    pub signal_map: IndexMap<String, NodeMapInfo>,
}

#[derive(Default, Clone)]
pub struct Circuit {
    pub graph: HWGraph,
    pub io_i: IndexMap<NodeIndex, String>, // Nodes that represent the input IO port
    pub io_o: IndexMap<NodeIndex, String>, // Nodes that represent the output IO port
    pub emulator: EmulatorInfo,
}

impl Circuit {
    pub fn set_cfg(&mut self, cfg: Configuration) {
        self.emulator.cfg = cfg;
    }

    pub fn save_emulator_info(&self, file_path: String) -> std::io::Result<()> {
        write_string_to_file(serde_json::to_string_pretty(&self.emulator)?, &file_path)
    }

    pub fn save_emulator_instructions(&self, file_path: &str) -> std::io::Result<()> {
        let mut nops = 0;
        let total = self.emulator.host_steps * self.emulator.used_procs;

        let mut inst_str = "".to_string();
        for (pi, proc_insts) in self.emulator.instructions.iter().enumerate() {
            inst_str.push_str(&format!("------------ processor {} ------------\n", pi));
            for (i, inst) in proc_insts.iter().enumerate() {
                if (i as u32) < self.emulator.host_steps {
                    inst_str.push_str(&format!("{} {:?}\n", i, inst));
                    match inst.opcode {
                        Primitives::NOP => nops += 1,
                        _ => ()
                    };
                } else {
                    break;
                }
            }
        }
        inst_str.push_str(&format!("Overal stats\nNOPs: {}\nTotal insts: {}\nUtilization: {}%\n",
                                   nops, total, ((total - nops) as f32)/(total as f32) * 100 as f32));
        write_string_to_file(inst_str, &file_path)
    }

    /// #debug_graph
    /// Given a `dbg_node` in the graph, search for all parents nodes up until
    /// it reaches Gate, Latch or Input.
    /// It will also print the bit-value associated with the node
    /// computed by the emulation processor.
    pub fn debug_graph(&self, dbg_node: NodeIndex, module: &EmulModule) -> String {
        let indent: &str = "    ";
        let mut vis_map = self.graph.visit_map();
        let mut q = vec![];
        q.push(dbg_node);
        let mut root = true;

        while !q.is_empty() {
            let nidx = q.remove(0);
            vis_map.visit(nidx);

            let node = self.graph.node_weight(nidx).unwrap();
            if node.is() == Primitives::Gate || node.is() == Primitives::Latch {
                if !root {
                    continue;
                } else {
                    root = false;
                }
            }

            let mut parents = self.graph.neighbors_directed(nidx, Incoming).detach();
            while let Some(pidx) = parents.next_node(&self.graph) {
                q.push(pidx);
            }
        }

        let mut outstring = "digraph {\n".to_string();

        // print nodes
        for nidx in self.graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                let node = self.graph.node_weight(nidx).unwrap();
                let val = module.peek(node.name()).unwrap();
                outstring.push_str(&format!(
                    "{}{} [ label = \"{:?} {}\"]\n",
                    indent,
                    nidx.index(),
                    node,
                    val
                ));
            }
        }

        // print edges
        for nidx in self.graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                let mut childs = self.graph.neighbors_directed(nidx, Outgoing).detach();
                while let Some(cidx) = childs.next_node(&self.graph) {
                    if vis_map.is_visited(&cidx) {
                        outstring.push_str(&format!(
                            "{}{} {} {} \n",
                            indent,
                            nidx.index(),
                            "->",
                            cidx.index()
                        ));
                    }
                }
            }
        }
        outstring.push_str("}");
        return outstring;
    }

    pub fn print_scheduled(&self) -> String {
        let mut outstring = "digraph {\n".to_string();
        let indent: &str = "    ";

        let mut vis_map = self.graph.visit_map();
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in self.io_i.keys() {
            q.push(*nidx);
        }

        while !q.is_empty() {
            let nidx = q.remove(0);
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);
            let node = self.graph.node_weight(nidx).unwrap();
            let color = match node.get_info().scheduled {
                true => "blue",
                _    => "red",
            };
            outstring.push_str(
                &format!("{} {}[ color = \"{}\"]\n",
                         indent,
                         nidx.index(),
                         color)
            );

            let mut childs = self.graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&self.graph) {
                if !vis_map.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }

        for (_, edge) in self.graph.edge_references().enumerate() {
            outstring.push_str(
                &format!("{}{} {} {} ",
                         indent,
                         edge.source().index(),
                         "->",
                         edge.target().index())
            );
            outstring.push_str("[ ]");
        }
        outstring.push_str("}");
        return outstring;
    }
}

impl Debug for Circuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent: &str = "    ";
        let graph = &self.graph;
        let io_i = &self.io_i;

        // Push Input nodes
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in io_i.keys() {
            q.push(*nidx);
        }

        write!(f, "digraph {{\n")?;

        // BFS
        let mut vis_map = graph.visit_map();
        while !q.is_empty() {
            let nidx = q.remove(0);
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);
            let node = graph.node_weight(nidx).unwrap();
            // red, blue, green, white, purple
            let proc = node.clone().get_info().proc % 5;
            let color = match proc {
                0 => "red",
                1 => "blue",
                2 => "green",
                3 => "orange",
                4 => "purple",
                _ => "white",
            };
            write!(
                f,
                "{}{} [ label = \"{:?}\" color = \"{}\"]\n",
                indent,
                nidx.index(),
                node,
                color
            )?;

            let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&graph) {
                if !vis_map.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }

        for (_, edge) in graph.edge_references().enumerate() {
            write!(
                f,
                "{}{} {} {} ",
                indent,
                edge.source().index(),
                "->",
                edge.target().index(),
            )?;
            writeln!(f, "[ ]")?;
        }

        write!(f, "}}")
    }
}
