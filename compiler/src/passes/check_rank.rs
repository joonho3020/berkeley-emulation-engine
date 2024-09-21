use crate::common::*;
use blif_parser::primitives::*;
use petgraph::Direction::Incoming;

pub fn check_rank_order(circuit: &Circuit) {
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();

        match node.is() {
            Primitive::Latch | Primitive::Gate | Primitive::Input => {
                assert!(node.info().rank.asap == 0,
                    "Latch, Gate, Input should have rank.asap 0, got {:?}",
                    node.info().rank);
            }
            _ => {
                let parents = circuit.graph.neighbors_directed(nidx, Incoming);
                for pidx in parents {
                    let pnode = circuit.graph.node_weight(pidx).unwrap();

                    assert!(node.info().rank.asap > pnode.info().rank.asap,
                        "node {:?} rank.asap {:?} should be > than pnode {:?} rank.asap {:?}",
                        node.is(),
                        node.info().rank,
                        pnode.is(),
                        pnode.info().rank);

                    if node.is() != Primitive::Latch ||
                       node.is() != Primitive::Gate {
                        assert!(node.info().rank.alap > pnode.info().rank.alap,
                            "node {:?} rank.alap {:?} should be > than pnode {:?} rank.alap {:?}",
                            node.is(),
                            node.info().rank,
                            pnode.is(),
                            pnode.info().rank);
                    }
                }
            }
        }
        assert!(node.info().rank.asap <= node.info().rank.alap,
            "node {:?} rank.asap {:?} should be <= than rank.alap {:?}",
            node.is(),
            node.info().rank.asap,
            node.info().rank.alap);
    }
}
