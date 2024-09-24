use crate::fsim::board::Board;
use crate::utils::write_string_to_file;
use serde::{Deserialize, Serialize};
use petgraph::{
    graph::{Graph, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use strum_macros::EnumCount as EnumCountMacro;
use strum::EnumCount;
use derivative::Derivative;
use indexmap::IndexMap;
use serde::ser::SerializeStruct;
use serde::Serializer;
use std::{
    cmp::{Ordering, min}, fmt::Debug, collections::LinkedList,
};
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Operand {
    /// index into data memory
    pub rs: u32,

    /// ldm or sdm?
    pub local: bool,

    /// for luts, which input does this operand correspond to
    pub idx: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SwitchInfo {
    pub local_set: bool,
    pub fwd_set: bool,

    /// proc to receive bit from
    pub idx: u32,

    /// Receive from local switch
    pub local: bool,

    /// forward the incomming bit
    pub fwd: bool
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Instruction {
    pub valid: bool,
    pub opcode: Primitive,
    pub lut: u64,
    pub operands: Vec<Operand>,
    pub sinfo: SwitchInfo,
}

impl Instruction {
    pub fn new(nops: u32) -> Self {
        Instruction {
            valid: false,
            opcode: Primitive::NOP,
            lut: 0,
            operands: Vec::with_capacity(nops as usize),
            sinfo: SwitchInfo::default(),
        }
    }

    pub fn to_bytes(self: &Self, cfg: &PlatformConfig) -> BitBuf {
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
        ret.push_bits(self.sinfo.idx as u64, cfg.switch_bits());
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

pub type HWGraph = Graph<HWNode, HWEdge>;

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

/// # Metadata attached to each `HWGraph` edge
#[derive(Debug, Clone, Default)]
pub struct HWEdge {
    /// Name of the output signal
    pub name: String,

    /// For inter-module communication, set to describe how the bit is routed
    /// over the global network.
    /// For communication that happens within a module, this is set to None.
    pub route: Option<NetworkRoute>,
}

impl HWEdge {
    pub fn new(name_: String) -> Self {
        HWEdge {
            name: name_,
            route: None,
        }
    }

    pub fn set_routing(self: &mut Self, route: NetworkRoute) {
        self.route = Some(route);
    }
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

    /// debug
    pub debug: bool,
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
    pub prim: ParsedPrimitive,
    pub info: NodeInfo
}

/// # Interface for accessing/manipulating the underlying node in `HWGraph`
impl HWNode {
    pub fn new(prim: ParsedPrimitive) -> Self {
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

    pub fn set_info(&mut self, i: NodeInfo) {
        self.info = i
    }

    pub fn get_lut_table(&self) -> Option<Vec<Vec<u8>>> {
        match &self.prim {
            ParsedPrimitive::Lut { inputs: _, output: _, table } => Some(table.to_vec()),
            _ => None
        }
    }

    pub fn get_lut_inputs(&self) -> Option<Vec<String>> {
        match &self.prim {
            ParsedPrimitive::Lut { inputs, .. } => Some(inputs.to_vec()),
            _ => None
        }
    }

    pub fn name(&self) -> &str {
        match &self.prim {
            ParsedPrimitive::Lut { inputs: _, output, .. } => &output,
            ParsedPrimitive::Gate { c: _, d: _, q, .. } => &q,
            ParsedPrimitive::Input { name } => &name,
            ParsedPrimitive::Output { name } => &name,
            ParsedPrimitive::Latch { input: _, output, .. } => &output,
            _ => ""
        }
    }
}

impl PartialEq for HWNode {
    fn eq(&self, other: &Self) -> bool {
        if self.is() == other.is() {
            self.info().rank.mob == other.info().rank.mob
        } else {
            false
        }
    }
}

impl Eq for HWNode {}

impl Ord for HWNode {
    fn cmp(&self, other: &Self) -> Ordering {
        if (self.is() == Primitive::Input) && (other.is() == Primitive::Input)
            || (self.is() != Primitive::Input) && (other.is() != Primitive::Input)
        {
            return self.info().rank.mob.cmp(&other.info().rank.mob);
        } else if self.is() == Primitive::Input {
            return Ordering::Less;
        } else {
            return Ordering::Greater;
        }
    }
}

impl PartialOrd for HWNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
    pub inter_mod_paths: IndexMap<(u32, u32), Vec<NetworkPath>>
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
        paths.push(NetworkPath::new(src, dst));
    }

    /// Returns a Vec<NetworkPath> where the path connects some processor in
    /// src.module to some processor in dst.module
    pub fn inter_mod_paths(self: &Self, src: Coordinate, dst: Coordinate) -> Vec<NetworkPath> {
        let paths = self.inter_mod_paths.get(&(src.module, dst.module)).unwrap();
        return paths.to_vec();
    }

    /// Returns a Vec<NetworkRoute> where the route connects src.module to dst.module
    /// while hopping to one intermediate module
    pub fn inter_mod_routes(self: &Self, src: Coordinate, dst: Coordinate) -> Vec<NetworkRoute> {
        let mut ret: Vec<NetworkRoute> = vec![];
        let mut src_to_inter: IndexMap<u32, Vec<NetworkPath>> = IndexMap::new();
        let mut inter_to_dst: IndexMap<u32, Vec<NetworkPath>> = IndexMap::new();
        for ((m1, m2), paths) in self.inter_mod_paths.iter() {
            if *m1 == src.module && *m2 != dst.module {
                if !src_to_inter.contains_key(m2) {
                    src_to_inter.insert(*m2, vec![]);
                }
                src_to_inter.get_mut(m2).unwrap().append(&mut paths.clone());
            }
            if *m1 != src.module && *m2 == dst.module {
                if !inter_to_dst.contains_key(m2) {
                    inter_to_dst.insert(*m1, vec![]);
                }
                inter_to_dst.get_mut(m1).unwrap().append(&mut paths.clone());
            }
        }
        for imod in src_to_inter.keys() {
            for s2i_path in src_to_inter.get(imod).unwrap().iter() {
                for i2d_path in inter_to_dst.get(imod).unwrap().iter() {
                    let route = if s2i_path.dst == i2d_path.src {
                        NetworkRoute::from([*s2i_path, *i2d_path])
                    } else {
                        NetworkRoute::from([*s2i_path,
                                           NetworkPath::new(
                                               s2i_path.dst,
                                               i2d_path.src),
                                           *i2d_path])
                    };
                    ret.push(route);
                }
            }
        }
        return ret;
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

    /// Latency of the switch network between processors in the same module
    pub inter_proc_nw_lat: Cycle,

    /// Latency of the switch network between modules
    pub inter_mod_nw_lat: Cycle,

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
            inter_proc_nw_lat: 0,
            inter_mod_nw_lat: 0,
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

    /// log2Ceil(number of Primitive)
    pub fn opcode_bits(self: &Self) -> u32 {
        // NOTE: Currently subtracting 2 to exclude Subckt and Module
        let num_prims: u32 = Primitive::COUNT as u32 - 2;
        self.log2ceil(num_prims)
    }

    /// number of bits for the LUT
    pub fn lut_bits(self: &Self) -> u32 {
        1 << self.lut_inputs
    }

    pub fn total_procs(self: &Self) -> u32 {
        self.num_mods * self.num_procs
    }

    /// - I can start using bits computed from a local processor at
    /// `local.pc + intra_proc_dep_lat`
    ///   <me> | read imem | read dmem | compute + write dmem |
    ///   <me>                                    | read imem | read dmem | compute
// pub fn intra_proc_dep_lat(self: &Self) -> Cycle {
// self.dmem_rd_lat + self.dmem_wr_lat
// }

    /// - I can start using bits computed from a remote processor at
    /// `remote.pc + inter_proc_dep_lat`.
    ///   <other> | read imem | read dmem | lut + network | write dmem |
    ///   <me>                                             | read imem | read dmem | compute |
// pub fn inter_proc_dep_lat(self: &Self) -> Cycle {
// self.dmem_rd_lat + self.inter_proc_nw_lat + self.dmem_wr_lat
// }

    /// - I have to receive a incoming bit from a remote processor at
    /// `remote.pc + remote_sin_lat`
    /// <other> | read imem | read dmem | compute | network |
    /// <me>                        | read imem | read dmem | compute + write sdm |
    pub fn remote_sin_lat(self: &Self) -> Cycle {
        self.inter_proc_nw_lat
    }

    /// If the current pc is `X`, store the current local compute result in
    /// `X - pc_ldm_offset`
    pub fn pc_ldm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat
    }

    /// If the current pc is `X`, store the current switch compute result in
    /// `X - pc_sdm_offset`
    pub fn pc_sdm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat + self.inter_proc_nw_lat
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_path_lat(self: &Self, path: &NetworkPath) -> u32 {
        match path.tpe {
            PathTypes::ProcessorInternal => 0,
            PathTypes::InterProcessor    => self.inter_proc_nw_lat,
            PathTypes::InterModule       => self.inter_mod_nw_lat
        }
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_route_lat(self: &Self, route: &NetworkRoute) -> u32 {
        let mut latency = 0;
        for (hop, path) in route.iter().enumerate() {
            latency += self.nw_path_lat(path);
            if hop != route.len() - 1 {
                latency += self.dmem_wr_lat
            }
        }
        return latency;
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_route_dep_lat(self: &Self, route: &NetworkRoute) -> u32 {
        return self.nw_route_lat(route) + self.dmem_wr_lat;
    }
}

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular emulator processor
#[derive(Serialize, Debug, Default, Clone)]
pub struct ProcessorMapping {
    /// Generated instructions for this module
    pub instructions: Vec<Instruction>,

    /// Signal mapping info
    pub signal_map: IndexMap<String, NodeMapInfo>,
}

/// # MappingInfo
/// - Fields specific to how the design is mapped to a particular emulator module
#[derive(Serialize, Debug, Default, Clone)]
pub struct ModuleMapping {
    /// Per processor emulation mapping information
    pub proc_mappings: IndexMap<u32, ProcessorMapping>,
}

/// # MappingInfo
/// - Contains fields specific to the emulator hardware
#[derive(Serialize, Debug, Default, Clone)]
pub struct EmulatorMapping {
    /// Number of host steps to emulate a single cycle
    pub host_steps: u32,

    /// Maximum rank of this design
    pub max_rank: u32,

    /// Per module emulation mapping information
    pub module_mappings: IndexMap<u32, ModuleMapping>
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct CompilerConfig {
    /// Name of the top module
    pub top_module: String,

    /// Path to the output directory
    pub output_dir: String,

    /// Number of consecutive PCs that is identified as a scheduling tail
    pub dbg_tail_length: u32,

    /// Number of nodes scheduled per PC for that PC to be classified as a tail
    pub dbg_tail_threshold: u32
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
        let file_path = format!("{}/{}.info",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);
        for (i, mapping) in self.emul.module_mappings.iter() {
            let mut out = file_path.clone();
            out.push_str(&format!("-{}", i));
            write_string_to_file(serde_json::to_string_pretty(&mapping)?, &out)?;
        }
        Ok(())
    }

    pub fn save_emulator_instructions(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.insts",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);
        let mut inst_str = "".to_string();
        let total_insns = self.emul.host_steps * self.platform_cfg.total_procs();
        let mut total_nops = 0;
        for (i, mapping) in self.emul.module_mappings.iter() {
            inst_str.push_str(&format!("============ module {} ============\n", i));

            for (pi, pmap) in mapping.proc_mappings.iter() {
                inst_str.push_str(&format!("------------ processor {} ------------\n", pi));
                for (i, inst) in pmap.instructions.iter().enumerate() {
                    if (i as u32) < self.emul.host_steps {
                        inst_str.push_str(&format!("{} {:?}\n", i, inst));
                        match inst.opcode {
                            Primitive::NOP => total_nops += 1,
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
        let file_path = format!("{}/{}.signals",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);

        let mut ret_str = "".to_string();
        for (_mi, mmap) in self.emul.module_mappings.iter() {
            for (_pi, pmap) in mmap.proc_mappings.iter() {
                ret_str.push_str(&format!("{:#?}", pmap.signal_map));
            }
        }
        write_string_to_file(ret_str, &file_path)?;
        Ok(())
    }

    /// #debug_graph
    /// Given a `dbg_node` in the graph, search for all parents nodes up until
    /// it reaches Gate, Latch or Input.
    /// It will also print the bit-value associated with the node
    /// computed by the emulation processor.
    pub fn debug_graph(&self, dbg_node: NodeIndex, board: &Board) -> String {
        let indent: &str = "    ";
        let mut vis_map = self.graph.visit_map();
        let mut q = vec![];
        q.push(dbg_node);
        let mut root = true;

        while !q.is_empty() {
            let nidx = q.remove(0);
            vis_map.visit(nidx);

            let node = self.graph.node_weight(nidx).unwrap();
            if node.is() == Primitive::Gate || node.is() == Primitive::Latch {
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
                let val = board.peek(node.name()).unwrap();
                if node.is() == Primitive::Lut {
                    outstring.push_str(&format!(
                        "{}{} [ label = {:?} ]\n",
                        indent,
                        nidx.index(),
                        format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nlut: {:?} val: {}",
                                node.name(),
                                node.is(),
                                node.info().coord.module,
                                node.info().coord.proc,
                                node.info().rank.asap,
                                node.info().rank.alap,
                                node.info().pc,
                                node.get_lut_table().unwrap(),
                                val)));
                } else {
                    outstring.push_str(&format!(
                        "{}{} [ label = {:?} ]\n",
                        indent,
                        nidx.index(),
                        format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nval: {}",
                                node.name(),
                                node.is(),
                                node.info().coord.module,
                                node.info().coord.proc,
                                node.info().rank.asap,
                                node.info().rank.alap,
                                node.info().pc,
                                val)));
                }
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

    pub fn get_nodes_type(self: &Self, nodetype: Primitive) -> Vec<NodeIndex> {
        let mut nodes: Vec<NodeIndex> = vec![];
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight(nidx).unwrap();
            if node.is() == nodetype {
                nodes.push(nidx);
            }
        }
        return nodes;
    }
}

impl Debug for Circuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent: &str = "    ";
        let graph = &self.graph;
        let io_i = self.get_nodes_type(Primitive::Input);

        // Push Input nodes
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in io_i.iter() {
            q.push(*nidx);
        }

        write!(f, "digraph {{\n")?;

        // Print nodes in BFS order
        let mut vis_map = graph.visit_map();
        while !q.is_empty() {
            let nidx = q.remove(0);
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);
            let node = graph.node_weight(nidx).unwrap();
            // red, blue, green, white, purple
            let proc = node.clone().info().coord.proc % 5;
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
                "{}{} [ label = {:?} color = \"{}\"]\n",
                indent,
                nidx.index(),
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\n",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap),
                color
            )?;

            let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&graph) {
                if !vis_map.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }

        // Print the unvisited nodes
        for nidx in graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                continue;
            }
            let node = graph.node_weight(nidx).unwrap();

            // red, blue, green, white, purple
            let proc = node.clone().info().coord.proc % 5;
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
                "{}{} [ label = {:?} color = \"{}\"]\n",
                indent,
                nidx.index(),
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\n",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap),
                color
            )?;
        }

        for (_, edge) in graph.edge_references().enumerate() {
            write!(f, "{}{} {} {} ",
                indent, edge.source().index(), "->", edge.target().index())?;
            writeln!(f, "[ label=\"{}-{:?}\" ]", edge.weight().name, edge.weight().route)?;
        }

        write!(f, "}}")
    }
}
