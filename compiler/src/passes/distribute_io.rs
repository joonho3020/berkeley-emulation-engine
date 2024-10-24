use indexmap::{IndexMap, IndexSet};
use crate::common::{
    circuit::Circuit,
    primitive::*,
    network::Coordinate
};
use petgraph::graph::NodeIndex;

/// Distribute the IO nodes so that they only one input and one output
/// IO node is assigned to each processor
pub fn distribute_io(circuit: &mut Circuit) {
    distribute_io_with_dir(circuit, Primitive::Input);
    distribute_io_with_dir(circuit, Primitive::Output);
}

fn distribute_io_with_dir(circuit: &mut Circuit, direction: Primitive) {
    let mut free_procs: IndexSet<Coordinate> = IndexSet::new();
    let pcfg = &circuit.platform_cfg;
    for m in 0..pcfg.num_mods {
        for p in 0..pcfg.num_procs {
            free_procs.insert(Coordinate{ module: m, proc: p });
        }
    }

    let mut io_mapping: IndexMap<Coordinate, Vec<NodeIndex>> = IndexMap::new();

    // Obtain current mappings from IO nodes to coordinates
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != direction {
            continue;
        }
        let coord = node.info().coord;
        if !io_mapping.contains_key(&coord) {
            io_mapping.insert(coord, vec![]);
        }
        io_mapping.get_mut(&coord).unwrap().push(nidx);
        free_procs.swap_remove(&coord);
    }

    // Try reassigning
    for (_, nodes) in io_mapping.iter() {
        if nodes.len() == 1 {
            continue;
        }
        assert!(nodes.len() - 1 <= free_procs.len(),
            "Not enough free processor for IO {:?}", direction);

        for (i, nidx) in nodes.iter().enumerate() {
            // Skip the first node
            if i == 0 {
                continue;
            }
            let free = free_procs.pop().unwrap();
            let info = circuit.graph.node_weight_mut(*nidx).unwrap().info_mut();
            info.coord = free;
        }
    }
}
