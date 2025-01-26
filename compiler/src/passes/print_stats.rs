use crate::common::circuit::Circuit;
use petgraph::{algo::connected_components, Direction::{Incoming, Outgoing}};

pub fn print_stats(circuit: &Circuit) {
    let cc = connected_components(&circuit.graph);
    let nodes = circuit.graph.node_count();
    let edges = circuit.graph.edge_count();
    println!("Connected components: {} V: {} E: {}", cc, nodes, edges);

    let mut global_communication_edges = 0;
    let mut local_communication_edges = 0;
    let mut proc_communication_eges = 0;

    for eidx in circuit.graph.edge_indices() {
        let eps = circuit.graph.edge_endpoints(eidx).unwrap();
        let src = circuit.graph.node_weight(eps.0).unwrap();
        let dst = circuit.graph.node_weight(eps.1).unwrap();

        let sc = src.info().coord;
        let dc = dst.info().coord;

        if sc == dc {
            proc_communication_eges += 1;
        } else if sc.module == dc.module {
            local_communication_edges += 1;
        } else {
            global_communication_edges += 1;
        }
    }

    let mut all_parents_internal = 0;
    let mut all_childs_internal = 0;

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();

        let parents = circuit.graph.neighbors_directed(nidx, Incoming);
        let mut local = true;
        for p in parents {
            let parent = circuit.graph.node_weight(p).unwrap();
            if node.info().coord != parent.info().coord {
                local = false;
            }
        }
        if local {
            all_parents_internal += 1;
        }

        let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
        let mut local = true;
        for c in childs {
            let child = circuit.graph.node_weight(c).unwrap();
            if node.info().coord != child.info().coord {
                local = false;
            }
        }
        if local {
            all_childs_internal += 1;
        }
    }

    println!("Total Edges: {}", edges);
    println!("- Module    crossing: {}", global_communication_edges);
    println!("- Processor crossing: {}", local_communication_edges);
    println!("- Processor internal: {}", proc_communication_eges);

    println!("Total nodes: {}", nodes);
    println!("- All parents in same processor: {}", all_parents_internal);
    println!("- All childs  in same processor: {}", all_childs_internal);
}
