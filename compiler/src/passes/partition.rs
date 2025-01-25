use indexmap::IndexMap;
use crate::common::{
    circuit::Circuit,
    hwgraph::*,
    mapping::*,
    config::*,
    network::*
};
use petgraph::{
    graph::{Graph, NodeIndex}, Direction::Outgoing, Undirected
};
use histo::Histogram;
use kaminpar::KaminParError;

fn edge_weight(circuit: &Circuit, src_idx: &NodeIndex, dst_idx: &NodeIndex) -> f32 {
    let dst = circuit.graph.node_weight(*dst_idx).unwrap().info();
    let src_child_cnt = circuit.graph.neighbors_directed(*src_idx, Outgoing).count();
    if dst.rank.critical() {
        0.0
    } else {
        (src_child_cnt - 1) as f32 / src_child_cnt as f32
    }
}

pub fn set_edge_weights(circuit: &mut Circuit, communication: u32) {
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let cost_f32 = 1000.0 * (communication as f32  - edge_weight(circuit, &e.0, &e.1));
        circuit.graph.edge_weight_mut(eidx).unwrap().weight = Some(cost_f32 as i32);
    }
}

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

/// HACK: KaMinPar with edge weights has some implicit assumption about the weights.
/// If the communication cost is lower when 2, it throws a unexplainable assertion.
/// To avoid this issue, simply add 2 to the edge weights if it is smaller than two.
fn adjust_communication_cost(c: u32) -> u32 {
    if c < 2 {
        c + 2
    } else {
        c
    }
}

/// Partition the design onto multiple modules and processors within a module
pub fn partition(circuit: &mut Circuit) {
    let pcfg = circuit.platform_cfg.clone();

    // Module partition
    let inter_mod_comm_cost = adjust_communication_cost(pcfg.inter_mod_nw_lat * 2 + pcfg.dmem_wr_lat);
    set_edge_weights(circuit, inter_mod_comm_cost);
    kaminpar_partition_module(circuit);

    // Processor partition
    let inter_proc_comm_cost = adjust_communication_cost(pcfg.inter_proc_nw_lat + pcfg.dmem_wr_lat);
    set_edge_weights(circuit, inter_proc_comm_cost);
    kaminpar_partition_processor(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
/// and assign a module ID to each node
pub fn kaminpar_partition_module(circuit: &mut Circuit) {
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
pub fn kaminpar_partition_processor(circuit: &mut Circuit) {
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
