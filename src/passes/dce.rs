use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Incoming,
};

pub fn dead_code_elimination(circuit: Circuit) -> Circuit {
    let mut graph = circuit.graph;
    let mut q: Vec<NodeIndex> = vec![];

    // Push Output nodes to the queue
    for nidx in circuit.io_o.keys() {
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
    let mut remove_nodes: Vec<NodeIndex> = vec![];
    for nidx in graph.node_indices() {
        if !vis_map.is_visited(&nidx) {
            remove_nodes.push(nidx);
        }
    }

    // Perform deletetion
    for nidx in remove_nodes.iter() {
        graph.remove_node(*nidx);
    }

    return Circuit {
        graph: graph,
        ..circuit
    };
}
