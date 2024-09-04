use crate::common::*;
use crate::fsim::module::Module as EmulModule;
use crate::utils::write_string_to_file;
use derivative::Derivative;
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

pub type HWGraph = Graph<Box<dyn HWNode>, HWEdge>;

pub fn get_nodes_type(graph: &HWGraph, nodetype: Primitives) -> Vec<NodeIndex> {
    let mut nodes: Vec<NodeIndex> = vec![];
    for nidx in graph.node_indices() {
        let node = graph.node_weight(nidx).unwrap();
        if node.is() == nodetype {
            nodes.push(nidx);
        }
    }
    return nodes;
}

#[derive(Serialize, Debug, Clone, Default, Eq, Hash, PartialEq, Copy)]
pub struct Coordinate {
    /// module id
    pub module: u32,

    /// processor id
    pub proc: u32
}

impl Coordinate {
    pub fn id(self: &Self, pcfg: &PlatformConfig) -> u32 {
        self.module * pcfg.num_procs + self.proc
    }
}

pub type InterModulePath = (Coordinate, Coordinate);

/// # Metadata attached to each `HWGraph` edge
#[derive(Debug, Clone, Default)]
pub struct HWEdge {
    pub name: String,
    pub path: Option<InterModulePath>,
}

impl HWEdge {
    pub fn new(name_: String) -> Self {
        HWEdge {
            name: name_,
            path: None
        }
    }

    pub fn set_path(self: &mut Self, path: InterModulePath) {
        self.path = Some(path);
    }
}

/// # Metadata attached to each `HWGraph` node
#[derive(Debug, Clone, Default)]
pub struct NodeInfo {
    /// Module and processor id that this node is mapped to
    pub coord: Coordinate,

    /// rank order index
    pub rank: u32,

    /// true if a imem slot has been allocated for this instruction
    pub scheduled: bool,

    /// index to the allocated imem slot
    pub pc: u32,

    /// register group that this node is in
    pub reggrp: u32,
}

impl Serialize for NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("NodeMapInfo", 4)?;
        state.serialize_field("module", &self.coord.module)?;
        state.serialize_field("proc", &self.coord.proc)?;
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

#[derive(Clone, Default, Serialize)]
pub struct GlobalNetworkTopology {
    pub edges: IndexMap<Coordinate, Coordinate>,
    pub inter_mod_paths: IndexMap<(u32, u32), Vec<(Coordinate, Coordinate)>>
}

impl GlobalNetworkTopology {
    pub fn new(num_mods: u32, num_procs: u32) -> Self {
        let mut ret = GlobalNetworkTopology::default();
        if num_mods == 1 {
            return ret;
        }
        let num_mods_1 = num_mods - 1;
        let grp_sz = num_procs / num_mods_1;

        assert!(num_mods_1 & (num_mods_1 - 1) == 0, "num_mods should be 2^n + 1");
        assert!(num_procs  & (num_procs - 1)  == 0, "num_procs should be 2^n + 1");
        assert!(num_procs >= num_mods_1, "num_procs {} < num_mods - 1 {}", num_procs, num_mods_1);

        for m in 0..num_mods_1 {
            for p in 0..num_procs {
                let r = p % grp_sz;
                let q = (p - r) / grp_sz;
                let src = Coordinate { module: m, proc: p };
                let dst = if q == m {
                    let dm = num_mods_1;
                    let dp = p;
                    Coordinate { module: dm, proc: dp }
                } else {
                    let dm = q;
                    let dp = m * grp_sz + r;
                    Coordinate { module: dm, proc: dp }
                };
                ret.edges.insert(src, dst);
                ret.edges.insert(dst, src);
                ret.add_path(src, dst);
                ret.add_path(dst, src);
            }
        }
        return ret;
    }

