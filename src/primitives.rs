use petgraph::graph::Graph;
use std::collections::HashMap;
use std::fmt::Debug;

pub enum Primitives {
    Input,
    Output,
    Lut,
    Subckt,
    Gate,
    Latch,
    Module
}

pub trait HWNode: Debug {
    fn is(&self) -> Primitives;
}

pub type HWGraph = Graph<Box<dyn HWNode>, String>;

#[derive(Debug)]
pub struct Input {
    pub name: String
}

impl HWNode for Input {
    fn is(&self) -> Primitives {
        return Primitives::Input
    }
}

#[derive(Debug)]
pub struct Output {
    pub name: String
}

impl HWNode for Output {
    fn is(&self) -> Primitives {
        return Primitives::Output
    }
}

#[derive(Debug, Clone)]
pub struct Lut {
    pub inputs: Vec<String>,
    pub output: String,
    pub table: Vec<Vec<u8>>,
}

impl HWNode for Lut {
    fn is(&self) -> Primitives {
        return Primitives::Lut
    }
}

#[derive(Debug)]
pub struct Subckt {
    pub name: String,
    pub conns: HashMap<String, String>,
}

impl HWNode for Subckt {
    fn is(&self) -> Primitives {
        return Primitives::Subckt
    }
}

#[derive(Debug, Clone)]
pub struct Gate {
    pub c: String,
    pub d: String,
    pub q: String,
    pub r: Option<String>,
    pub e: Option<String>,
}

impl Default for Gate {
    fn default() -> Gate {
        Gate {
            c: "".to_string(),
            d: "".to_string(),
            q: "".to_string(),
            r: None,
            e: None,
        }
    }
}

impl HWNode for Gate {
    fn is(&self) -> Primitives {
        return Primitives::Gate
    }
}

#[repr(u8)]
#[derive(Debug)]
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

#[derive(Debug)]
pub struct Latch {
    pub input: String,
    pub output: String,
    pub control: String,
    pub init: LatchInit,
}

impl Default for Latch {
    fn default() -> Latch {
        Latch {
            input: "".to_string(),
            output: "".to_string(),
            control: "".to_string(),
            init: LatchInit::UNKNOWN,
        }
    }
}

impl HWNode for Latch {
    fn is(&self) -> Primitives {
        return Primitives::Latch
    }
}

pub struct Module {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub luts: Vec<Lut>,
    pub subckts: Vec<Subckt>,
    pub gates: Vec<Gate>,
    pub latches: Vec<Latch>,
}

impl HWNode for Module {
    fn is(&self) -> Primitives {
        return Primitives::Module
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

#[derive(Debug)]
pub struct Circuit {
    pub mods: Vec<Module>,
    pub graph: HWGraph,
}
