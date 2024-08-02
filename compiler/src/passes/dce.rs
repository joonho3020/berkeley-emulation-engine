use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Incoming,
};

pub fn dead_code_elimination(circuit: &mut Circuit) {
    let mut q: Vec<NodeIndex> = vec![];

    // Push Output nodes to the queue
    for nidx in circuit.io_o.keys() {
        q.push(*nidx);
    }

    // BFS starting from the Output node
    let mut vis_map = circuit.graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        if vis_map.is_visited(&nidx) {
            continue;
        }
        vis_map.visit(nidx);

        let parents = circuit.graph.neighbors_directed(nidx, Incoming);
        for pidx in parents {
            if !vis_map.is_visited(&pidx) {
                q.push(pidx);
            }
        }
    }

    // Find nodes to delete (can't delete here due to immutable borrow)
    for nidx in circuit.graph.node_indices().rev() {
        if !vis_map.is_visited(&nidx) {
            let nnodes = circuit.graph.node_count();
            let last_nidx = NodeIndex::new(nnodes - 1);

            // TODO : find a case where this actually happens and test it?
            if circuit.io_i.contains_key(&nidx) {
                circuit.io_i.swap_remove(&nidx);
            } else if let Some(v) = circuit.io_i.swap_remove(&last_nidx) {
                circuit.io_i.insert(nidx, v);
            }

            if circuit.io_o.contains_key(&nidx) {
                circuit.io_o.swap_remove(&nidx);
            } else if let Some(v) = circuit.io_o.swap_remove(&last_nidx) {
                circuit.io_o.insert(nidx, v);
            }

            circuit.graph.remove_node(nidx);
        }
    }
}
