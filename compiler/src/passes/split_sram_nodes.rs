use indexmap::{IndexMap, IndexSet};
use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::*,
    config::*,
    network::*
};
use petgraph::{
    graph::NodeIndex, visit::EdgeRef, Direction::{Incoming, Outgoing}
};

pub fn split_sram_nodes(circuit: &mut Circuit) {
    adjust_sram_nodes(circuit);
    split_sram_node_by_io(circuit);
}

/// Currently, we assume that each SRAM is mapped to one module.
/// Try reassigning SRAM nodes if this is not the case.
fn adjust_sram_nodes(circuit: &mut Circuit) {
    let mut free_modules: IndexSet<u32> = IndexSet::new();
    let mut sram_mapping: IndexMap<u32, Vec<NodeIndex>> = IndexMap::new();

    let pcfg = &circuit.platform_cfg;
    for p in 0..pcfg.num_mods {
        free_modules.insert(p);
    }

    // Obtain current mappings from SRAM nodes to modules
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != Primitive::SRAMNode {
            continue;
        }

        let module = node.info().coord.module;
        if !sram_mapping.contains_key(&module) {
            sram_mapping.insert(module, vec![]);
        }
        sram_mapping.get_mut(&module).unwrap().push(nidx);
        free_modules.swap_remove(&module);
    }

    // Try reassigning
    for (_, nodes) in sram_mapping.iter() {
        // Only one SRAM node in the current module, don't need to do anything
        if nodes.len() == 1 {
            continue;
        }
        assert!(nodes.len() - 1 <= free_modules.len(), "Not enough free modules for SRAM");

        for (i, nidx) in nodes.iter().enumerate() {
            // Skip the first node
            if i == 0 {
                continue;
            }
            let free = free_modules.pop().unwrap();
            let info = circuit.graph.node_weight_mut(*nidx).unwrap().info_mut();
            info.coord = Coordinate { module: free, proc: info.coord.proc };
        }
    }
}

#[derive(Debug)]
struct ReplaceSRAMInfo {
    pub parents: IndexMap<NodeIndex, HWEdge>,
    pub childs:  IndexMap<NodeIndex, HWEdge>,
    pub node: HWNode
}

impl ReplaceSRAMInfo {
    fn new(n: HWNode) -> Self {
        ReplaceSRAMInfo {
            parents: IndexMap::default(),
            childs : IndexMap::default(),
            node: n
        }
    }
}

fn assign_proc_to_sram_node(node: &HWNode, i: u32, pcfg: &PlatformConfig) -> HWNode {
    let coord = node.info().coord;
    let new_coord = Coordinate { proc: (i as u32) % pcfg.num_procs, ..coord };

    let mut ret = node.clone();
    ret.info_mut().coord = new_coord;
    return ret;
}

/// Split the SRAM node into nodes that represent each bit of the SRAM port
fn split_sram_node_by_io(circuit: &mut Circuit) {
    let pcfg = &circuit.platform_cfg;
    let mut sram_info: IndexMap<NodeIndex, ReplaceSRAMInfo> = IndexMap::new();
    let mut check_nodes: IndexSet<NodeIndex> = IndexSet::new();

    // Search for nodes to replace
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != Primitive::SRAMNode {
            continue;
        }

        if !sram_info.contains_key(&nidx) {
            sram_info.insert(nidx, ReplaceSRAMInfo::new(node.clone()));
        }

        // collect parent nodes & the edges
        let pedges = circuit.graph.edges_directed(nidx, Incoming);
        for pedge in pedges {
            let pidx = pedge.source();
            let edge = circuit.graph.edge_weight(pedge.id()).unwrap().clone();
            sram_info.get_mut(&nidx).unwrap().parents.insert(pidx, edge);
            check_nodes.insert(pidx);
        }

        // collect child nodes & associated edges
        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            let cidx = cedge.target();
            let edge = circuit.graph.edge_weight(cedge.id()).unwrap().clone();
            sram_info.get_mut(&nidx).unwrap().childs.insert(cidx, edge);
            check_nodes.insert(cidx);
        }
    }

    // Add bitwise SRAM port nodes
    for (_, rinfo) in sram_info.iter() {
        // Fill from processor 0
        for (i, (pidx, edge)) in rinfo.parents.iter().enumerate() {
            let mut node = assign_proc_to_sram_node(&rinfo.node, i as u32, pcfg);
            node.prim = CircuitPrimitive::from(&edge.signal);

            let sram_idx = circuit.graph.add_node(node);
            circuit.graph.add_edge(*pidx, sram_idx, edge.clone());

            assert!(!check_nodes.contains(&sram_idx),
                "sram_info contains newly added NodeIndex {:?}", sram_idx);
        }

        // Fill from processor (nprocs - 1)
        for (i, (cidx, edge)) in rinfo.childs.iter().enumerate().rev() {
            let mut node = assign_proc_to_sram_node(&rinfo.node, i as u32, pcfg);
            node.prim = CircuitPrimitive::from(&edge.signal);

            let sram_idx = circuit.graph.add_node(node);
            circuit.graph.add_edge(sram_idx, *cidx, edge.clone());

            assert!(!check_nodes.contains(&sram_idx),
                "sram_info contains newly added NodeIndex {:?}", sram_idx);
        }
    }

    // Remove SRAM nodes
    for nidx in circuit.graph.node_indices().rev() {
        if sram_info.contains_key(&nidx) {
            circuit.graph.remove_node(nidx);
        }
    }
}
