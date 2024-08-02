use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
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

    // Push Input & FF nodes
    let mut q: Vec<NodeIndex> = vec![];
    for nidx in circuit.io_i.keys() {
        q.push(*nidx);
        set_rank(&mut circuit.graph, *nidx, 0);
    }
    for nidx in ff_nodes.iter() {
        q.push(*nidx);
        set_rank(&mut circuit.graph, *nidx, 0);
    }

    // compute indeg
    let mut indeg: IndexMap<NodeIndex, u32> = IndexMap::new();
    for nidx in circuit.graph.node_indices() {
        indeg.insert(nidx, 0);
    }
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let dst = e.1;
        *indeg.get_mut(&dst).unwrap() += 1;
    }

    // BFS
    let mut topo_sort_order: Vec<NodeIndex> = vec![];
    let mut vis_map = circuit.graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        vis_map.visit(nidx);
        topo_sort_order.push(nidx);

        let mut childs = circuit.graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&circuit.graph) {
            *indeg.get_mut(&cidx).unwrap() -= 1;
            if *indeg.get(&cidx).unwrap() == 0 && !vis_map.is_visited(&cidx) {
                q.push(cidx);
            }
        }
    }

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

pub fn map_to_processor(circuit: &mut Circuit) {
    // Start from Input
    let mut q: Vec<NodeIndex> = vec![];
    for nidx in circuit.io_i.keys() {
        q.push(*nidx);
    }

    let mut proc_id = 0;
    let max_gates = circuit.emulator.cfg.gates_per_partition;

    let mut vis_map = circuit.graph.visit_map();
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
            set_proc(
                &mut circuit.graph,
                nidx,
                proc_id,
                &mut cur_proc_size,
                max_gates,
            );

            let mut childs = circuit.graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&circuit.graph) {
                let child_type = circuit.graph.node_weight_mut(cidx).unwrap().is();
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
}
