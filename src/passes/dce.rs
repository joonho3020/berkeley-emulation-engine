
use crate::primitives::*;
use petgraph::{
     graph::NodeIndex,
     visit::{VisitMap, Visitable},
     Direction::Incoming
 };

pub fn dead_code_elimination(circuit: Circuit) -> Circuit {
    let mut graph = circuit.graph;
    let mut q: Vec<NodeIndex> = vec![];

    // Push Output nodes to the queue
    // TODO: is there a better way to search for leaf nodes?
    for nidx in graph.node_indices() {
        let node = graph.node_weight(nidx).unwrap();
        match node.is() {
            Primitives::Output => {
                q.push(nidx);
            }
            _ => {
            }
        }
    }

    // BFS starting from the Output node
    let mut vis_map = graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        if vis_map.is_visited(&nidx) {
            continue;
        }
        vis_map.visit(nidx);

        let parents = graph.neighbors_directed(nidx, Incoming);
        for pidx in parents {
            if !vis_map.is_visited(&pidx) {
                q.push(pidx);
            }
        }
    }

    // Need to delete nodes from the back due to how remove_node works
    for nidx in graph.node_indices().rev() {
        if !vis_map.is_visited(&nidx) {
            graph.remove_node(nidx);
        }
    }

    return Circuit {
        mods: circuit.mods,
        graph: graph
    }
}
