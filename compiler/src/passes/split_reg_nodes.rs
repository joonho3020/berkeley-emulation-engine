use std::hash::{DefaultHasher, Hash, Hasher};
use indexmap::IndexMap;
use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::*,
};
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
    Direction::Outgoing
};

type SplitRegInfo = (NodeIndex, NodeIndex, EdgeIndex);
type NewNodeInfo = (HWNode, HWEdge, HWEdge);

/// If a register node (Latch/Gate) has a child that is also a register node,
/// insert a dummy LUT with table [[1]] (i.e. a passthrough) to remove the
/// ordering constraints in between the nodes
pub fn split_reg_nodes(circuit: &mut Circuit) {
    let mut new_nodes: IndexMap<SplitRegInfo, NewNodeInfo> = IndexMap::new();

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != Primitive::Latch && node.is() != Primitive::Gate {
            continue;
        }

        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            let eidx = cedge.id();
            let cidx = cedge.target();
            let cnode = circuit.graph.node_weight(cidx).unwrap();
            if cnode.is() == Primitive::Latch || cnode.is() == Primitive::Gate {
                // Try generating a unique name for this new node
                let mut hasher = DefaultHasher::new();
                format!("{}-{}", node.name(), cnode.name()).hash(&mut hasher);
                let new_node_name = format!("SPLIT-{}-{}", node.name(), hasher.finish());

                // Passthrough LUT node
                let lut = CircuitPrimitive::Lut {
                    inputs: vec![ node.name().to_string() ],
                    output: new_node_name.clone(),
                    table: vec![ vec![1] ]
                };
                let mut lut_node = HWNode::new(lut);
                lut_node.info_mut().coord = node.info().coord;

                assert!(!new_nodes.contains_key(&(nidx, cidx, eidx)),
                    "{:?} -> {:?} dummy lut already inserted",
                    nidx, cidx);

                // Parent & child both are Latch/Gate. SignalType must be a `Wire` type
                let pedge = HWEdge::new(SignalType::Wire { name: node.name().to_string() });
                let nedge = HWEdge::new(SignalType::Wire { name: new_node_name });

                new_nodes.insert((nidx, cidx, eidx), (lut_node, pedge, nedge));
            }
        }
    }

    // Add new nodes and edges
    for ((pidx, cidx, _), (lut_node, pedge, nedge)) in new_nodes.iter() {
        let lut_idx = circuit.graph.add_node(lut_node.clone());
        circuit.graph.add_edge(*pidx, lut_idx, pedge.clone());
        circuit.graph.add_edge(lut_idx, *cidx, nedge.clone());
    }

    // Remove the original edges
    for ((_, _, eidx), _) in new_nodes.iter() {
        circuit.graph.remove_edge(*eidx);
    }
}
