use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Outgoing
};

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
// partition_on_register_boundary(circuit);
    kaminpar_partition(circuit);
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


fn kaminpar_partition(circuit: &mut Circuit) {
    let kaminpar = &circuit.emulator.cfg.kaminpar;
    let undirected_graph = circuit.graph.clone().into_edge_type();
    let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
        .seed(kaminpar.seed)
        .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
        .partition(&undirected_graph, circuit.emulator.cfg.module_sz);

    match result {
        Ok(partition) => {
            assert!(partition.len() == circuit.graph.node_count(),
                "partition assignment doesn't match node cnt");
            println!("partition: {:?}", partition);

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
