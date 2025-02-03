use std::collections::VecDeque;

use crate::common::{circuit::Circuit, primitive::Primitive, hwgraph::{HWNode, HWEdge}};
use petgraph::{algo::connected_components, graph::{Graph, NodeIndex}, visit::{VisitMap, Visitable}, Undirected};

pub fn find_register_boundaries(c: &Circuit) {
    let mut circuit = c.clone();

    // remove all register nodes
    for nidx in circuit.graph.node_indices().rev() {
        match circuit.graph.node_weight(nidx).unwrap().is() {
            Primitive::Gate |
            Primitive::Latch => {
                circuit.graph.remove_node(nidx);
            }
            _ => { }
        }
    }

    let undir_graph: Graph<HWNode, HWEdge, Undirected> = circuit.graph.clone().into_edge_type();

    let mut vis_map = undir_graph.visit_map();
    let mut cc_sizes = vec![];

    for nidx in undir_graph.node_indices() {
        if vis_map.is_visited(&nidx) {
            continue;
        } else {
            let mut cc_size = 0;
            let mut q: VecDeque<NodeIndex> = VecDeque::new();
            q.push_back(nidx);

            while !q.is_empty() {
                let nidx = q.pop_front().unwrap();
                if vis_map.is_visited(&nidx) {
                    continue;
                }
                vis_map.visit(nidx);
                cc_size += 1;

                for cidx in undir_graph.neighbors_undirected(nidx) {
                    q.push_back(cidx);
                }
            }
            cc_sizes.push(cc_size);
        }
    }
    println!("Register boundary partition sizes: {:?}", cc_sizes);
    assert!(false);

}
