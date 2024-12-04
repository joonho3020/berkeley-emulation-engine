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

/// - split_sram_nodes
/// Given a SRAMNode which represents a SRAM blackbox, split up its
/// IO port bits into separate nodes.
pub fn split_sram_nodes(circuit: &mut Circuit) {
    spread_sram_nodes(circuit);
    check_sram_node_assignment(circuit);
    split_sram_node_by_io(circuit);
}

/// Currently, we assume that each SRAM is mapped to one module.
/// Try reassigning SRAM nodes if this is not the case.
fn spread_sram_nodes(circuit: &mut Circuit) {
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
            assert!(info.coord.module != free, "Assigning to a already assigned node");
            info.coord = Coordinate { module: free, proc: info.coord.proc };
        }
    }
}

/// Check if the assigned SRAM block fits in the platform's SRAM processor
fn check_sram_node_assignment(circuit: &Circuit) {
    let mut allocated_sram_procs: IndexSet<u32> = IndexSet::new();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() != Primitive::SRAMNode {
            continue;
        }

        if allocated_sram_procs.contains(&node.info().coord.module) {
            assert!(false,
                "Module {} already contains SRAM node",
                node.info().coord.module);
        } else {
            let pedges = circuit.graph.edges_directed(nidx, Incoming);
            let mut addr_bits = 0;
            let mut data_bits = 0;
            for pedge in pedges {
                let edge = circuit.graph.edge_weight(pedge.id()).unwrap().clone();
                match edge.signal {
                    SignalType::SRAMRdAddr   { .. } => { addr_bits += 1; }
                    SignalType::SRAMRdWrAddr { .. } => { addr_bits += 1; }
                    SignalType::SRAMWrData   { .. } => { data_bits += 1; }
                    _ => {}
                }
            }

            allocated_sram_procs.insert(node.info().coord.module);

            let w = circuit.platform_cfg.sram_width;
            let e = circuit.platform_cfg.sram_entries;
            assert!(w >= data_bits,
                "SRAM processor has {} bits per entry, got {}",
                w, data_bits);
            assert!(e >= 1 << addr_bits,
                "SRAM processor has {} entries, got {}",
                e, 1 << addr_bits);
        }
    }
}

#[derive(Debug)]
struct ReplaceSRAMInfo {
    pub parents: Vec<(NodeIndex, HWEdge)>,
    pub childs:  Vec<(NodeIndex, HWEdge)>,
    pub node: HWNode,
    pub width_bits: u32,
    pub wmask_bits: u32,
}

impl ReplaceSRAMInfo {
    fn new(n: HWNode) -> Self {
        ReplaceSRAMInfo {
            parents: vec![],
            childs : vec![],
            node: n,
            width_bits: 0,
            wmask_bits: 0
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

        let mut width_bits = 0;
        let mut wmask_bits = 0;

        // collect parent nodes & the edges
        let pedges = circuit.graph.edges_directed(nidx, Incoming);
        for pedge in pedges {
            let pidx = pedge.source();
            let edge = circuit.graph.edge_weight(pedge.id()).unwrap().clone();
            match edge.signal {
                SignalType::SRAMWrMask { .. }  => { wmask_bits += 1; }
                SignalType::SRAMWrData { .. }  => { width_bits += 1; }
                _ => { }
            }
            sram_info.get_mut(&nidx).unwrap().parents.push((pidx, edge));
            check_nodes.insert(pidx);
        }

        // Mark the width and wmask bits of this SRAM node
        sram_info.get_mut(&nidx).unwrap().wmask_bits = wmask_bits;
        sram_info.get_mut(&nidx).unwrap().width_bits = width_bits;

        // collect child nodes & associated edges
        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            let cidx = cedge.target();
            let edge = circuit.graph.edge_weight(cedge.id()).unwrap().clone();
            sram_info.get_mut(&nidx).unwrap().childs.push((cidx, edge));
            check_nodes.insert(cidx);
        }
    }

    // Add bitwise SRAM port nodes
    for (_, rinfo) in sram_info.iter() {
        // Fill from processor 0
        for (i, (pidx, edge)) in rinfo.parents.iter().enumerate() {
            match &edge.signal {
                // For bits corresponding to write masks, replicate the nodes
                // If we don't do this, we have to expand the write mask bits
                // into width_bits in the hardware implementation.
                // Since we want the number of wmask_bits and width_bits to
                // be configurable, this results in a giant crossbar in the
                // hardware which consumes a lot of resources (especially for
                // FPGAs).
                // If we expand these bits in the compiler, the hardware
                // implementation becomes simple: a simple bitwise and between
                // the expanded mask bits and the data bits.
                SignalType::SRAMWrMask { name, idx } => {
                    assert!(rinfo.wmask_bits != 0);
                    let nbits_per_mask_bit = rinfo.width_bits / rinfo.wmask_bits;

                    for j in 0..nbits_per_mask_bit {
                        let mut node = assign_proc_to_sram_node(&rinfo.node, i as u32 + j, pcfg);
                        let data_bit_idx = idx * nbits_per_mask_bit + j;
                        node.prim = CircuitPrimitive::SRAMWrMask {
                            name: name.to_string(),
                            idx: data_bit_idx,
                        };

                        let sram_idx = circuit.graph.add_node(node);
                        circuit.graph.add_edge(*pidx, sram_idx, edge.clone());
                        assert!(!check_nodes.contains(&sram_idx),
                            "sram_info contains newly added NodeIndex {:?}", sram_idx);
                    }
                }
                _ => {
                    let mut node = assign_proc_to_sram_node(&rinfo.node, i as u32, pcfg);
                    node.prim = CircuitPrimitive::from(&edge.signal);

                    let sram_idx = circuit.graph.add_node(node);
                    circuit.graph.add_edge(*pidx, sram_idx, edge.clone());
                    assert!(!check_nodes.contains(&sram_idx),
                        "sram_info contains newly added NodeIndex {:?}", sram_idx);
                }
            }
        }

        // Fill from processor (nprocs - 1)
        for (i, (cidx, edge)) in rinfo.childs.iter().enumerate().rev() {
            let mut node = assign_proc_to_sram_node(&rinfo.node, i as u32, pcfg);
            node.prim = CircuitPrimitive::from(&edge.signal);

            assert!(node.is() != Primitive::SRAMNode, "Node to add should not be a SRAMNode type");

            let sram_idx = circuit.graph.add_node(node);
            circuit.graph.add_edge(sram_idx, *cidx, edge.clone());

            assert!(!check_nodes.contains(&sram_idx),
                "sram_info contains newly added NodeIndex {:?}", sram_idx);
        }
    }

    // Remove SRAM nodes
    for nidx in circuit.graph.node_indices().rev() {
        match circuit.graph.node_weight(nidx) {
            Some(node) => {
                if node.is() == Primitive::SRAMNode {
                    circuit.graph.remove_node(nidx);
                }
            }
            None => { }
        }
    }
}
