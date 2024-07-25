use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Incoming,
};

pub fn dead_code_elimination(circuit: Circuit) -> Circuit {
    let mut graph = circuit.graph;
    let mut io_i = circuit.io_i;
    let mut io_o = circuit.io_o;
    let mut q: Vec<NodeIndex> = vec![];

    // Push Output nodes to the queue
    for nidx in io_o.keys() {
        q.push(*nidx);
    }

    // BFS starting from the Output node
    let mut vis_map = graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        if vis_map.is_visited(&nidx) {
            continue;
        }
        vis_map.visit(nidx);

        let parents = graph.neighbors_directed(nidx, Incoming);
        for pidx in parents {
            if !vis_map.is_visited(&pidx) {
                q.push(pidx);
            }
        }
    }

    // Find nodes to delete (can't delete here due to immutable borrow)
    for nidx in graph.node_indices().rev() {
        if !vis_map.is_visited(&nidx) {
            let nnodes = graph.node_count();
            let last_nidx = NodeIndex::new(nnodes - 1);

            // TODO : find a case where this actually happens and test it?
            if io_i.contains_key(&nidx) {
                io_i.remove(&nidx);
            } else if let Some(v) = io_i.remove(&last_nidx) {
                io_i.insert(nidx, v);
            }

            if io_o.contains_key(&nidx) {
                io_o.remove(&nidx);
            } else if let Some(v) = io_o.remove(&last_nidx) {
                io_o.insert(nidx, v);
            }

            graph.remove_node(nidx);
        }
    }

    return Circuit {
        io_i: io_i,
        io_o: io_o,
        graph: graph,
        ..circuit
    };
}
