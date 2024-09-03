use indexmap::IndexMap;
use crate::primitives::*;
use petgraph::{
    graph::{Graph, NodeIndex},
    Undirected
};
use histo::Histogram;
use kaminpar::KaminParError;
use std::cmp::max;

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    node.set_info(NodeInfo {
        coord: Coordinate { proc: proc, ..info.coord },
        ..info
    });
}

fn set_module(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    module: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    node.set_info(NodeInfo {
        coord: Coordinate { module: module, ..info.coord },
        ..info
    });
}

fn kaminpar_partition(
    g: &Graph<Box<dyn HWNode>, String, Undirected>,
    kaminpar: &KaMinParConfig,
    npartitions: u32
) -> Result<Vec<u32>, KaminParError> {
    let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
        .seed(kaminpar.seed)
        .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
        .partition(&g, npartitions);
    return result;
}

/// Return the histogram where `partition` is the output from the
/// kaminpar partitioner
fn get_partition_histogram(partition: Vec<u32>) -> Histogram {
    let mut pid_to_cnt_map: IndexMap<u32, u32> = IndexMap::new();
    for pid in partition.iter() {
        match pid_to_cnt_map.get(pid) {
            Some(sz) => {
                pid_to_cnt_map.insert(*pid, *sz + 1);
            }
            None => {
                pid_to_cnt_map.insert(*pid, 1);
            }
        }
    }
    let mut histogram = Histogram::with_buckets(10);
    for sz in pid_to_cnt_map.values() {
        histogram.add(*sz as u64);
    }
    return histogram;
}

pub fn partition(circuit: &mut Circuit) {
    kaminpar_partition_module(circuit);
    kaminpar_partition_processor(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
/// and assign a module ID to each node
fn kaminpar_partition_module(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;
    let undir_graph = circuit.graph.clone().into_edge_type();

    if pcfg.num_mods == 1 {
        circuit.emul.used_mods = 1;
    } else {
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.num_mods);
        match result {
            Ok(partition) => {
                assert!(partition.len() == circuit.graph.node_count(),
                    "partition assignment doesn't match node cnt");
                for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
                    set_module(&mut circuit.graph, nidx, *pid);
                }
                circuit.emul.used_mods = partition.iter().max().unwrap() + 1;

                println!("========== Global Partition Statistics ============");
                println!("{}", get_partition_histogram(partition));
                println!("===================================================");

            }
            Err(_) => {
                println!("Global Kaminpar partitioning failed");
            }
        }
    }
}

#[derive(Default, Debug)]
struct SubGraph {
    subgraph: HWGraph,
    to_global: IndexMap<NodeIndex, NodeIndex>,
    to_local:  IndexMap<NodeIndex, NodeIndex>
}

fn get_subgraphs(circuit: &Circuit) -> IndexMap<u32, SubGraph> {
    let used_modules = circuit.emul.used_mods;
    let mut ret: IndexMap<u32, SubGraph> = IndexMap::new();
    for i in 0..used_modules {
        ret.insert(i, SubGraph::default());
    }

    // assign nodes to subgraphs
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let module = node.get_info().coord.module;

        let sg = ret.get_mut(&module).unwrap();
        let sg_nidx = sg.subgraph.add_node(node.clone());
        sg.to_global.insert(sg_nidx, nidx);
        sg.to_local.insert(nidx, sg_nidx);
    }

    // assign edges to subgraphs
    for eidx in circuit.graph.edge_indices() {
        let ep = circuit.graph.edge_endpoints(eidx).unwrap();
        let src = circuit.graph.node_weight(ep.0).unwrap();
        let dst = circuit.graph.node_weight(ep.1).unwrap();

        if src.get_info().coord.module == dst.get_info().coord.module {
            let sg = ret.get_mut(&src.get_info().coord.module).unwrap();
            sg.subgraph.add_edge(
                *sg.to_local.get(&ep.0).unwrap(),
                *sg.to_local.get(&ep.1).unwrap(),
                circuit.graph.edge_weight(eidx).unwrap().clone());
        }
    }

    return ret;
}

/// For each subgraph assigned to each module, parttion & assign a it
/// to a processor
fn kaminpar_partition_processor(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;

    let subgraphs = get_subgraphs(circuit);
    for (module, sg) in subgraphs.iter() {
        let undir_graph = sg.subgraph.clone().into_edge_type();
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.num_procs);
        match result {
            Ok(partition) => {
                let mut max_pidx = 0;
                for (local_nidx, pidx) in sg.subgraph.node_indices().zip(&partition) {
                    let global_nidx = sg.to_global.get(&local_nidx).unwrap();
                    set_proc(&mut circuit.graph, *global_nidx, *pidx);
                    max_pidx = max(max_pidx, *pidx);
                }
                circuit.emul.mod_mappings.insert(
                    *module,
                    ModuleMapping {
                        used_procs: max_pidx + 1,
                        instructions: vec![],
                        signal_map: IndexMap::new()
                    });
                println!("========== Local Partition Statistics ============");
                println!("{}", get_partition_histogram(partition));
                println!("===================================================");
            }
            Err(_) => {
                println!("Local Kaminpar partitioning failed {}", module);
            }
        }
    }
}
