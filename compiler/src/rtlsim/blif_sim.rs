use crate::common::circuit::*;
use crate::common::mapping::SRAMMapping;
use crate::common::primitive::*;
use crate::rtlsim::rtlsim_utils::InputStimuliMap;
use crate::fsim::sram::{SRAMEntry, SRAMInputs};
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::VecDeque;
use petgraph::{
    Undirected,
    prelude::Dfs,
    visit::{VisitMap, Visitable},
    graph::NodeIndex,
    Direction::{Incoming, Outgoing},
};

#[derive(Debug, Default)]
pub struct SRAMState {
    pub cfg: SRAMMapping,
    pub mem: Vec<SRAMEntry>,
    pub input: SRAMInputs,
    pub rddata: SRAMEntry
}

impl SRAMState {
    pub fn new(cfg: &SRAMMapping) -> Self {
        SRAMState {
            cfg: cfg.clone(),
            mem: vec![SRAMEntry::new(cfg.width_bits); 1024], // FIXME
            input: SRAMInputs::new(cfg.width_bits),
            rddata: SRAMEntry::new(cfg.width_bits)
        }
    }
}

#[derive(Debug, Default)]
pub struct BlifSimulator {
    pub circuit: Circuit,
    pub input_stimulti_blasted: InputStimuliMap,
    srams: IndexMap<u32, SRAMState>,
    topo_sort_order: Vec<NodeIndex>,
    cur_cycle: u32
}

impl BlifSimulator {
    pub fn new(circuit: Circuit, input_stimulti_blasted: InputStimuliMap) -> Self {
        let mut topo_sort_order = vec![];

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
                    Primitive::SRAMRdData => {
                        sr_nodes.push(nx);
                    }
                    _ => {
                    }
                }
            }

            // Start topological sort
            let mut q: VecDeque<NodeIndex> = VecDeque::new();
            for nidx in ff_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in in_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in sr_nodes.iter() {
                q.push_back(*nidx);
            }

            // BFS
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
                        cnode.is() != Primitive::SRAMRdData {
                        *indeg.get_mut(&cidx).unwrap() -= 1;
                        if *indeg.get(&cidx).unwrap() == 0 {
                            q.push_back(cidx);
                        }
                    }
                }
            }
        }

        // Assume that we have one SRAM per module for now
        let mut srams: IndexMap<u32, SRAMState> = IndexMap::new();
        for (m, mmap) in circuit.emul.module_mappings.iter() {
            let smap = &mmap.sram_mapping;
            if smap.width_bits == 0 {
                continue;
            }
            srams.insert(*m, SRAMState::new(&smap));

        }

        return BlifSimulator {
            circuit: circuit,
            input_stimulti_blasted: input_stimulti_blasted,
            topo_sort_order: topo_sort_order,
            srams: srams,
            cur_cycle: 0
        };
    }

    pub fn run_cycle(self: &mut Self) {
        for nidx in self.topo_sort_order.iter() {
            let parents = self.circuit.graph.neighbors_directed(*nidx, Incoming);
            let node = self.circuit.graph.node_weight(*nidx).unwrap();
            let module = node.info().coord.module;
            let prim = node.prim.clone();

            let mut node_value = 0;
            match prim {
                CircuitPrimitive::Gate { .. } => {
                    assert!(false, "Should find no gates here");
                }
                CircuitPrimitive::ConstLut { val, .. } => {
                    node_value = val;
                }
                CircuitPrimitive::Lut { inputs, output:_, table } => {
                    let mut pvs = vec![];
                    for pidx in parents {
                        let pnode = self.circuit.graph.node_weight(pidx).unwrap();
                        let pval = pnode.info().debug.val;
                        let idx = inputs.iter().position(|n| n == pnode.name()).unwrap();
                        pvs.push((idx, pval));
                    }
                    pvs.sort_by(|a, b| a.0.cmp(&b.0));
                    let ivs = pvs.iter().map(|x| x.1).collect_vec();
                    node_value = if table.contains(&ivs) { 1 } else { 0 };
                }
                CircuitPrimitive::Latch { input:_, output:_, control:_, init:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;
                }
                CircuitPrimitive::Input { name } => {
                    node_value = *self.input_stimulti_blasted
                        .get(&name).unwrap()
                        .get(self.cur_cycle as usize).unwrap() as Bit;
                }
                CircuitPrimitive::Output { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;
                }
                CircuitPrimitive::SRAMRdEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_en(node_value);

                }
                CircuitPrimitive::SRAMWrEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_en(node_value);
                }
                CircuitPrimitive::SRAMRdAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMWrAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMWrMask { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_mask(node_value, idx);
                }
                CircuitPrimitive::SRAMWrData { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_data(node_value, idx);
                }
                CircuitPrimitive::SRAMRdWrEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_en(node_value);
                }
                CircuitPrimitive::SRAMRdWrMode { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_en(node_value);
                }
                CircuitPrimitive::SRAMRdWrAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMRdData { name:_, idx } => {
                    node_value = self.srams.get(&module).unwrap().rddata.bit(idx);
                }
                _ => {
                }
            }
            self.circuit.graph.node_weight_mut(*nidx).unwrap().info_mut().debug.val = node_value;
        }

        // TODO: update self.srams

        self.cur_cycle += 1;
    }
}
