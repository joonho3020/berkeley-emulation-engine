use crate::primitives::*;
use petgraph::graph::NodeIndex;

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
    kaminpar_partition(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
fn kaminpar_partition(circuit: &mut Circuit) {
    let kaminpar = &circuit.emulator.kaminpar;
    let undirected_graph = circuit.graph.clone().into_edge_type();
    let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
        .seed(kaminpar.seed)
        .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
        .partition(&undirected_graph, circuit.emulator.cfg.module_sz);

    match result {
        Ok(partition) => {
            assert!(partition.len() == circuit.graph.node_count(),
                "partition assignment doesn't match node cnt");
// println!("partition: {:?}", partition);
            for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
                set_proc(&mut circuit.graph, nidx, *pid);
            }
            circuit.emulator.used_procs = partition.iter().max().unwrap() + 1;
        }
        Err(_) => {
            println!("Kaminpar partitioning failed");
        }
    }
}