    fn add_path(self: &mut Self, src: Coordinate, dst: Coordinate) {
        if !self.inter_mod_paths.contains_key(&(src.module, dst.module)) {
            self.inter_mod_paths.insert((src.module, dst.module), vec![]);
        }
        if !self.inter_mod_paths.contains_key(&(dst.module, src.module)) {
            self.inter_mod_paths.insert((dst.module, src.module), vec![]);
        }
        let paths = self.inter_mod_paths.get_mut(&(src.module, dst.module)).unwrap();
        paths.push((src, dst));
    }

    pub fn inter_mod_paths(self: &Self, src: Coordinate, dst: Coordinate) -> Vec<(Coordinate, Coordinate)> {
        let paths = self.inter_mod_paths.get(&(src.module, dst.module)).unwrap();
        return paths.to_vec();
    }
}

impl Debug for GlobalNetworkTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent: &str = "    ";

        write!(f, "digraph {{\n")?;

        let mut map: IndexMap<Coordinate, u32> = IndexMap::new();

        for (i, (src, _)) in self.edges.iter().enumerate() {
            map.insert(*src, i as u32);

            write!(
                f,
                "{}{} [ label = \"{:?}\" ]\n",
                indent,
                i,
                src
            )?;
        }
        for (i, (_, dst)) in self.edges.iter().enumerate() {
            write!(
                f,
                "{}{} {} {} ",
                indent,
                i,
                "->",
                map.get(dst).unwrap()
            )?;
            writeln!(f, "[ ]")?;
        }

        write!(f, "}}")
    }
}

/// # Context
/// - Config of the underlying hardware emulation platform
#[derive(Clone, Serialize, Derivative)]
#[derivative(Debug)]
pub struct PlatformConfig {
    /// Num modules
    pub num_mods: u32,

    /// Number of processor in a module
    pub num_procs: u32,

    /// Maximum host steps that can be run
    pub max_steps: u32,

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

    /// Global network topology
    #[derivative(Debug="ignore")]
    pub topology: GlobalNetworkTopology
}

impl Default for PlatformConfig {
    fn default() -> Self {
        PlatformConfig {
            num_mods: 1,
            num_procs: 64,
            max_steps: 128,
            lut_inputs: 3,
            network_lat: 0,
            imem_lat: 0,
            dmem_rd_lat: 0,
            dmem_wr_lat: 1,
            topology: GlobalNetworkTopology::default()
        }
    }
}

impl PlatformConfig {
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

    /// log2Ceil(self.num_procs)
    pub fn switch_bits(self: &Self) -> u32 {
        self.log2ceil(self.num_procs)
    }

    /// log2Ceil(number of Primitives)
    pub fn opcode_bits(self: &Self) -> u32 {
        // NOTE: Currently subtracting 2 to exclude Subckt and Module
        let num_prims: u32 = Primitives::COUNT as u32 - 2;
        self.log2ceil(num_prims)
    }

    /// number of bits for the LUT
    pub fn lut_bits(self: &Self) -> u32 {
        1 << self.lut_inputs
    }

    /// - I can start using bits computed from a local processor at
    /// `local.pc + intra_proc_dep_lat`
    ///   <me> | read imem | read dmem | compute + write dmem |
    ///   <me>                                    | read imem | read dmem | compute
    pub fn intra_proc_dep_lat(self: &Self) -> Cycle {
        self.dmem_rd_lat + self.dmem_wr_lat
    }

    /// - I can start using bits computed from a remote processor at
    /// `remote.pc + inter_proc_dep_lat`.
    ///   <other> | read imem | read dmem | lut + network | write dmem |
    ///   <me>                                             | read imem | read dmem | compute |
    pub fn inter_proc_dep_lat(self: &Self) -> Cycle {
        self.dmem_rd_lat + self.network_lat + self.dmem_wr_lat
    }

    // TODO
    /// Bit travels from SRC -> DST
    pub fn inter_mod_zerohop_dep_lat(self: &Self) -> Cycle {
        1
    }

    /// Bit travels from SRC -> TMP (same module with SRC) -> DST
    pub fn inter_mod_local_onehop_dep_lat(self: &Self) -> Cycle {
        2
    }

