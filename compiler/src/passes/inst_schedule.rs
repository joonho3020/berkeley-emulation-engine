use crate::primitives::*;
use crate::utils::write_string_to_file;
use indexmap::{IndexMap, IndexSet};
use petgraph::{
    graph::{Graph, NodeIndex},
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use std::cmp::max;

#[derive(Eq, Hash, PartialEq, Clone)]
struct InstOrProc {
    nidx: Option<NodeIndex>,
    pidx: Option<u32>,
}

/// # Helper struct for instruction scheduling
#[derive(Debug, Default)]
struct NodeArray {
    nodes: Vec<NodeIndex>,
    ptr: usize,
}

impl NodeArray {
    fn push_node(&mut self, nidx: NodeIndex) {
        self.nodes.push(nidx);
    }

    fn current(&self) -> NodeIndex {
        // println!("current {} {}", self.nodes.len(), self.ptr);
        return self.nodes[self.ptr];
    }

    fn done(&self) -> bool {
        return self.nodes.len() == self.ptr;
    }

    fn schedule(&mut self) {
        self.ptr += 1;
    }

    fn max_rank_node(&self) -> NodeIndex {
        return self.nodes[self.nodes.len() - 1];
    }
}

/// # Finds a valid instruction schedule given a partitioned graph
/// - Schedule Input & Gates first
/// - For each partition get next rank to simulate
///    - if dependencies are resolved, schedule it
///    - else insert nop
/// - For each partition, if there is two ore more instructions that are parents scheduled in
///   step 2, unschedule some of them.
pub fn schedule_instructions(circuit: &mut Circuit) {
    let mut max_proc = 0;
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        max_proc = max(max_proc, node.clone().get_info().proc);
    }

    // subgraphs is ordered in BFS order starting from the input nodes
    let mut subgraphs_rank_order: Vec<NodeArray> = vec![];
    for _ in 0..(max_proc + 1) {
        subgraphs_rank_order.push(NodeArray::default());
    }

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        subgraphs_rank_order
            .get_mut(node.get_info().proc as usize)
            .unwrap()
            .push_node(nidx);
    }
    for sro in subgraphs_rank_order.iter_mut() {
        sro.nodes.sort_by(|idx1, idx2| {
            let n1 = circuit.graph.node_weight(*idx1).unwrap();
            let n2 = circuit.graph.node_weight(*idx2).unwrap();
            n1.cmp(&n2)
        });
    }

    let mut pc = 0;
    let mut scheduled_map = circuit.graph.visit_map();

    while scheduled_map.count_ones(..) != scheduled_map.len() {
        let mut schedule_candidates: IndexSet<NodeIndex> = IndexSet::new();

        for (_, node_array) in subgraphs_rank_order.iter_mut().enumerate() {
            if node_array.done() {
                continue;
            }
            let nidx = node_array.current();
            let node = circuit.graph.node_weight_mut(nidx).unwrap();

            if node.is() == Primitives::Input
                || node.is() == Primitives::Gate
                || node.is() == Primitives::Latch
            {
                schedule_candidates.insert(nidx);
            } else {
                let mut parents = circuit.graph.neighbors_directed(nidx, Incoming).detach();
                let mut unresolved_dep = false;
                while let Some(pidx) = parents.next_node(&circuit.graph) {
                    let pnode = circuit.graph.node_weight(pidx).unwrap();
                    if !pnode.get_info().scheduled {
                        unresolved_dep = true;
                        break;
                    }
                }
                if !unresolved_dep {
                    schedule_candidates.insert(nidx);
                }
            }
        }

        let mut dep_graph: Graph<InstOrProc, usize> = Graph::default();
        let mut proc_nodes: IndexMap<InstOrProc, NodeIndex> = IndexMap::new();
        let mut inst_criticality_map: IndexMap<NodeIndex, u32> = IndexMap::new();

        // Construct a bipartite graph where the edges look like:
        // Schedule Candidate Node Index -> Proc index of children
        for nidx in schedule_candidates.iter() {
            // add candidate instruction to dependency graph
            let inst_node = InstOrProc {
                nidx: Some(*nidx),
                pidx: None,
            };
            let inst_node_idx = dep_graph.add_node(inst_node);
            let mut criticality = 0;

            let mut childs = circuit.graph.neighbors_directed(*nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&circuit.graph) {
                // for children nodes of this node, add the partition index
                // into the dependency graph
                let child_proc_idx = circuit.graph.node_weight(cidx).unwrap().get_info().proc;
                let cur_proc_idx = circuit.graph.node_weight(*nidx).unwrap().get_info().proc;
                if child_proc_idx != cur_proc_idx {
                    let proc_node = InstOrProc {
                        nidx: None,
                        pidx: Some(child_proc_idx),
                    };
                    if !proc_nodes.contains_key(&proc_node) {
                        let proc_node_idx = dep_graph.add_node(proc_node.clone());
                        proc_nodes.insert(proc_node.clone(), proc_node_idx);
                    }

                    dep_graph.add_edge(inst_node_idx, *proc_nodes.get(&proc_node).unwrap(), 0);

                    // compute criticality
                    let max_rank_node = subgraphs_rank_order
                        .get(child_proc_idx as usize)
                        .unwrap()
                        .max_rank_node();
                    let max_rank = circuit
                        .graph
                        .node_weight(max_rank_node)
                        .unwrap()
                        .get_info()
                        .rank;
                    criticality = max(criticality, max_rank);
                }
            }
            inst_criticality_map.insert(inst_node_idx, criticality);
        }

        // Select instructions to schedule greedily based on the criticality
        let mut criticality_vec: Vec<(&NodeIndex, &u32)> = inst_criticality_map.iter().collect();
        criticality_vec.sort_by(|a, b| b.1.cmp(a.1));

        let mut dep_graph_vis = dep_graph.visit_map();

        for (nidx, _) in criticality_vec.iter() {
            let inst_node = dep_graph.node_weight(**nidx).unwrap();
            let mut child_procs = dep_graph.neighbors_directed(**nidx, Outgoing).detach();
            let mut scheduleable = true;

            // check if scheduleable
            while let Some(child_proc_idx) = child_procs.next_node(&dep_graph) {
                if dep_graph_vis.is_visited(&child_proc_idx) {
                    scheduleable = false;
                    break;
                }
            }

            if scheduleable {
                // mark the children procs as visisted in the dependency graph
                let mut child_procs_to_remove =
                    dep_graph.neighbors_directed(**nidx, Outgoing).detach();
                while let Some(child_proc_idx) = child_procs_to_remove.next_node(&dep_graph) {
                    dep_graph_vis.visit(child_proc_idx);
                }

                let original_node_idx = inst_node.nidx.unwrap();
                let original_node = circuit.graph.node_weight_mut(original_node_idx).unwrap();
                let proc_idx = original_node.get_info().proc;

                scheduled_map.visit(original_node_idx);
                subgraphs_rank_order
                    .get_mut(proc_idx as usize)
                    .unwrap()
                    .schedule();
                original_node.set_info(NodeInfo {
                    pc: pc,
                    scheduled: true,
                    ..original_node.get_info()
                });
            }
        }
        pc += 1;
        if pc >= circuit.emulator.cfg.gates_per_partition {
            let _ = write_string_to_file(format!("{:?}", circuit), "schedule-failed.dot");
            break;
        }
    }
    circuit.emulator.host_steps = pc + 1;
}
