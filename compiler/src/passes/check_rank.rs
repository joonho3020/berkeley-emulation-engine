use crate::primitives::*;
use petgraph::Direction::Incoming;

pub fn check_rank_order(circuit: &Circuit) {
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();

        match node.is() {
            Primitives::Latch | Primitives::Gate | Primitives::Input => {
                assert!(node.get_info().rank == 0, "Latch, Gate, Input should have rank 0, got {}", node.get_info().rank);
            }
            _ => {
                let mut parents = circuit.graph.neighbors_directed(nidx, Incoming).detach();
                while let Some(pidx) = parents.next_node(&circuit.graph) {
                    let pnode = circuit.graph.node_weight(pidx).unwrap();
                    assert!(node.get_info().rank > pnode.get_info().rank,
                        "node {:?} rank {} should be > than pnode {:?} rank {}",
                        node.is(),
                        node.get_info().rank,
                        pnode.is(),
                        pnode.get_info().rank);
                }
            }
        }
    }
}
