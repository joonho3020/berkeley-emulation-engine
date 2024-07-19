use crate::primitives::*;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::Incoming,
    Undirected,
};

pub fn partition(circuit: Circuit) -> Circuit {
    let num_partitions = 2;
    let undirected_graph = circuit.graph.clone().into_edge_type();
    let partition = kaminpar::PartitionerBuilder::with_epsilon(0.03)
        .seed(123)
        .threads(std::num::NonZeroUsize::new(6).unwrap())
        .partition(&undirected_graph, num_partitions);

    println!("{:?}", partition);
    circuit
}
