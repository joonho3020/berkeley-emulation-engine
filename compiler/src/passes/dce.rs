use crate::common::*;
use blif_parser::primitives::Primitive;
use petgraph::{
    prelude::Bfs,
    visit::{VisitMap, Visitable}
};

pub fn dead_code_elimination(circuit: &mut Circuit) {
    // Get input and output nodes
    let io_i = circuit.get_nodes_type(Primitive::Input);
    let io_o = circuit.get_nodes_type(Primitive::Output);

    // BFS from inputs
    let mut i_vismap = circuit.graph.visit_map();
    for nidx in io_i.iter() {
        let mut bfs = Bfs::new(&circuit.graph, *nidx);
        while let Some(nx) = bfs.next(&circuit.graph) {
            i_vismap.visit(nx);
        }
    }

    // BFS from outputs
    circuit.graph.reverse();
    let mut o_vismap = circuit.graph.visit_map();
    for nidx in io_o.iter() {
        let mut bfs = Bfs::new(&circuit.graph, *nidx);
        while let Some(nx) = bfs.next(&circuit.graph) {
            o_vismap.visit(nx);
        }
    }
    circuit.graph.reverse();

    // Find nodes to delete (can't delete here due to immutable borrow)
    for nidx in circuit.graph.node_indices().rev() {
        if !o_vismap.is_visited(&nidx) || !i_vismap.is_visited(&nidx) {
            circuit.graph.remove_node(nidx);
        }
    }
}
