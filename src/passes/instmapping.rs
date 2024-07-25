use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};

pub fn instruction_mapping(circuit: Circuit) -> Circuit {
    circuit
}
