use crate::common::{
    circuit::Circuit,
    primitive::*
};
use petgraph::{
    prelude::Bfs,
    visit::{VisitMap, Visitable}, Direction::Outgoing
};

pub fn dead_code_elimination(circuit: &mut Circuit) {
    // Get inputs and outputs
    let io_o     = circuit.get_nodes_type(Primitive::Output);
    let mut io_i = circuit.get_nodes_type(Primitive::Input);
    let consts   = circuit.get_nodes_type(Primitive::ConstLut);

    io_i.extend(consts);

    // BFS from inputs and constants
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

    for nidx in circuit.graph.node_indices().rev() {
        match circuit.graph.node_weight(nidx).unwrap().is() {
            Primitive::ConstLut => {
                if circuit.graph.neighbors_directed(nidx, Outgoing).count() == 0 {
                    circuit.graph.remove_node(nidx);
                }
            }
            _ => {
                if !o_vismap.is_visited(&nidx) || !i_vismap.is_visited(&nidx) {
                    circuit.graph.remove_node(nidx);
                }
            }
        }
    }
}
