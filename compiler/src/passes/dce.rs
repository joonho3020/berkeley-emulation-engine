use crate::primitives::*;
use petgraph::{
    prelude::Bfs,
    visit::{VisitMap, Visitable}
};

pub fn dead_code_elimination(circuit: &mut Circuit) {
    // BFS from inputs
    let mut i_vismap = circuit.graph.visit_map();
    for nidx in circuit.io_i.keys() {
        let mut bfs = Bfs::new(&circuit.graph, *nidx);
        while let Some(nx) = bfs.next(&circuit.graph) {
            i_vismap.visit(nx);
        }
    }

    // BFS from outputs
    circuit.graph.reverse();
    let mut o_vismap = circuit.graph.visit_map();
    for nidx in circuit.io_o.keys() {
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

    // Reset the IO mappings
    circuit.io_i.clear();
    circuit.io_o.clear();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        match node.is() {
            Primitives::Input => {
                circuit.io_i.insert(nidx, node.name().to_string());
            }
            Primitives::Output => {
                circuit.io_o.insert(nidx, node.name().to_string());
            }
            _ => { }
        }
    }
}
