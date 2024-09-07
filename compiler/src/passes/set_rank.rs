use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::{
    graph::NodeIndex,
    prelude::Dfs,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
    Undirected
};
use std::{cmp::max, collections::VecDeque};

fn set_rank(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info();
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

    let undir_graph = circuit.graph.clone().into_edge_type::<Undirected>();
    let mut visited = 0;
    let mut vis_map = circuit.graph.visit_map();
    for curidx in circuit.graph.node_indices() {
        if vis_map.is_visited(&curidx) {
            continue;
        }

        // Found new connected component
        // DFS to search for all the relevant nodes
        let mut ff_nodes: Vec<NodeIndex> = vec![];
        let mut in_nodes: Vec<NodeIndex> = vec![];

        let mut dfs = Dfs::new(&undir_graph, curidx);
        while let Some(nx) = dfs.next(&undir_graph) {
            vis_map.visit(nx);

            let node = circuit.graph.node_weight(nx).unwrap();
            match node.is() {
                Primitives::Latch | Primitives::Gate => {
                    ff_nodes.push(nx);
                }
                Primitives::Input => {
                    in_nodes.push(nx);
                }
                _ => {
                }
            }
        }

        // Start topological sort
        let mut q: VecDeque<NodeIndex> = VecDeque::new();
        for nidx in in_nodes.iter() {
            q.push_back(*nidx);
            set_rank(&mut circuit.graph, *nidx, 0);
        }
        for nidx in ff_nodes.iter() {
            q.push_back(*nidx);
            set_rank(&mut circuit.graph, *nidx, 0);
        }

        // BFS
        let mut topo_sort_order: Vec<NodeIndex> = vec![];
        let mut topo_vis_map = circuit.graph.visit_map();
        while !q.is_empty() {
            let nidx = q.pop_front().unwrap();
            if topo_vis_map.is_visited(&nidx) {
                continue;
            }

            topo_vis_map.visit(nidx);
            topo_sort_order.push(nidx);

            let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
            for cidx in childs {
                let cnode = circuit.graph.node_weight(cidx).unwrap();
                if !topo_vis_map.is_visited(&cidx) &&
                    cnode.is() != Primitives::Gate &&
                    cnode.is() != Primitives::Latch &&
                    cnode.is() != Primitives::Input {
                    *indeg.get_mut(&cidx).unwrap() -= 1;
                    if *indeg.get(&cidx).unwrap() == 0 {
                        q.push_back(cidx);
                    }
                }
            }
        }

        // Set rank based on the topo sorted order
        for nidx in topo_sort_order.iter() {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            if node.is() != Primitives::Gate &&
               node.is() != Primitives::Latch &&
               node.is() != Primitives::Input {
                let mut max_parent_rank = 0;
                let parents = circuit.graph.neighbors_directed(*nidx, Incoming);
                for pidx in parents {
                    let parent = circuit.graph.node_weight(pidx).unwrap();
                    max_parent_rank = max(max_parent_rank, parent.info().rank);
                }
                set_rank(&mut circuit.graph, *nidx, max_parent_rank + 1);
                if max_parent_rank + 1 > max_rank {
                    max_rank = max_parent_rank + 1;
                }
            }
        }
        visited += topo_sort_order.len();
    }
    println!("Max rank of this graph: {}", max_rank);
    assert!(
        visited == vis_map.len(),
        "Visited {} nodes out of {} nodes while topo sorting",
        visited,
        vis_map.len());
}
