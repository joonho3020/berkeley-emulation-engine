use std::cmp::max;

use crate::primitives::*;
use petgraph::{
    data::{DataMap, DataMapMut},
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};

pub fn kaminpar_partition(circuit: Circuit) -> (Vec<u32>, Circuit) {
    let undirected_graph = circuit.graph.clone().into_edge_type();
    let partition = kaminpar::PartitionerBuilder::with_epsilon(circuit.ctx.kaminpar_epsilon)
        .seed(circuit.ctx.kaminpar_seed)
        .threads(std::num::NonZeroUsize::new(circuit.ctx.kaminpar_nthreads).unwrap())
        .partition(&undirected_graph, circuit.ctx.num_partitions);

    let ret = match partition {
        Ok(assignments) => assignments,
        Err(e) => {
            println!("Kaminpar partition error {}", e);
            vec![]
        }
    };
    return (ret, circuit);
}

pub fn set_rank(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    let new_rank = max(info.rank, rank);
    node.set_info(NodeInfo {
        rank: new_rank,
        ..info
    })
}

pub fn find_rank_order(circuit: Circuit) -> Circuit {
    let mut graph = circuit.graph;
    let io_i = circuit.io_i;

    // Search for flip-flop nodes
    let mut ff_nodes: Vec<NodeIndex> = vec![];
    for nidx in graph.node_indices() {
        let node = graph.node_weight(nidx).unwrap();
        match node.is() {
            Primitives::Latch | Primitives::Gate => {
                ff_nodes.push(nidx);
            }
            _ => {}
        }
    }

    // Push Input & FF nodes
    let mut q: Vec<NodeIndex> = vec![];
    for nidx in io_i.keys() {
        q.push(*nidx);
        set_rank(&mut graph, *nidx, 0);
    }
    for nidx in ff_nodes.iter() {
        q.push(*nidx);
        set_rank(&mut graph, *nidx, 0);
    }

    // BFS
    let mut vis_map = graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        if vis_map.is_visited(&nidx) {
            continue;
        }

        vis_map.visit(nidx);
        let parent_rank = graph.node_weight_mut(nidx).unwrap().get_info().rank;

        let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&graph) {
            set_rank(&mut graph, cidx, parent_rank + 1);
            if !vis_map.is_visited(&cidx) {
                q.push(cidx);
            }
        }
    }

    return Circuit {
        io_i: io_i,
        graph: graph,
        ..circuit
    };
}
