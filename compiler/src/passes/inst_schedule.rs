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

#[derive(Eq, Hash, PartialEq, Clone)]
struct DepNode {
    nidx: Option<NodeIndex>,
    pidx: Option<u32>,
}

/// # Helper struct for instruction scheduling
#[derive(Debug, Default, Clone)]
struct NodeArray {
    /// Node indices of this subraph
    nodes: Vec<NodeIndex>,

    /// Next node to schedule in this subgraph
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

fn child_max_rank(
    circuit: &Circuit,
    rank_order: &Vec<Vec<NodeArray>>,
    nidx: &NodeIndex
) -> u32 {
    let cnode = circuit.graph.node_weight(*nidx).unwrap();
    let cinfo = cnode.get_info();
    let max_rank_node = rank_order
        .get(cinfo.module as usize)
        .unwrap()
        .get(cinfo.proc as usize)
        .unwrap()
        .max_rank_node();
    let rank = circuit
        .graph
        .node_weight(max_rank_node)
        .unwrap()
        .get_info()
        .rank;
    return rank;
}

fn prune_global_conflicts(
    circuit: &Circuit,
    candidates: &IndexSet<NodeIndex>,
    rank_order: &Vec<Vec<NodeArray>>
) -> IndexSet<NodeIndex> {
    let mut pruned: IndexSet<NodeIndex> = IndexSet::new();
    let mut dep_graph: Graph<DepNode, usize> = Graph::default();
    let mut module_nodes: IndexMap<DepNode, NodeIndex> = IndexMap::new();
    let mut criticality: IndexMap<NodeIndex, u32> = IndexMap::new();
    let mut local_nodes: IndexSet<NodeIndex> = IndexSet::new();

    // Construct a bipartite graph where the edges look like:
    // Schedule Candidate Node Index -> Module index of children
    // This graph is used to check whether there are multiple network
    // inputs to this module at this cycle.
    for nidx in candidates.iter() {
        let inode = DepNode {
            nidx: Some(*nidx),
            pidx: None
        };
        let inode_idx = dep_graph.add_node(inode);
        let mut crit = 0;
        let mut global = false;

        let node = circuit.graph.node_weight(*nidx).unwrap();
        let module = node.get_info().module;

        let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
        for cidx in childs {
            let child_node = circuit.graph.node_weight(cidx).unwrap();
            let child_module = child_node.get_info().module;
            if module != child_module {
                let module_node = DepNode {
                    nidx: None,
                    pidx: Some(child_module)
                };
                if !module_nodes.contains_key(&module_node) {
                    let module_node_idx = dep_graph.add_node(module_node.clone());
                    module_nodes.insert(module_node.clone(), module_node_idx);
                }
                dep_graph.add_edge(inode_idx, *module_nodes.get(&module_node).unwrap(), 0);

                // compute criticality
                crit = max(crit, child_max_rank(circuit, rank_order, &cidx));
                global = true;
            }
        }
        criticality.insert(inode_idx, crit);

        match global {
            true  => {}
            false => { local_nodes.insert(*nidx); }
        }
    }

    // Select instructions to schedule greedily based on the criticality
    let mut criticality_vec: Vec<(&NodeIndex, &u32)> = criticality.iter().collect();
    criticality_vec.sort_by(|a, b| b.1.cmp(a.1));

    let mut dep_graph_vis = dep_graph.visit_map();

    for (nidx, _) in criticality_vec.iter() {
        let inst_node = dep_graph.node_weight(**nidx).unwrap();
        let childs = dep_graph.neighbors_directed(**nidx, Outgoing);

        // check if scheduleable
        let mut scheduleable = true;
        for cidx in childs {
            if dep_graph_vis.is_visited(&cidx) {
                scheduleable = false;
                break;
            }
        }

        if scheduleable {
            // mark the children procs as visisted in the dependency graph
            let childs_to_remove  = dep_graph.neighbors_directed(**nidx, Outgoing);
            for cidx in childs_to_remove {
                dep_graph_vis.visit(cidx);
            }

            let original_node_idx = inst_node.nidx.unwrap();
            pruned.insert(original_node_idx);
        }
    }
    pruned.append(&mut local_nodes);
    return pruned;
}

fn prune_interlevel_conflicts(
    circuit: &Circuit,
    candidates: &IndexSet<NodeIndex>,
    rank_order: &Vec<Vec<NodeArray>>
) -> IndexSet<NodeIndex> {
    let mut pruned: IndexSet<NodeIndex> = IndexSet::new();
    let mut dep_graph: Graph<DepNode, usize> = Graph::default();
    let mut dep_nodes: IndexMap<DepNode, NodeIndex> = IndexMap::new();
    let mut criticality: IndexMap<NodeIndex, u32> = IndexMap::new();

    // nodes communicating over global switch
    let mut global_nodes: IndexSet<NodeIndex> = IndexSet::new();

    // construct dependency graph
    for nidx in candidates.iter() {
        let node = circuit.graph.node_weight(*nidx).unwrap();
        let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);

        let inode = DepNode {
            nidx: Some(*nidx),
            pidx: None
        };
        let inode_idx = dep_graph.add_node(inode);

        let mut crit = 0;
        let mut global = false;
        for cidx in childs {
            let cnode = circuit.graph.node_weight(cidx).unwrap();

            // check if this node has global connections
            if node.get_info().module != cnode.get_info().module {
                global = true;
            }

            // construct dependency graph
            let cinfo = cnode.get_info();
            let ninfo = node.get_info();

            if cinfo.module != ninfo.module || cinfo.proc != ninfo.proc {
                let unique_id = cinfo.module * circuit.platform_cfg.num_procs + cinfo.proc;
                let dep_node = DepNode {
                    nidx: None,
                    pidx: Some(unique_id)
                };

                if !dep_nodes.contains_key(&dep_node) {
                    let didx = dep_graph.add_node(dep_node.clone());
                    dep_nodes.insert(dep_node.clone(), didx);
                }
                dep_graph.add_edge(inode_idx, *dep_nodes.get(&dep_node).unwrap(), 0);

                // compute criticality
                crit = max(crit, child_max_rank(circuit, rank_order, &cidx));
            }
        }

        match global {
            true  => { global_nodes.insert(inode_idx); }
            false => { criticality.insert(inode_idx, crit); }
        }
    }

