use crate::common::Instruction;
use crate::fsim::module::Module as EmulModule;
use crate::utils::write_string_to_file;
use indexmap::IndexMap;
use petgraph::{
    dot::{Config, Dot},
    graph::{Graph, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::{
    cmp::{max, Ordering},
    fmt::Debug,
    fs::File,
    io::Write,
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
        assert!(
            self.table.len() <= 6,
            "Can support up to 6 operands with u64 Lut {} Table {:?}",
            self.name(),
            self.table
        );

        for entry in self.table.iter() {
            let mut x = 0;
            for (i, e) in entry.iter().enumerate() {
                x = x + (e << i);
            }
            table = table | (1 << x);
        }
        write!(f, "Lut 0x{:x} {:?}", table, self.info)
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

/// # Context
/// - Configuration of the underlying hardware emulation platform
#[derive(Debug, Clone, Serialize)]
pub struct Configuration {
    /// Maximum host steps that can be run
    pub max_steps: u32,
    /// Number of processor in a module
    pub module_sz: u32,
    /// Number of lut inputs
    pub lut_inputs: u32,
    /// Latency of the switch network
    pub network_lat: u32,
    /// Numer of cycles to perform
    pub compute_lat: u32,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            max_steps: 128,
            module_sz: 64,
            lut_inputs: 3,
            network_lat: 0,
            compute_lat: 0,
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

    /// Cost function for performing local compute within a processor
    pub fn compute_cost(self: &Self) -> u32 {
        self.compute_lat + 1
    }

    /// Cost function for performing inter-processor communication
    pub fn network_cost(self: &Self) -> u32 {
        self.network_lat + 2
    }
}

/// # EmulatorInfo
/// - Contains fields specific to the emulator hardware
#[derive(Serialize, Debug, Default, Clone)]
pub struct EmulatorInfo {
    pub cfg: Configuration,
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

    pub fn proc_subgraph(&self, proc_id: u32) -> Graph<&Box<dyn HWNode>, &String> {
        return self.graph.filter_map(
            |_, y| {
                if y.clone().get_info().proc == proc_id {
                    Some(y)
                } else {
                    None
                }
            },
            |_, y| Some(y),
        );
    }

    pub fn save_all_subgraphs(&self, file_pfx: String) -> std::io::Result<()> {
        // save main graph
        let mut file = File::create(format!("{}-tot.dot", file_pfx))?;
        write!(
            &mut file,
            "{:?}",
            Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
        )?;

        // save subgraphs
        let mut max_proc = 0;
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight(nidx).unwrap();
            max_proc = max(max_proc, node.clone().get_info().proc);
        }

        for proc_id in 0..(max_proc + 1) {
            let psg = self.proc_subgraph(proc_id);
            let mut file = File::create(format!("{}-{}.dot", file_pfx, proc_id))?;
            write!(
                &mut file,
                "{:?}",
                Dot::with_config(&psg, &[Config::EdgeNoLabel])
            )?;
        }
        Ok(())
    }

    pub fn save_emulator_info(&self, file_path: String) -> std::io::Result<()> {
        write_string_to_file(serde_json::to_string_pretty(&self.emulator)?, &file_path)
    }

    pub fn save_emulator_instructions(&self, file_path: String) -> std::io::Result<()> {
        let mut inst_str = "".to_string();
        for (pi, proc_insts) in self.emulator.instructions.iter().enumerate() {
            inst_str.push_str(&format!("------------ processor {} ------------\n", pi));
            for (i, inst) in proc_insts.iter().enumerate() {
                if (i as u32) < self.emulator.host_steps {
                    inst_str.push_str(&format!("{} {:?}\n", i, inst));
                } else {
                    break;
                }
            }
        }
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

    pub fn topo_sorted_nodes(&self) -> Vec<NodeIndex> {
        // Search for flip-flop nodes
        let mut ff_nodes: Vec<NodeIndex> = vec![];
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight(nidx).unwrap();
            match node.is() {
                Primitives::Latch | Primitives::Gate => {
                    ff_nodes.push(nidx);
                }
                _ => {}
            }
        }

        // Push Input & FF nodes
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in self.io_i.keys() {
            q.push(*nidx);
        }
        for nidx in ff_nodes.iter() {
            q.push(*nidx);
        }

        // compute indeg
        let mut indeg: IndexMap<NodeIndex, u32> = IndexMap::new();
        for nidx in self.graph.node_indices() {
            indeg.insert(nidx, 0);
        }
        for eidx in self.graph.edge_indices() {
            let e = self.graph.edge_endpoints(eidx).unwrap();
            let dst = e.1;
            *indeg.get_mut(&dst).unwrap() += 1;
        }

        // BFS
        let mut topo_sort_order: Vec<NodeIndex> = vec![];
        let mut vis_map = self.graph.visit_map();
        while !q.is_empty() {
            let nidx = q.remove(0);
            vis_map.visit(nidx);
            topo_sort_order.push(nidx);

            let mut childs = self.graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&self.graph) {
                *indeg.get_mut(&cidx).unwrap() -= 1;
                if *indeg.get(&cidx).unwrap() == 0 && !vis_map.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }
        return topo_sort_order;
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
