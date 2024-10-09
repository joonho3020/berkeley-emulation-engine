use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::*
};
use indexmap::IndexMap;
use petgraph::{
    graph::NodeIndex, visit::EdgeRef, Direction::Outgoing
};

struct ConstLutNode {
    pub node: HWNode,
    pub edge: HWEdge
}

impl ConstLutNode {
    fn new(n: HWNode, e: HWEdge) -> Self {
        Self {
            node: n,
            edge: e
        }
    }
}

/// Replicate `ConstLut`s so that we don't have to broadcast these nodes
pub fn replicate_consts(circuit: &mut Circuit) {
    let mut const_info: IndexMap<NodeIndex, Vec<ConstLutNode>> = IndexMap::new();
    let mut remove_nodes: Vec<NodeIndex> = vec![];

    // Search for all the nodes that has ConstLut as a input
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != Primitive::ConstLut {
            continue;
        }
        remove_nodes.push(nidx);

        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            let cidx = cedge.target();
            let edge = circuit.graph.edge_weight(cedge.id()).unwrap();
            if !const_info.contains_key(&cidx) {
                const_info.insert(cidx, vec![]);
            }
            const_info.get_mut(&cidx).unwrap().push(ConstLutNode::new(node.clone(), edge.clone()));
        }
    }

    // Add separate ConstLut parent nodes
    for (nidx, const_luts) in const_info.iter() {
        for constlut in const_luts {
            let new_idx = circuit.graph.add_node(constlut.node.clone());
            circuit.graph.add_edge(new_idx, *nidx, constlut.edge.clone());
        }
    }

    // Remove original ConstLut nodes
    for ridx in remove_nodes.iter() {
        circuit.graph.remove_node(*ridx);
    }
}
