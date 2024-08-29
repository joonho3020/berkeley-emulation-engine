use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};
use std::{cmp::max, collections::VecDeque};

fn set_rank(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.get_info();
    let new_rank = max(info.rank, rank);
    node.set_info(NodeInfo {
        rank: new_rank,
        ..info
    })
}

pub fn find_rank_order(circuit: &mut Circuit) {
    let mut max_rank: u32 = 0;

    // compute indeg for the entire graph
    let mut indeg: IndexMap<NodeIndex, u32> = IndexMap::new();
    for nidx in circuit.graph.node_indices() {
        indeg.insert(nidx, 0);
    }
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let dst = e.1;
        *indeg.get_mut(&dst).unwrap() += 1;
    }

    let mut vis_map = circuit.graph.visit_map();
    for curidx in circuit.graph.node_indices() {
        if vis_map.is_visited(&curidx) {
            continue;
        }

        // Found new connected component
        // DFS to search for all the relevant nodes
        let mut ff_nodes: Vec<NodeIndex> = vec![];
        let mut in_nodes: Vec<NodeIndex> = vec![];
        let mut stack: VecDeque<NodeIndex> = VecDeque::new();
        stack.push_back(curidx);

        while !stack.is_empty() {
            let top = stack.pop_back().unwrap();
            if vis_map.is_visited(&top) {
                continue;
            }
            vis_map.visit(top);

            let node = circuit.graph.node_weight(top).unwrap();
            match node.is() {
                Primitives::Latch | Primitives::Gate => {
                    ff_nodes.push(top);
                }
                Primitives::Input => {
                    in_nodes.push(top);
                }
                _ => {
                }
            }

            let mut adj = circuit.graph.neighbors_undirected(top).detach();
            while let Some(adjidx) = adj.next_node(&circuit.graph) {
                if !vis_map.is_visited(&adjidx) {
                    stack.push_back(adjidx);
                }
            }
        }

        // Start topological sort
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in in_nodes.iter() {
            q.push(*nidx);
            set_rank(&mut circuit.graph, *nidx, 0);
        }
        for nidx in ff_nodes.iter() {
            q.push(*nidx);
            set_rank(&mut circuit.graph, *nidx, 0);
        }

        // BFS
        let mut topo_sort_order: Vec<NodeIndex> = vec![];
        let mut topo_vis_map = circuit.graph.visit_map();
        while !q.is_empty() {
            let nidx = q.remove(0);
            topo_vis_map.visit(nidx);
            topo_sort_order.push(nidx);

            let mut childs = circuit.graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&circuit.graph) {
// println!("nidx: {:?} node: {:#?} cidx {:?}, cnode: {:#?} indeg: {}",
// nidx, circuit.graph.node_weight(nidx).unwrap(),
// cidx, circuit.graph.node_weight(cidx).unwrap(),
// *indeg.get(&cidx).unwrap());
                let cnode = circuit.graph.node_weight(cidx).unwrap();
                if !topo_vis_map.is_visited(&cidx) &&
                    cnode.is() != Primitives::Gate &&
                    cnode.is() != Primitives::Latch &&
                    cnode.is() != Primitives::Input {
                    *indeg.get_mut(&cidx).unwrap() -= 1;
                    if *indeg.get(&cidx).unwrap() == 0 {
                        q.push(cidx);
                    }
                }
            }
        }

        // Set rank based on the topo sorted order
        for nidx in topo_sort_order.iter() {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            if node.is() != Primitives::Gate || node.is() != Primitives::Latch {
                let mut max_parent_rank = 0;
                let mut parents = circuit.graph.neighbors_directed(*nidx, Incoming).detach();
                while let Some(pidx) = parents.next_node(&circuit.graph) {
                    let parent = circuit.graph.node_weight(pidx).unwrap();
                    max_parent_rank = max(max_parent_rank, parent.get_info().rank);
                }
                set_rank(&mut circuit.graph, *nidx, max_parent_rank + 1);
                if max_parent_rank + 1 > max_rank {
                    max_rank = max_parent_rank + 1;
                }
            }
        }
    }
    println!("Max rank of this graph: {}", max_rank);
    assert!(
        vis_map.count_ones(..) == vis_map.len(),
        "Missed {} nodes out of {} nodes while topo sorting",
        vis_map.count_ones(..),
        vis_map.len());
}
