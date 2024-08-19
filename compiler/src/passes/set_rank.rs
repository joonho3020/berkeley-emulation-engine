use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    Direction::Incoming,
};
use std::cmp::max;

fn set_rank(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    let new_rank = max(info.rank, rank);
    node.set_info(NodeInfo {
        rank: new_rank,
        ..info
    })
}

pub fn find_rank_order(circuit: &mut Circuit) {
    // Search for flip-flop nodes
    let mut ff_nodes: Vec<NodeIndex> = vec![];
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        match node.is() {
            Primitives::Latch | Primitives::Gate => {
                ff_nodes.push(nidx);
            }
            _ => {}
        }
    }

    // Set rank for input & FF nodes
    for nidx in circuit.io_i.keys() {
        set_rank(&mut circuit.graph, *nidx, 0);
    }
    for nidx in ff_nodes.iter() {
        set_rank(&mut circuit.graph, *nidx, 0);
    }

    // Set rank based on the topo sorted order
    let topo_sort_order = circuit.topo_sorted_nodes();
    for nidx in topo_sort_order.iter() {
        let node = circuit.graph.node_weight(*nidx).unwrap();
        if node.is() != Primitives::Gate || node.is() != Primitives::Latch {
            let mut max_parent_rank = 0;
            let mut parents = circuit.graph.neighbors_directed(*nidx, Incoming).detach();
            while let Some(pidx) = parents.next_node(&circuit.graph) {
                let parent = circuit.graph.node_weight(pidx).unwrap();
                max_parent_rank = max(max_parent_rank, parent.get_info().rank);
            }
            set_rank(&mut circuit.graph, *nidx, max_parent_rank + 1);
        }
    }
}