    // First select the nodes that has global communication
    let mut dep_graph_vis = dep_graph.visit_map();
    for gidx in global_nodes.iter() {
        let inode = dep_graph.node_weight(*gidx).unwrap();
        let childs = dep_graph.neighbors_directed(*gidx, Outgoing);

        let mut schedulable = true;
        for cidx in childs {
            if dep_graph_vis.is_visited(&cidx) {
                schedulable = false;
                break;
            }
        }

        if schedulable {
            // mark the children procs as visisted in the dependency graph
            let childs_to_remove  = dep_graph.neighbors_directed(*gidx, Outgoing);
            for cidx in childs_to_remove {
                dep_graph_vis.visit(cidx);
            }
            let original_node_index = inode.nidx.unwrap();
            pruned.insert(original_node_index);
        }
    }

    // Next, select the remaining nodes based on criticality
    let mut criticality_vec: Vec<(&NodeIndex, &u32)> = criticality.iter().collect();
    criticality_vec.sort_by(|a, b| b.1.cmp(a.1));

    for (nidx, _) in criticality_vec.iter() {
        let inst_node = dep_graph.node_weight(**nidx).unwrap();
        let childs = dep_graph.neighbors_directed(**nidx, Outgoing);

        let mut schedulable = true;
        for cidx in childs {
            if dep_graph_vis.is_visited(&cidx) {
                schedulable = false;
                break;
            }
        }

        if schedulable {
            let childs_to_remove = dep_graph.neighbors_directed(**nidx, Outgoing);
            for cidx in childs_to_remove {
                dep_graph_vis.visit(cidx);
            }
            let original_node_idx = inst_node.nidx.unwrap();
            pruned.insert(original_node_idx);
        }
    }

    return pruned;
}

