use crate::primitives::*;
use std::time::Instant;
use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};

pub fn dead_code_elimination(circuit: &mut Circuit) {
    let mut q: Vec<NodeIndex> = vec![];

    let in_bfs_start = Instant::now();

    let mut i_vismap = circuit.graph.visit_map();
    for nidx in circuit.io_i.keys() {
        // Push Input nodes to the queue
        q.push(*nidx);

        // BFS starting from the Input node
        while !q.is_empty() {
            let nidx = q.remove(0);
            if i_vismap.is_visited(&nidx) {
                continue;
            }
            i_vismap.visit(nidx);
            let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
            for cidx in childs {
                if !i_vismap.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }
    }

    let in_bfs_time = in_bfs_start.elapsed().as_millis();

    let out_bfs_start = Instant::now();
    // Push Output nodes to the queue
    for nidx in circuit.io_o.keys() {
        q.push(*nidx);
    }

    // BFS starting from the Output node
    let mut o_vismap = circuit.graph.visit_map();
    while !q.is_empty() {
        let nidx = q.remove(0);
        if o_vismap.is_visited(&nidx) {
            continue;
        }
        o_vismap.visit(nidx);

        let parents = circuit.graph.neighbors_directed(nidx, Incoming);
        for pidx in parents {
            if !o_vismap.is_visited(&pidx) {
                q.push(pidx);
            }
        }
    }
    let out_bfs_time = out_bfs_start.elapsed().as_millis();



    let remove_start = Instant::now();

    // Find nodes to delete (can't delete here due to immutable borrow)
    for nidx in circuit.graph.node_indices().rev() {
        if !o_vismap.is_visited(&nidx) || !i_vismap.is_visited(&nidx) {
            circuit.graph.remove_node(nidx);
        }
    }

    let remove_time = remove_start.elapsed().as_millis();

    let remap_start = Instant::now();

    // Reset the IO mappings
    circuit.io_i.clear();
    circuit.io_o.clear();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        match node.is() {
            Primitives::Input => {
                circuit.io_i.insert(nidx, node.name().to_string());
            }
            Primitives::Output => {
                circuit.io_o.insert(nidx, node.name().to_string());
            }
            _ => { }
        }
    }
    let remap_time = remap_start.elapsed().as_millis();
    println!("DCE time");
    println!("Out BFS: {}", out_bfs_time);
    println!("In BFS: {}", in_bfs_time);
    println!("remove: {}", remove_time);
    println!("reset io: {}", remap_time);
}
