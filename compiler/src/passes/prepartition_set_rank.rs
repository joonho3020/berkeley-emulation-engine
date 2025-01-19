use crate::common::{
    circuit::Circuit, primitive::*
};
use crate::passes::set_rank::{set_rank_alap, set_rank_asap};
use indexmap::IndexMap;
use petgraph::{
    graph::NodeIndex, prelude::Dfs, visit::{VisitMap, Visitable}, Direction::{Incoming, Outgoing}, Undirected
};
use std::{cmp::{max, min}, collections::VecDeque};

pub fn prepartition_find_rank_order(circuit: &mut Circuit) {
    prepartition_find_asap_rank_order(circuit);
    prepartition_find_alap_rank_order(circuit);
}

pub fn init_rank_order(circuit: &mut Circuit) {
    for nidx in circuit.graph.node_indices() {
        set_rank_asap(&mut circuit.graph, nidx, 0);
        set_rank_alap(&mut circuit.graph, nidx, 0);
    }
}

fn edge_weight(circuit: &Circuit, src_idx: &NodeIndex, dst_idx: &NodeIndex) -> f32 {
    let dst = circuit.graph.node_weight(*dst_idx).unwrap().info();
    let src_child_cnt = circuit.graph.neighbors_directed(*src_idx, Outgoing).count();
    if dst.rank.alap - dst.rank.asap == 0 {
        0.0
    } else {
        (src_child_cnt - 1) as f32 / src_child_cnt as f32
    }
}

pub fn set_edge_weights(circuit: &mut Circuit, communication: u32) {
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let cost_f32 = 300.0 * (communication as f32  - edge_weight(circuit, &e.0, &e.1));
        circuit.graph.edge_weight_mut(eidx).unwrap().weight = Some(cost_f32 as i32);
    }
}

fn prepartition_find_asap_rank_order(circuit: &mut Circuit) {
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
        let mut sr_nodes: Vec<NodeIndex> = vec![];

        let mut dfs = Dfs::new(&undir_graph, curidx);
        while let Some(nx) = dfs.next(&undir_graph) {
            vis_map.visit(nx);

            let node = circuit.graph.node_weight(nx).unwrap();
            match node.is() {
                Primitive::Latch => {
                    ff_nodes.push(nx);
                }
                Primitive::Gate => {
                    ff_nodes.push(nx);
                }
                Primitive::Input => {
                    in_nodes.push(nx);
                }
                Primitive::ConstLut => {
                    in_nodes.push(nx);
                }
                Primitive::SRAMNode => {
                    sr_nodes.push(nx);
                }
                _ => {
                }
            }
        }

        // Start topological sort
        let mut q: VecDeque<NodeIndex> = VecDeque::new();
        for nidx in in_nodes.iter() {
            q.push_back(*nidx);
            set_rank_asap(&mut circuit.graph, *nidx, 0);
        }
        for nidx in ff_nodes.iter() {
            q.push_back(*nidx);
            set_rank_asap(&mut circuit.graph, *nidx, 0);
        }
        for nidx in sr_nodes.iter() {
            q.push_back(*nidx);
            set_rank_asap(&mut circuit.graph, *nidx, 0);
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
                if !topo_vis_map.is_visited(&cidx)    &&
                    cnode.is() != Primitive::Gate     &&
                    cnode.is() != Primitive::Latch    &&
                    cnode.is() != Primitive::Input    &&
                    cnode.is() != Primitive::ConstLut &&
                    cnode.is() != Primitive::SRAMNode {
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
            if node.is() != Primitive::Gate     &&
               node.is() != Primitive::Latch    &&
               node.is() != Primitive::Input    &&
               node.is() != Primitive::ConstLut &&
               node.is() != Primitive::SRAMNode {
                let mut max_parent_rank = 0;
                let parents = circuit.graph.neighbors_directed(*nidx, Incoming);
                for pidx in parents {
                    let parent = circuit.graph.node_weight(pidx).unwrap();
                    max_parent_rank = max(max_parent_rank, parent.info().rank.asap);
                }
                set_rank_asap(&mut circuit.graph, *nidx, max_parent_rank + 1);
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

fn prepartition_find_alap_rank_order(circuit: &mut Circuit) {
    let mut odeg: IndexMap<NodeIndex, u32> = IndexMap::new();
    let max_rank = circuit.emul.max_rank;

    // compute odeg for the entire graph
    for nidx in circuit.graph.node_indices() {
        odeg.insert(nidx, 0);
    }
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let src = e.0;
        *odeg.get_mut(&src).unwrap() += 1;
    }

    let undir_graph = circuit.graph.clone().into_edge_type::<Undirected>();
    let mut visited = 0;
    let mut vis_map = circuit.graph.visit_map();
    for curidx in circuit.graph.node_indices() {
        if vis_map.is_visited(&curidx) {
            continue;
        }

        let mut q: VecDeque<NodeIndex> = VecDeque::new();
        let mut dfs = Dfs::new(&undir_graph, curidx);
        while let Some(nx) = dfs.next(&undir_graph) {
            vis_map.visit(nx);

            let node = circuit.graph.node_weight(nx).unwrap();
            match node.is() {
                Primitive::Latch | Primitive::Gate | Primitive::SRAMNode => {
                    q.push_back(nx);
                    set_rank_alap(&mut circuit.graph, nx, 0);
                }
                Primitive::Output => {
                    q.push_back(nx);
                    set_rank_alap(&mut circuit.graph, nx, max_rank);
                }
                _ => {
                }
            }
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

            let parents = circuit.graph.neighbors_directed(nidx, Incoming);
            for pidx in parents {
                let pnode = circuit.graph.node_weight(pidx).unwrap();
                if !topo_vis_map.is_visited(&pidx) &&
                   pnode.is() != Primitive::Gate     ||
                   pnode.is() != Primitive::Latch    ||
                   pnode.is() != Primitive::Input    ||
                   pnode.is() != Primitive::ConstLut ||
                   pnode.is() != Primitive::SRAMNode {
                   *odeg.get_mut(&pidx).unwrap() -= 1;
                    if *odeg.get(&pidx).unwrap() == 0 {
                        q.push_back(pidx);
                    }
                }
            }
        }

        // Set rank based on the topo sorted order
        for nidx in topo_sort_order.iter() {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            if node.is() != Primitive::Gate     &&
               node.is() != Primitive::Latch    &&
               node.is() != Primitive::Input    &&
               node.is() != Primitive::ConstLut &&
               node.is() != Primitive::SRAMNode {
                let mut min_child_rank = circuit.emul.max_rank + 1;
                let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
                for cidx in childs {
                    let child = circuit.graph.node_weight(cidx).unwrap();
                    if child.is() == Primitive::Gate  ||
                       child.is() == Primitive::Latch ||
                       child.is() == Primitive::SRAMNode {
                        min_child_rank = min(min_child_rank, circuit.emul.max_rank + 1);
                    } else {
                        min_child_rank = min(min_child_rank, child.info().rank.alap);
                    };
                }
                set_rank_alap(&mut circuit.graph, *nidx, min_child_rank - 1);
            }
        }
        visited += topo_sort_order.len();
    }
    assert!(
        visited == vis_map.len(),
        "Visited {} nodes out of {} nodes while topo sorting",
        visited,
        vis_map.len());
}
