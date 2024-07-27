use crate::instruction::Instruction;
use indexmap::IndexMap;
use petgraph::{
    dot::{Config, Dot},
    graph::{Graph, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::Outgoing,
};
use std::{
    cmp::{max, Ordering},
    fmt::Debug,
    fs::File,
    io::Write,
};

pub type HWGraph = Graph<Box<dyn HWNode>, String>;

#[derive(PartialEq, Debug, Clone, Default)]
pub enum Primitives {
    #[default]
    NOP,
    Input,
    Output,
    Lut,
    Subckt,
    Gate,
    Latch,
    Module,
}

pub trait HWNode: Debug {
    fn is(&self) -> Primitives;
    fn box_clone(&self) -> Box<dyn HWNode>;
    fn set_info(&mut self, info: NodeInfo);
    fn get_info(&self) -> NodeInfo;
    fn get_lut(&self) -> Option<Lut>;
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

#[derive(Debug, Clone, Default)]
pub struct NodeInfo {
    pub proc: u32,
    pub rank: u32,
    pub scheduled: bool,
    pub pc: u32,
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
}

impl Debug for Lut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lut {:?}", self.info)
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
}

impl Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gate {:?}", self.info)
    }
}

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum LatchInit {
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
}

impl Debug for Latch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Latch {:?}", self.info)
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

#[derive(Debug, Default, Clone)]
pub struct Context {
    pub gates_per_partition: u32,
    pub network_latency: u32,
}

#[derive(Default, Clone)]
pub struct Circuit {
    pub mods: Vec<Module>,
    pub graph: HWGraph,
    pub io_i: IndexMap<NodeIndex, String>,
    pub io_o: IndexMap<NodeIndex, String>,
    pub ctx: Context,
    pub instructions: Vec<Vec<Instruction>>,
}

impl Circuit {
    pub fn set_ctx(&mut self, ctx: Context) {
        self.ctx = ctx;
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

    pub fn save_insts(&self, file_pfx: String) -> std::io::Result<()> {
        let mut file = File::create(format!("{}.instructions", file_pfx))?;
        for (proc, insts) in self.instructions.iter().enumerate() {
            write!(&mut file, "-----------------------------\n")?;
            for (pc, inst) in insts.iter().enumerate() {
                write!(&mut file, "{} {}: {:?}\n", proc, pc, inst)?;
            }
        }
        Ok(())
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
