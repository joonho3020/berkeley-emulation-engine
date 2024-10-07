use crate::common::{
    circuit::{self, Circuit},
    primitive::*
};
use petgraph::{
    prelude::Bfs,
    visit::{VisitMap, Visitable}
};

pub fn replicate_consts(circuit: &mut Circuit) {
    // if the const node does not have any children, remove it
}
