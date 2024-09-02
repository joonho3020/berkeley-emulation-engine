use std::collections::VecDeque;
use indexmap::IndexMap;
use crate::primitives::*;
use petgraph::{
    graph::{Graph, NodeIndex},
    visit::{VisitMap, Visitable},
    Direction::Outgoing,
    Undirected
};
use histo::Histogram;

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    node.set_info(NodeInfo { proc: proc, ..info });
}

pub fn partition(circuit: &mut Circuit) {
    partition_reg_boundaries(circuit);
    kaminpar_partition(circuit);
}

/// Assign each node to a a register group. An edge crossing the register group
/// boundary must have at least one FF as its vertex.
fn partition_reg_boundaries(circuit: &mut Circuit) {
    let mut q: VecDeque<NodeIndex> = VecDeque::new();
    let io_i = get_nodes_type(&circuit.graph, Primitives::Input);
    for nidx in io_i.iter() {
        q.push_back(*nidx);
    }

    let mut reggrp = 0;
    let mut reggrp_sizes: IndexMap<u32, u32> = IndexMap::new();

    let undir_graph = circuit.graph.clone().into_edge_type::<Undirected>();
    let mut vis_map = undir_graph.visit_map();
    while !q.is_empty() {
        let root = q.pop_front().unwrap();
        if vis_map.is_visited(&root) {
            continue;
        }

        let mut qq: VecDeque<NodeIndex> = VecDeque::new();
        qq.push_back(root);
        while !qq.is_empty() {
            let nidx = qq.pop_front().unwrap();
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);

            // set reggrp here
            let node = circuit.graph.node_weight_mut(nidx).unwrap();
            let info = node.get_info();
            node.set_info(NodeInfo { reggrp: reggrp, ..info });

            // update reggrp_size mapping
            let cur_size = match reggrp_sizes.get(&reggrp) {
                Some(cnt) => *cnt,
                None => 0
            };
            reggrp_sizes.insert(reggrp, cur_size + 1);

            let childs = undir_graph.neighbors(nidx);
            for cidx in childs {
                let child_type = undir_graph.node_weight(cidx).unwrap().is();
                if !vis_map.is_visited(&cidx) {
                    if (child_type != Primitives::Gate) && (child_type != Primitives::Latch) {
                        qq.push_back(cidx);
                    } else {
                        q.push_back(cidx);
                    }
                }
            }
        }
        reggrp += 1;
    }
    println!("number of register groups: {}", reggrp);
    println!("number of total nodes: {}", circuit.graph.node_count());
    assert!(vis_map.count_ones(..) == vis_map.len(), "Didn't visit all nodes when reg grouping");

    // Construct a graph where the node weights represents the size of each reggrp,
    // and the edges are reggrp boundaries.
    let mut reggrp_graph: Graph<i32, i32> = Graph::new();
    let mut reggrp_node_map: IndexMap<u32, NodeIndex> = IndexMap::new();
    let mut node_reggrp_map: IndexMap<NodeIndex, u32> = IndexMap::new();
    for (grp, sz) in reggrp_sizes.iter() {
        let nidx = reggrp_graph.add_node(*sz as i32);
        reggrp_node_map.insert(*grp, nidx);
        node_reggrp_map.insert(nidx, *grp);
    }

    let szs = reggrp_sizes.values();
    let mut histogram = Histogram::with_buckets(10);
    for sz in szs {
        histogram.add(*sz as u64);
    }
    println!("{}", histogram);

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
        for cidx in childs {
            let cnode = circuit.graph.node_weight(cidx).unwrap();
            if node.get_info().reggrp != cnode.get_info().reggrp {
                let v1 = reggrp_node_map.get(& node.get_info().reggrp).unwrap();
                let v2 = reggrp_node_map.get(&cnode.get_info().reggrp).unwrap();
                reggrp_graph.add_edge(*v1, *v2, 1);

                assert!(cnode.is() == Primitives::Latch ||
                        cnode.is() == Primitives::Gate  ||
                         node.is() == Primitives::Latch ||
                         node.is() == Primitives::Gate,
                    "node {} {:?} {:?} crosses boundary but isn't a reg type\ncnode {} {:?} {:?}",
                        nidx.index(),  node.is(),  node.get_info(),
                        cidx.index(), cnode.is(), cnode.get_info());
            }
        }
    }

    // TODO: Check if this is sufficient???
    let total_nodes = circuit.graph.node_count() as u32;
    let pcfg = &circuit.platform_cfg;
    let gates_per_module = pcfg.num_procs * pcfg.max_steps;
    let n_partitions = total_nodes / (gates_per_module / 5);
    println!("total nodes {} gates per module {} num modules {} partitions {}",
             total_nodes, gates_per_module, pcfg.num_mods, n_partitions);

    if n_partitions == 1 {
        let mut global_to_subgraph_node_map: IndexMap<NodeIndex, NodeIndex> = IndexMap::new();
        for nidx in circuit.graph.node_indices() {
            global_to_subgraph_node_map.insert(nidx, nidx);
        }
        circuit.graph_to_subgraph = global_to_subgraph_node_map;
        circuit.subcircuits.insert(
            0, SubCircuit {subgraph: circuit.graph.clone(), mapping: MappingInfo::default()});
        return;
    } else {
        // Run partitioner on the reggrp_graph
        let kaminpar = &circuit.kaminpar_cfg;
        let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
            .seed(kaminpar.seed)
            .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
            .partition_weighted(&reggrp_graph.clone().into_edge_type::<Undirected>(), n_partitions);

        println!("Partitioning done");

        // map each reggrp to partition
        let mut reggrp_partition_map: IndexMap<u32, u32> = IndexMap::new();
        match result {
            Ok(partition) => {
                assert!(reggrp_graph.node_count() == partition.len(), "Partitioned result doesn't match");

                for (nidx, part_id) in reggrp_graph.node_indices().zip(&partition) {
                    let reggrp_idx = node_reggrp_map.get(&nidx).unwrap();
                    reggrp_partition_map.insert(*reggrp_idx, *part_id);
                }
            }
            Err(_) => {
                println!("Kaminpar partitioning failed");
            }
        }


        // assign partition id to each node
        // add nodes to each subgraph
        let mut subgraphs: IndexMap<u32, HWGraph> = IndexMap::new();
        let mut global_to_subgraph_node_map: IndexMap<NodeIndex, NodeIndex> = IndexMap::new();
        for nidx in circuit.graph.node_indices() {
            let node = circuit.graph.node_weight_mut(nidx).unwrap();
            let reggrp = node.get_info().reggrp;
            let part_idx = *reggrp_partition_map.get(&reggrp).unwrap();
            node.get_info().module = part_idx;

            match subgraphs.get(&part_idx) {
                Some(_) => { }
                None => { subgraphs.insert(part_idx, HWGraph::new()); }
            }

            let subgraph = subgraphs.get_mut(&part_idx).unwrap();
            let snode_idx = subgraph.add_node(node.clone());
            global_to_subgraph_node_map.insert(nidx, snode_idx);
        }
        println!("{}", line!());

        // add edges for each subgraph
        for eidx in circuit.graph.edge_indices() {
            let ep = circuit.graph.edge_endpoints(eidx).unwrap();
            let src = circuit.graph.node_weight(ep.0).unwrap();
            let dst = circuit.graph.node_weight(ep.1).unwrap();
            if src.get_info().module == dst.get_info().module {
                let subgraph = subgraphs.get_mut(&src.get_info().module).unwrap();
// println!("module {} ep.0 {} ep.1 {} u {} v {} subgraph nodes {}",
// src.get_info().module,
// ep.0.index(),
// ep.1.index(),
// global_to_subgraph_node_map.get(&ep.0).unwrap().index(),
// global_to_subgraph_node_map.get(&ep.1).unwrap().index(),
// subgraph.node_count());

                subgraph.add_edge(
                    *global_to_subgraph_node_map.get(&ep.0).unwrap(),
                    *global_to_subgraph_node_map.get(&ep.1).unwrap(),
                    circuit.graph.edge_weight(eidx).unwrap().clone());
            }
        }
        println!("{}", line!());

        circuit.graph_to_subgraph = global_to_subgraph_node_map;
        for (id, sg) in subgraphs.iter() {
            circuit.subcircuits.insert(
                *id,SubCircuit {subgraph: sg.clone(), mapping: MappingInfo::default()});
        }
        println!("{}", line!());

        let subgraph_sizes = subgraphs.values().map(|x| x.node_count());
        let mut histogram = Histogram::with_buckets(10);
        for sz in subgraph_sizes {
            histogram.add(sz as u64);
        }
        println!("{}", histogram);
        return;
    }
}

/// Partition the circuit using the KaMinPar partitioning algorithm
fn kaminpar_partition(circuit: &mut Circuit) {
// let kaminpar = &circuit.kaminpar_cfg;
// let undirected_graph = circuit.graph.clone().into_edge_type();
// let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
// .seed(kaminpar.seed)
// .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
// .partition(&undirected_graph, circuit.platform_cfg.num_procs);

// match result {
// Ok(partition) => {
// assert!(partition.len() == circuit.graph.node_count(),
// "partition assignment doesn't match node cnt");
// for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
// set_proc(&mut circuit.graph, nidx, *pid);
// }
// circuit.emulator.used_procs = partition.iter().max().unwrap() + 1;
// }
// Err(_) => {
// println!("Kaminpar partitioning failed");
// }
// }
}