/// # Finds a valid instruction schedule given a partitioned graph
/// 1. Add nodes to schedule as candidates
///    - if a node is a Input or a Gate or a Latch
///    - else if dependencies are resolved add it as a candidate
/// 2. Prune the candidates if they have network contention
///     - Check for global communication conflicts
///     - For procs in a module, prune nodes that sends stuff to procs that receive from global
///     network
///     - Resolve intra-module communication conflicts
pub fn schedule_instructions(circuit: &mut Circuit) {
    let mut rank_order: Vec<Vec<NodeArray>> = vec![];
    for module in 0..circuit.emul.used_mods {
        let used_procs = circuit.emul.mod_mappings.get(&module).unwrap().used_procs as usize;
        let local_rank_order: Vec<NodeArray> = vec![NodeArray::default(); used_procs];
        rank_order.push(local_rank_order);
    }

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        rank_order
            .get_mut(node.get_info().module as usize)
            .unwrap()
            .get_mut(node.get_info().proc as usize)
            .unwrap()
            .push_node(nidx);
    }

    // sort the nodes by rank within each processor
    for ro in rank_order.iter_mut() {
        for na in ro.iter_mut() {
            na.nodes.sort_by(|idx1, idx2| {
                let n1 = circuit.graph.node_weight(*idx1).unwrap();
                let n2 = circuit.graph.node_weight(*idx2).unwrap();
                n1.cmp(&n2)
            });
        }
    }

    let mut pc = 0;
    let mut scheduled_map = circuit.graph.visit_map();
    let pcfg = &circuit.platform_cfg;

    let mut global_pruned = 0;
    let mut local_pruned = 0;
    let mut candidate_cnt = 0;

    while scheduled_map.count_ones(..) != scheduled_map.len() {
        println!("nodes left to schedule {}", scheduled_map.len() - scheduled_map.count_ones(..));
        let mut schedule_candidates: IndexSet<NodeIndex> = IndexSet::new();

        // Find all the scheduling candidates
        for (_module, local_rank_order) in rank_order.iter_mut().enumerate() {
            for (_proc, node_array) in local_rank_order.iter_mut().enumerate() {
                if node_array.done() {
                    continue;
                }
                let nidx = node_array.current();
                let node = circuit.graph.node_weight(nidx).unwrap();
                let ninfo = node.get_info();

                if node.is() == Primitives::Input ||
                   node.is() == Primitives::Gate  ||
                   node.is() == Primitives::Latch {
                    schedule_candidates.insert(nidx);
                } else {
                    let parents = circuit.graph.neighbors_directed(nidx, Incoming);
                    let mut unresolved_dep = false;
                    for pidx in parents {
                        let pnode = circuit.graph.node_weight(pidx).unwrap();
                        let pinfo = pnode.get_info();

                        // TODO: Add global scheduling constraints here
                        if !pinfo.scheduled ||
                           ((pinfo.module == ninfo.module) && (pinfo.proc == ninfo.proc) && (pinfo.pc + pcfg.local_dep_lat()  > pc)) ||
                           ((pinfo.module == ninfo.module) && (pinfo.proc != ninfo.proc) && (pinfo.pc + pcfg.remote_dep_lat() > pc))
                        {
                            unresolved_dep = true;
                            break;
                        }
                    }
                    if !unresolved_dep {
                        schedule_candidates.insert(nidx);
                    }
                }
            }
        }
        println!("schedule candidates: {}", schedule_candidates.len());
        assert!(schedule_candidates.len() > 0, "no more schedule candidates");

        let pruned_1 = prune_global_conflicts(circuit, &schedule_candidates, &rank_order);
        assert!(pruned_1.len() > 0, "No more schedulable entries after global prune");
        println!("pruned_1: {}", pruned_1.len());

        let pruned_2 = prune_interlevel_conflicts(circuit, &pruned_1, &rank_order);
        assert!(pruned_2.len() > 0, "No more schedulable entries after local prune");
        println!("pruned_2: {}", pruned_2.len());

        candidate_cnt += schedule_candidates.len();
        global_pruned += schedule_candidates.len() - pruned_1.len();
        local_pruned  += pruned_1.len() - pruned_2.len();

        for nidx in pruned_2.iter() {
            let node = circuit.graph.node_weight_mut(*nidx).unwrap();
            let ninfo = node.get_info();

            assert!(!scheduled_map.is_visited(nidx), "{:?} already scheduled", *nidx);

            scheduled_map.visit(*nidx);
            rank_order
                .get_mut(ninfo.module as usize)
                .unwrap()
                .get_mut(ninfo.proc as usize)
                .unwrap()
                .schedule();
            node.set_info(NodeInfo {
                pc: pc,
                scheduled: true,
                ..node.get_info()
            });
        }
        pc += 1;

        // TODO: consider global networking lat
        if pc + 1 + circuit.platform_cfg.pc_sdm_offset() >= circuit.platform_cfg.max_steps {
// let _ = write_string_to_file(circuit.print_scheduled(), "schedule-failed.dot");
            assert!(false, "Schedule failed {} nodes out of {} nodes scheduled",
                    scheduled_map.count_ones(..),
                    scheduled_map.len());
        }
    }

    // TODO: consider global networking lat
    circuit.emul.host_steps = pc + 1 + circuit.platform_cfg.pc_sdm_offset();

    let total_steps = circuit.emul.host_steps * circuit.emul.used_mods * circuit.platform_cfg.num_procs;
    println!("Machine ({} / {}) = {:.2} %, host_steps = {} global pruned {:.2} % local pruned {:.2} % candidates {}",
             circuit.graph.node_count(),
             total_steps,
             circuit.graph.node_count() as f32 / total_steps as f32 * 100f32,
             circuit.emul.host_steps,
             global_pruned as f32 / candidate_cnt as f32 * 100f32,
             local_pruned  as f32 / candidate_cnt as f32 * 100f32,
             candidate_cnt);
}
