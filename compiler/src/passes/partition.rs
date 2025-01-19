use indexmap::IndexMap;
use crate::common::{
    circuit::Circuit,
    hwgraph::*,
    mapping::*,
    config::*,
    network::*
};
use crate::passes::prepartition_set_rank::set_edge_weights;
use petgraph::{
    graph::{Graph, NodeIndex}, Undirected
};
use histo::Histogram;
use kaminpar::KaminParError;

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.coord = Coordinate { proc: proc, ..info.coord };
}

fn set_module(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    module: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.coord = Coordinate { module: module, ..info.coord };
}

/// Call the KaMinPar partitioner
fn kaminpar_partition(
    g: &Graph<HWNode, HWEdge, Undirected>,
    kaminpar: &KaMinParConfig,
    npartitions: u32
) -> Result<Vec<u32>, KaminParError> {
    let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
        .seed(kaminpar.seed)
        .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
        .partition_edge_weighted(&g, npartitions);
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

/// Partition the design onto multiple modules and processors within a module
pub fn partition(circuit: &mut Circuit) {
    let pcfg = circuit.platform_cfg.clone();

    // Module partition
    set_edge_weights(circuit, pcfg.inter_mod_nw_lat * 2 + pcfg.dmem_wr_lat);
    kaminpar_partition_module(circuit);

    // Processor partition
    set_edge_weights(circuit, pcfg.inter_proc_nw_lat + pcfg.dmem_wr_lat);
    kaminpar_partition_processor(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
/// and assign a module ID to each node
fn kaminpar_partition_module(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;
    let undir_graph = circuit.graph.clone().into_edge_type();

    if pcfg.num_mods != 1 {
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.num_mods);
        match result {
            Ok(partition) => {
                assert!(partition.len() == circuit.graph.node_count(),
                    "partition assignment doesn't match node cnt");
                for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
                    set_module(&mut circuit.graph, nidx, *pid);
                }

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
    /// Subgraph
    subgraph: HWGraph,

    /// Subgraph NodeIndex to graph NodeIndex
    to_global: IndexMap<NodeIndex, NodeIndex>,

    /// Graph NodeIndex to subgraph NodeIndex
    to_local:  IndexMap<NodeIndex, NodeIndex>
}

fn get_subgraphs(circuit: &Circuit) -> IndexMap<u32, SubGraph> {
    let used_modules = circuit.platform_cfg.num_mods;
    let mut ret: IndexMap<u32, SubGraph> = IndexMap::new();
    for i in 0..used_modules {
        ret.insert(i, SubGraph::default());
    }

    // assign nodes to subgraphs
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let module = node.info().coord.module;

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

        if src.info().coord.module == dst.info().coord.module {
            let sg = ret.get_mut(&src.info().coord.module).unwrap();
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
                for (local_nidx, pidx) in sg.subgraph.node_indices().zip(&partition) {
                    let global_nidx = sg.to_global.get(&local_nidx).unwrap();
                    set_proc(&mut circuit.graph, *global_nidx, *pidx);
                }
                println!("========== Local Partition Statistics ============");
                println!("{}", get_partition_histogram(partition));
                println!("===================================================");
            }
            Err(_) => {
                println!("Local Kaminpar partitioning failed {}", module);
            }
        }
    }

    for i in 0..circuit.platform_cfg.num_mods {
        circuit.emul.module_mappings.insert(i, ModuleMapping::default());
    }
}