    /// Bit travels from SRC -> TMP (same module with DST) -> DST
    pub fn inter_mod_remote_onehop_dep_lat(self: &Self) -> Cycle {
        2
    }

    /// Bit travels from SRC -> TMP 1 (same mod w/ SRC) -> TMP2 (same mod w/ DST) -> DST
    pub fn inter_mod_twohop_dep_lat(self: &Self) -> Cycle {
        3
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

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular emulator module
#[derive(Serialize, Debug, Default, Clone)]
pub struct ModuleMapping {
    pub used_procs: u32,
    pub instructions: Vec<Vec<Instruction>>,
    pub signal_map: IndexMap<String, NodeMapInfo>,
}

/// # MappingInfo
/// - Contains fields specific to the emulator hardware
#[derive(Serialize, Debug, Default, Clone)]
pub struct EmulatorMapping {
    pub host_steps: u32,
    pub used_mods: u32,
    pub mod_mappings: IndexMap<u32, ModuleMapping>
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct CompilerConfig {
    pub top_module: String,
    pub output_dir: String
}

#[derive(Default, Clone)]
pub struct Circuit {
    pub compiler_cfg: CompilerConfig,
    pub platform_cfg: PlatformConfig,
    pub kaminpar_cfg: KaMinParConfig,

    pub graph: HWGraph,
    pub emul:  EmulatorMapping
}

impl Circuit {
    pub fn set_cfg(&mut self, pcfg: PlatformConfig, ccfg: CompilerConfig) {
        self.platform_cfg = pcfg;
        self.compiler_cfg = ccfg;
    }

    pub fn save_emulator_info(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.info", self.compiler_cfg.output_dir, self.compiler_cfg.top_module);
        for (i, mapping) in self.emul.mod_mappings.iter() {
            let mut out = file_path.clone();
            out.push_str(&format!("-{}", i));
            write_string_to_file(serde_json::to_string_pretty(&mapping)?, &out)?;
        }
        Ok(())
    }

    pub fn save_emulator_instructions(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.insts", self.compiler_cfg.output_dir, self.compiler_cfg.top_module);
        let mut inst_str = "".to_string();
        let mut total_insns = 0;
        let mut total_nops = 0;
        for (i, mapping) in self.emul.mod_mappings.iter() {
            total_insns += self.emul.host_steps * mapping.used_procs;
            inst_str.push_str(&format!("============ module {} ============\n", i));

            for (pi, proc_insts) in mapping.instructions.iter().enumerate() {
                inst_str.push_str(&format!("------------ processor {} ------------\n", pi));
                for (i, inst) in proc_insts.iter().enumerate() {
                    if (i as u32) < self.emul.host_steps {
                        inst_str.push_str(&format!("{} {:?}\n", i, inst));
                        match inst.opcode {
                            Primitives::NOP => total_nops += 1,
                            _ => ()
                        };
                    } else {
                        break;
                    }
                }
            }
        }
        inst_str.push_str(&format!("Overall stats\nNOPs: {}\nTotal insts: {}\nUtilization: {}%\n",
                                   total_nops,
                                   total_insns,
                                   ((total_insns - total_nops) as f32)/(total_insns as f32) * 100 as f32));
        write_string_to_file(inst_str, &file_path)?;
        Ok(())
    }

    pub fn save_emulator_sigmap(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.signals", self.compiler_cfg.output_dir, self.compiler_cfg.top_module);
        for (i, mapping) in self.emul.mod_mappings.iter() {
            let mut out = file_path.to_string();
            out.push_str(&format!("-{}", i));
            write_string_to_file(
                format!("{:#?}", mapping.signal_map),
                &out)?;
        }
        Ok(())
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

        let io_i = get_nodes_type(&self.graph, Primitives::Input);
        let mut vis_map = self.graph.visit_map();
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in io_i.iter() {
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
        let io_i = get_nodes_type(&self.graph, Primitives::Input);

        // Push Input nodes
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in io_i.iter() {
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
            let proc = node.clone().get_info().coord.proc % 5;
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
