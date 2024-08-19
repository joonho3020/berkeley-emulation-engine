use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::{
    graph::{Graph, NodeIndex},
    visit::{NodeCount, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use std::cmp::max;

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    node.set_info(NodeInfo { proc: proc, ..info });
}

pub fn map_to_processor(circuit: &mut Circuit) {
    greedy_partition(circuit);

// partition_on_register_boundary(circuit);
// merge_partitions(circuit)
}

/// Map the nodes to the processors as sparsely as possible. Whenever there
/// is a Gate or a Latch, add it into a new processor.
fn partition_on_register_boundary(circuit: &mut Circuit) {
    // Start from Input
    let mut q: Vec<NodeIndex> = vec![];
    for nidx in circuit.io_i.keys() {
        q.push(*nidx);
    }

    let mut proc_id = 0;
    let mut vis_map = circuit.graph.visit_map();
    while !q.is_empty() {
        let root = q.remove(0);
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
                proc_id
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
    circuit.emulator.used_procs = proc_id;
}


fn compute_parent_cost(
    circuit: &mut Circuit,
    node_cost: &mut IndexMap<NodeIndex, u32>,
    nidx: &NodeIndex) -> u32 {
    let cfg = &circuit.emulator.cfg;
    let mut parent_cost = 0;
    let mut parents = circuit.graph.neighbors_directed(*nidx, Incoming).detach();
    while let Some(pidx) = parents.next_node(&circuit.graph) {
        let pnode = circuit.graph.node_weight(pidx).unwrap();
        parent_cost = max(parent_cost, *node_cost.get_mut(&pidx).unwrap() + cfg.network_cost());
    }
    return parent_cost;
}

fn compute_cost(
    circuit: &mut Circuit,
    global_cost: &u32,
    proc_cost: &IndexMap<u32, u32>,
    proc_id: &u32,
    parent_cost: &u32) -> (u32, u32) {

    let cfg = &circuit.emulator.cfg;
    let cost =  proc_cost.get(proc_id).unwrap() + cfg.compute_cost() + max(global_cost, parent_cost);
    let delta = if cost > *global_cost {
        cost - *global_cost
    } else {
        0
    };
    return (cost, delta);
}


pub fn greedy_partition(circuit: &mut Circuit) {
    let mut global_cost: u32 = 0;
    let mut proc_cost: IndexMap<u32, u32> = IndexMap::new();
    let mut node_cost: IndexMap<NodeIndex, u32> = IndexMap::new();
    for i in 0..circuit.emulator.cfg.module_sz {
        proc_cost.insert(i, 0);
    }

    let mut max_proc_id = 0;
    let topo_sort_nodes = circuit.topo_sorted_nodes();
    for nidx in topo_sort_nodes.iter() {
        let mut min_proc_id = 0;
        let mut min_delta = u32::MAX;
        let mut min_cost  = u32::MAX;
        for i in 0..circuit.emulator.cfg.module_sz {
            let node = circuit.graph.node_weight(*nidx).unwrap();

            let parent_cost = if node.is() == Primitives::Gate || node.is() == Primitives::Latch {
                0
            } else {
                compute_parent_cost(circuit, &mut node_cost, &nidx)
            };
            let (cost, delta) = compute_cost(circuit, &global_cost, &proc_cost, &i, &parent_cost);
            if delta < min_delta {
                min_delta = delta;
                min_cost  = cost;
                min_proc_id = i;
            }
        }

        // update proc_cost
        // update global cost
        // update procid for nidx

        let prev_proc_cost = proc_cost.get(&min_proc_id).unwrap();
        proc_cost.insert(min_proc_id, prev_proc_cost + min_delta);
        node_cost.insert(*nidx, min_cost);
        global_cost = global_cost + min_delta;
        set_proc(
            &mut circuit.graph,
            *nidx,
            min_proc_id);
        max_proc_id = max(max_proc_id, min_proc_id);
    }
    circuit.emulator.used_procs = max_proc_id + 1;
}

struct ProcInfo {
    pub proc: u32,
    pub cnt:  u32
}

fn merge_partitions(circuit: &mut Circuit) {
    // Everything just fits nicely
    if circuit.emulator.used_procs <= circuit.emulator.cfg.module_sz {
        return;
    }

    // Spread out the graph onto the processors as evenly as possible
    let total_nodes = circuit.graph.node_count() as u32;
    let avg_nodes_per_proc: u32 = total_nodes.div_ceil(circuit.emulator.cfg.module_sz);

    // Count the size of each partition
    let mut proc_sizes: IndexMap<u32, u32> = IndexMap::new();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let proc = node.get_info().proc;
        match proc_sizes.get(&proc) {
            Some(cnt) => { proc_sizes.insert(proc, cnt + 1); }
            None =>      { proc_sizes.insert(proc, 1); }
        }
    }

    // Nodes of proc_graph represents the size of each partition
    let mut proc_graph: Graph<u32, u32> = Graph::default();
    let mut proc_nodes: IndexMap<u32, NodeIndex> = IndexMap::new();
    for i in 0..circuit.emulator.used_procs {
        let nidx = proc_graph.add_node(i);
        proc_nodes.insert(i, nidx);
    }

    // BFS and add dependencies between procs as edges in proc_graph
    let mut q: Vec<NodeIndex> = vec![];
    let mut vis_map = circuit.graph.visit_map();
    for nidx in circuit.io_i.keys() {
        q.push(*nidx);
    }
    while !q.is_empty() {
        let nidx = q.remove(0);
        vis_map.visit(nidx);

        let mut childs = circuit.graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&circuit.graph) {
            // for children nodes of this node, add the partition index
            // into the dependency graph
            let child_proc_idx = circuit.graph.node_weight(cidx).unwrap().get_info().proc;
            let cur_proc_idx = circuit.graph.node_weight(nidx).unwrap().get_info().proc;

            let cur_nodx_idx   = proc_nodes.get(&cur_proc_idx).unwrap();
            let child_nodx_idx = proc_nodes.get(&child_proc_idx).unwrap();
            let find_edge = proc_graph.find_edge(*cur_nodx_idx, *child_nodx_idx);
            if (child_proc_idx != cur_proc_idx) && find_edge.is_none() {
                proc_graph.add_edge(*cur_nodx_idx, *child_nodx_idx, 1);
            }

            if !vis_map.is_visited(&cidx) {
                q.push(cidx);
            }
        }
    }

    // Partitioning algorithm
    // - Reassign used_procs
    // - Reassign proc for each node after merging


    // Greedy? Partitioning?
    //
    // merged = {}
    // nodes = pq (proc_sz, procid)
    //
}
