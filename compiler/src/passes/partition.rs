use indexmap::{IndexMap, IndexSet};
use crate::common::*;
use petgraph::{
    graph::{Graph, NodeIndex}, visit::EdgeRef, Direction::{Incoming, Outgoing}, Undirected
};
use histo::Histogram;
use kaminpar::KaminParError;

fn set_proc(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    proc: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.coord = Coordinate { proc: proc, ..info.coord };
}

fn set_module(
    graph: &mut HWGraph,
    nidx: NodeIndex,
    module: u32
) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.coord = Coordinate { module: module, ..info.coord };
}

/// Call the KaMinPar partitioner
fn kaminpar_partition(
    g: &Graph<HWNode, HWEdge, Undirected>,
    kaminpar: &KaMinParConfig,
    npartitions: u32
) -> Result<Vec<u32>, KaminParError> {
    let result = kaminpar::PartitionerBuilder::with_epsilon(kaminpar.epsilon)
        .seed(kaminpar.seed)
        .threads(std::num::NonZeroUsize::new(kaminpar.nthreads as usize).unwrap())
        .partition(&g, npartitions);
    return result;
}

/// Return the histogram where `partition` is the output from the
/// kaminpar partitioner
fn get_partition_histogram(partition: Vec<u32>) -> Histogram {
    let mut pid_to_cnt_map: IndexMap<u32, u32> = IndexMap::new();
    for pid in partition.iter() {
        match pid_to_cnt_map.get(pid) {
            Some(sz) => {
                pid_to_cnt_map.insert(*pid, *sz + 1);
            }
            None => {
                pid_to_cnt_map.insert(*pid, 1);
            }
        }
    }
    let mut histogram = Histogram::with_buckets(10);
    for sz in pid_to_cnt_map.values() {
        histogram.add(*sz as u64);
    }
    return histogram;
}

/// Partition the design onto multiple modules and processors within a module
pub fn partition(circuit: &mut Circuit) {
    kaminpar_partition_module(circuit);
    kaminpar_partition_processor(circuit);
    adjust_sram_nodes(circuit);
    split_sram_node_by_io(circuit);
}

/// Partition the circuit using the KaMinPar partitioning algorithm
/// and assign a module ID to each node
fn kaminpar_partition_module(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;
    let undir_graph = circuit.graph.clone().into_edge_type();

    if pcfg.num_mods != 1 {
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.num_mods);
        match result {
            Ok(partition) => {
                assert!(partition.len() == circuit.graph.node_count(),
                    "partition assignment doesn't match node cnt");
                for (nidx, pid) in circuit.graph.node_indices().zip(&partition) {
                    set_module(&mut circuit.graph, nidx, *pid);
                }

                println!("========== Global Partition Statistics ============");
                println!("{}", get_partition_histogram(partition));
                println!("===================================================");

            }
            Err(_) => {
                println!("Global Kaminpar partitioning failed");
            }
        }
    }
}

#[derive(Default, Debug)]
struct SubGraph {
    /// Subgraph
    subgraph: HWGraph,

    /// Subgraph NodeIndex to graph NodeIndex
    to_global: IndexMap<NodeIndex, NodeIndex>,

    /// Graph NodeIndex to subgraph NodeIndex
    to_local:  IndexMap<NodeIndex, NodeIndex>
}

fn get_subgraphs(circuit: &Circuit) -> IndexMap<u32, SubGraph> {
    let used_modules = circuit.platform_cfg.num_mods;
    let mut ret: IndexMap<u32, SubGraph> = IndexMap::new();
    for i in 0..used_modules {
        ret.insert(i, SubGraph::default());
    }

    // assign nodes to subgraphs
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let module = node.info().coord.module;

        let sg = ret.get_mut(&module).unwrap();
        let sg_nidx = sg.subgraph.add_node(node.clone());
        sg.to_global.insert(sg_nidx, nidx);
        sg.to_local.insert(nidx, sg_nidx);
    }

    // assign edges to subgraphs
    for eidx in circuit.graph.edge_indices() {
        let ep = circuit.graph.edge_endpoints(eidx).unwrap();
        let src = circuit.graph.node_weight(ep.0).unwrap();
        let dst = circuit.graph.node_weight(ep.1).unwrap();

        if src.info().coord.module == dst.info().coord.module {
            let sg = ret.get_mut(&src.info().coord.module).unwrap();
            sg.subgraph.add_edge(
                *sg.to_local.get(&ep.0).unwrap(),
                *sg.to_local.get(&ep.1).unwrap(),
                circuit.graph.edge_weight(eidx).unwrap().clone());
        }
    }

    return ret;
}

/// For each subgraph assigned to each module, parttion & assign a it
/// to a processor
fn kaminpar_partition_processor(circuit: &mut Circuit) {
    let kaminpar = &circuit.kaminpar_cfg;
    let pcfg = &circuit.platform_cfg;

    let subgraphs = get_subgraphs(circuit);
    for (module, sg) in subgraphs.iter() {
        let undir_graph = sg.subgraph.clone().into_edge_type();
        let result = kaminpar_partition(&undir_graph, &kaminpar, pcfg.num_procs);
        match result {
            Ok(partition) => {
                for (local_nidx, pidx) in sg.subgraph.node_indices().zip(&partition) {
                    let global_nidx = sg.to_global.get(&local_nidx).unwrap();
                    set_proc(&mut circuit.graph, *global_nidx, *pidx);
                }
                println!("========== Local Partition Statistics ============");
                println!("{}", get_partition_histogram(partition));
                println!("===================================================");
            }
            Err(_) => {
                println!("Local Kaminpar partitioning failed {}", module);
            }
        }
    }

    for i in 0..circuit.platform_cfg.num_mods {
        circuit.emul.module_mappings.insert(
            i,
            ModuleMapping { proc_mappings: IndexMap::new() });
    }
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
