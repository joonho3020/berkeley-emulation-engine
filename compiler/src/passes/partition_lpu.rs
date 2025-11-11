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

fn set_lpu_stage(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    mem_tile: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.lpu.mem_tile = Some(mem_tile);
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
pub fn partition_lpu(circuit: &mut Circuit) {
    kaminpar_partition_stages(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
/// and assign a module ID to each node
fn kaminpar_partition_stages(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;
    let undir_graph = circuit.graph.clone().into_edge_type();

    if pcfg.lpu_stream_stages() != 1 {
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.lpu_memtiles_per_superlane);
        match result {
            Ok(partition) => {
                assert!(partition.len() == circuit.graph.node_count(),
                    "partition assignment doesn't match node cnt");
                for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
                    set_lpu_stage(&mut circuit.graph, nidx, *pid);
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
