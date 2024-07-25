use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Outgoing,
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
            let node_type = graph.node_weight_mut(cidx).unwrap().is();
            if (node_type != Primitives::Gate) && (node_type != Primitives::Latch) {
                set_rank(&mut graph, cidx, parent_rank + 1);
            }
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

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32,
    cur_proc_size: &mut u32,
    max_gates: u32,
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    node.set_info(NodeInfo { proc: proc, ..info });

    *cur_proc_size = *cur_proc_size + 1;
    assert!(
        *cur_proc_size <= max_gates,
        "Number of gates ({}) exceeded max_gates ({})",
        cur_proc_size,
        max_gates
    );
}

pub fn map_to_processor(circuit: Circuit) -> Circuit {
    let mut graph = circuit.graph;
    let io_i = circuit.io_i;

    // Start from Input
    let mut q: Vec<NodeIndex> = vec![];
    for nidx in io_i.keys() {
        q.push(*nidx);
    }

    let mut proc_id = 0;
    let max_gates = circuit.ctx.gates_per_partition;

    let mut vis_map = graph.visit_map();
    while !q.is_empty() {
        let root = q.remove(0);

        let mut cur_proc_size = 0;
        let mut qq: Vec<NodeIndex> = vec![];
        qq.push(root);
        while !qq.is_empty() {
            let nidx = qq.remove(0);
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);
            set_proc(&mut graph, nidx, proc_id, &mut cur_proc_size, max_gates);

            let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&graph) {
                let child_type = graph.node_weight_mut(cidx).unwrap().is();
                if !vis_map.is_visited(&cidx) {
                    if (child_type != Primitives::Gate) && (child_type != Primitives::Latch) {
                        qq.push(cidx);
                    } else {
                        q.push(cidx);
                    }
                }
            }
        }
        proc_id += 1;
    }

    return Circuit {
        io_i: io_i,
        graph: graph,
        ..circuit
    };
}
