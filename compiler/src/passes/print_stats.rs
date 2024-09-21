use crate::common::*;
use petgraph::algo::connected_components;

pub fn print_stats(circuit: &Circuit) {
    let cc = connected_components(&circuit.graph);
    let nodes = circuit.graph.node_count();
    let edges = circuit.graph.edge_count();
    println!("Connected components: {} V: {} E: {}", cc, nodes, edges);
}
