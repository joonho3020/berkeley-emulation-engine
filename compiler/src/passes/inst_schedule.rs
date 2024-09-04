use crate::primitives::*;
use indexmap::{IndexMap, IndexSet};
use petgraph::{
    graph::{EdgeIndex, NodeIndex}, visit::{EdgeRef, VisitMap, Visitable}, Direction::{Incoming, Outgoing}
};
use fixedbitset::FixedBitSet;
use std::cmp::max;
use plotters::prelude::*;

#[derive(Eq, Hash, PartialEq, Clone)]
struct InstOrProc {
    nidx: Option<NodeIndex>,
    pidx: Option<u32>,
}

#[derive(Eq, Hash, PartialEq, Clone)]
struct DepNode {
    nidx: Option<NodeIndex>,
    pidx: Option<u32>,
}

/// # Helper struct for instruction scheduling
#[derive(Debug, Default, Clone)]
struct NodeArray {
    /// Node indices of this subraph
    nodes: Vec<NodeIndex>,

    /// Next node to schedule in this subgraph
    ptr: usize,
}

impl NodeArray {
    fn push_node(&mut self, nidx: NodeIndex) {
        self.nodes.push(nidx);
    }

    fn current(&self) -> NodeIndex {
        return self.nodes[self.ptr];
    }

    fn done(&self) -> bool {
        return self.nodes.len() == self.ptr;
    }

    fn schedule(&mut self) {
        self.ptr += 1;
    }

    fn max_rank_node(&self) -> NodeIndex {
        return self.nodes[self.nodes.len() - 1];
    }
}

#[derive(Debug, Default, Clone)]
struct NetworkAvailability {
    busy: Vec<FixedBitSet>,
    ptr: usize,
    size: usize,
}

impl NetworkAvailability {
    fn new(nbits: u32, nentries: u32) -> Self {
        let mut ret = NetworkAvailability::default();
        ret.size = (nentries + 1) as usize;
        ret.busy = vec![FixedBitSet::with_capacity(nbits as usize); ret.size];
        ret.ptr = 0;
        return ret;
    }

    pub fn step(self: &mut Self) {
        self.busy.get_mut(self.ptr).unwrap().clear();
        self.ptr = (self.ptr + 1) % self.size;
    }

    pub fn is_busy(self: &Self, idx: u32, step: u32) -> bool {
        let ptr = (self.ptr + step as usize) % self.size;
        return self.busy.get(ptr).unwrap().contains(idx as usize);
    }

    pub fn set_busy(self: &mut Self, idx: u32, step: u32) {
        let ptr = (self.ptr + step as usize) % self.size;
        self.busy.get_mut(ptr).unwrap().set(idx as usize, true);
    }
}

fn child_max_rank(
    circuit: &Circuit,
    rank_order: &Vec<Vec<NodeArray>>,
    nidx: &NodeIndex
) -> u32 {
    let cnode = circuit.graph.node_weight(*nidx).unwrap();
    let cinfo = cnode.get_info();
    let max_rank_proc = rank_order
        .get(cinfo.coord.module as usize)
        .unwrap()
        .get(cinfo.coord.proc as usize)
        .unwrap();
    let max_rank_node = max_rank_proc.max_rank_node();
    let ret = if max_rank_proc.done() {
        0
    } else {
        circuit
            .graph
            .node_weight(max_rank_node)
            .unwrap()
            .get_info()
            .rank
    };
    return ret;
}

fn nw_path_usable(
    nw: &NetworkAvailability,
    src: &Coordinate,
    dst: &Coordinate,
    path: &(Coordinate, Coordinate),
    pcfg: &PlatformConfig
) -> bool {
    let mut usable = false;
    let (c1, c2) = path;
    if c1 == src && c2 == dst {
        if !nw.is_busy(dst.id(pcfg), pcfg.inter_mod_zerohop_dep_lat()) {
            usable = true;
        }
    } else if c1 == src && c2 != dst {
        if !nw.is_busy(dst.id(pcfg), pcfg.inter_mod_remote_onehop_dep_lat()) &&
           !nw.is_busy( c2.id(pcfg), pcfg.inter_mod_zerohop_dep_lat()) {
            usable = true;
        }
    } else if c1 != src && c2 == dst {
        if !nw.is_busy(dst.id(pcfg), pcfg.inter_mod_local_onehop_dep_lat()) &&
           !nw.is_busy( c1.id(pcfg), pcfg.inter_proc_dep_lat()) {
            usable = true;
        }
    } else {
        if !nw.is_busy(dst.id(pcfg), pcfg.inter_mod_twohop_dep_lat()) &&
           !nw.is_busy( c2.id(pcfg), pcfg.inter_mod_local_onehop_dep_lat()) &&
           !nw.is_busy( c1.id(pcfg), pcfg.inter_proc_dep_lat()) {
            usable = true;
        }
    }
    return usable;
}

fn set_new_path(
    nw: &mut NetworkAvailability,
    src: &Coordinate,
    dst: &Coordinate,
    path: &(Coordinate, Coordinate),
    pcfg: &PlatformConfig)
{
    let (c1, c2) = path;
    if c1 == src && c2 == dst {
        nw.set_busy(dst.id(pcfg), pcfg.inter_mod_zerohop_dep_lat());
    } else if c1 == src && c2 != dst {
        nw.set_busy(dst.id(pcfg), pcfg.inter_mod_remote_onehop_dep_lat());
        nw.set_busy( c2.id(pcfg), pcfg.inter_mod_zerohop_dep_lat());
    } else if c1 != src && c2 == dst {
        nw.set_busy(dst.id(pcfg), pcfg.inter_mod_local_onehop_dep_lat());
        nw.set_busy( c1.id(pcfg), pcfg.inter_proc_dep_lat());
    } else {
        nw.set_busy(dst.id(pcfg), pcfg.inter_mod_twohop_dep_lat());
        nw.set_busy( c2.id(pcfg), pcfg.inter_mod_local_onehop_dep_lat());
        nw.set_busy( c1.id(pcfg), pcfg.inter_proc_dep_lat());
    }
}

/// # Finds a valid instruction schedule given a partitioned graph
/// 1. Add nodes to schedule as candidates
///    - if a node is a Input or a Gate or a Latch
///    - else if dependencies are resolved add it as a candidate
/// 2. Prune the candidates if they have network contention
///     - Check for global communication conflicts
///     - For procs in a module, prune nodes that sends stuff to procs that receive from global
///     network
///     - Resolve intra-module communication conflicts
pub fn schedule_instructions(circuit: &mut Circuit) {
    let mut rank_order: Vec<Vec<NodeArray>> = vec![];
    for module in 0..circuit.emul.used_mods {
        let used_procs = circuit.emul.mod_mappings.get(&module).unwrap().used_procs as usize;
        let local_rank_order: Vec<NodeArray> = vec![NodeArray::default(); used_procs];
        rank_order.push(local_rank_order);
    }

    let mut rank_cnt: IndexMap<u32, f64> = IndexMap::new();
    let mut rank_dist: IndexMap<u32, IndexMap<Coordinate, f64>> = IndexMap::new();

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        rank_order
            .get_mut(node.get_info().coord.module as usize)
            .unwrap()
            .get_mut(node.get_info().coord.proc as usize)
            .unwrap()
            .push_node(nidx);

        let rank = node.get_info().rank;
        if !rank_cnt.contains_key(&rank) {
            rank_cnt.insert(rank, 0.0);
        }
        *rank_cnt.get_mut(&rank).unwrap() += 1.0;

        if !rank_dist.contains_key(&rank) {
            rank_dist.insert(rank, IndexMap::new());
        }
        let x = rank_dist.get_mut(&rank).unwrap();
        let c = node.get_info().coord;
        if !x.contains_key(&c) {
            x.insert(c, 0.0);
        }
        *x.get_mut(&c).unwrap() += 1.0;
    }

    let mut rank_dist_processed: IndexMap<u32, Vec<f64>> = IndexMap::new();
    for (r, rm) in rank_dist.iter() {
        let dist = rm.values().cloned().collect::<Vec<f64>>();
        rank_dist_processed.insert(*r, dist);
    }

    rank_cnt.sort_keys();
    let rank_cnt_data = rank_cnt.values().cloned().collect::<Vec<f64>>();
    let rank_cnt_plot = lowcharts::plot::XyPlot::new(rank_cnt_data.as_slice(),  80, 30, None);
    println!("rank_cnt: {:?}", rank_cnt);
    println!("{}", rank_cnt_plot);

    rank_dist_processed.sort_keys();
    for (r, rd) in rank_dist_processed.iter() {
        println!("====================== Rank: {} {} =================", r, rank_cnt.get(r).unwrap());
        let plot = lowcharts::plot::XyPlot::new(rd.as_slice(),  80, 30, None);
        println!("{}", plot);
    }

    // sort the nodes by rank within each processor
    for ro in rank_order.iter_mut() {
        for na in ro.iter_mut() {
            na.nodes.sort_by(|idx1, idx2| {
                let n1 = circuit.graph.node_weight(*idx1).unwrap();
                let n2 = circuit.graph.node_weight(*idx2).unwrap();
                n1.cmp(&n2)
            });
        }
    }

    let pcfg = &circuit.platform_cfg;
    let mut scheduled_map = circuit.graph.visit_map();
    let mut nw = NetworkAvailability::new(pcfg.num_mods * pcfg.num_procs, pcfg.inter_mod_twohop_dep_lat());

    let mut pc = 0;
    let mut global_pruned = 0;
    let mut local_pruned = 0;
    let mut candidate_cnt = 0;
    let mut busy_procs_vec: Vec<f64> = vec![];
    let mut candidate_vec: Vec<f64> = vec![];
    let mut scheduled_vec: Vec<f64> = vec![];
    let mut per_round_candidates_by_rank: IndexMap<u32, Vec<u32>> = IndexMap::new();
    let mut per_round_scheduled_by_rank:  IndexMap<u32, Vec<u32>> = IndexMap::new();
    let max_rank = *rank_cnt.keys().last().unwrap();
    for r in 0..max_rank {
        per_round_candidates_by_rank.insert(r, vec![]);
        per_round_scheduled_by_rank.insert(r,  vec![]);
    }

    while scheduled_map.count_ones(..) != scheduled_map.len() {
        let mut schedule_candidates: IndexSet<NodeIndex> = IndexSet::new();

        let mut busy_procs = 0;

        // Find all the scheduling candidates
        for (_module, local_rank_order) in rank_order.iter_mut().enumerate() {
            for (_proc, node_array) in local_rank_order.iter_mut().enumerate() {
                if node_array.done() {
                    continue;
                }

                busy_procs += 1;

                let nidx = node_array.current();
                let node = circuit.graph.node_weight(nidx).unwrap();
                let ni = node.get_info();
                let src = ni.coord;

                if node.is() == Primitives::Input ||
                   node.is() == Primitives::Gate  ||
                   node.is() == Primitives::Latch {
                    schedule_candidates.insert(nidx);
                } else {
                    let mut unresolved_dep = false;
                    let parent_edges = circuit.graph.edges_directed(nidx, Incoming);
                    for pedge in parent_edges {
                        let pnode = circuit.graph.node_weight(pedge.source()).unwrap();
                        let pi = pnode.get_info();
                        let dst = pi.coord;

                        // check for deps within a module
                        if !pi.scheduled ||
                           ((dst.module == src.module) && (dst.proc == src.proc) && (pi.pc + pcfg.intra_proc_dep_lat() > pc)) ||
                           ((dst.module == src.module) && (dst.proc != src.proc) && (pi.pc + pcfg.inter_proc_dep_lat() > pc)) {
                            unresolved_dep = true;
                            break;
                       }

                        // check for deps between modules
                       match pedge.weight().path {
                           Some(path) => {
                               if (src == path.0 && dst == path.1) && (pi.pc + pcfg.inter_mod_zerohop_dep_lat() > pc) ||
                                  (src == path.0 && dst != path.1) && (pi.pc + pcfg.inter_mod_remote_onehop_dep_lat() > pc) ||
                                  (src != path.0 && dst == path.1) && (pi.pc + pcfg.inter_mod_local_onehop_dep_lat() > pc) ||
                                  (src != path.0 && dst != path.1) && (pi.pc + pcfg.inter_mod_twohop_dep_lat() > pc) {
                                  unresolved_dep = true;
                                  break;
                               }
                           }
                           None => {
                               assert!(dst.module == src.module, "Parent scheduled, in a different module, but no path set");
                           }
                       }

                    }
                    if !unresolved_dep {
                        schedule_candidates.insert(nidx);
                    }
                }
            }
        }

        let mut global_candidates: IndexMap<NodeIndex, u32> = IndexMap::new();
        let mut  local_candidates: IndexMap<NodeIndex, u32> = IndexMap::new();
        for candidate in schedule_candidates.iter() {
            let node = circuit.graph.node_weight(*candidate).unwrap();
            let childs = circuit.graph.neighbors_directed(*candidate, Outgoing);
            let mut global = false;
            let mut crit = 0;
            for cidx in childs {
                let child = circuit.graph.node_weight(cidx).unwrap();
                crit = max(crit, child_max_rank(circuit, &rank_order, &cidx));
                if child.get_info().coord.module != node.get_info().coord.module {
                    global = true;
                }
            }
            match global {
                true =>  { global_candidates.insert(*candidate, crit); }
                false => {  local_candidates.insert(*candidate, crit); }
            }
        }

        let mut global_nodes_scheduled: Vec<NodeIndex> = vec![];
        let mut global_edges_scheduled: Vec<(EdgeIndex, InterModulePath)> = vec![];
        let mut global_criticality_vec: Vec<(&NodeIndex, &u32)> = global_candidates.iter().collect();
        global_criticality_vec.sort_by(|a, b| b.1.cmp(a.1));

        for (nidx, _criticality) in global_criticality_vec.iter() {
            let mut schedulable = true;
            let mut dst_mod_paths: IndexMap<u32, InterModulePath> = IndexMap::new();
            let node = circuit.graph.node_weight(**nidx).unwrap();
            let ninfo = node.get_info();
            let src = ninfo.coord;

            let childs = circuit.graph.neighbors_directed(**nidx, Outgoing);

            for cidx in childs {
                let child = circuit.graph.node_weight(cidx).unwrap();
                let dst = child.get_info().coord;

                assert!(dst.module < pcfg.num_mods, "dst module {} >= num_mods {}", dst.module, pcfg.num_mods);
                assert!(dst.proc < pcfg.num_procs, "dst proc {} >= num_proc {}", dst.proc, pcfg.num_procs);
                assert!(dst.id(pcfg) < pcfg.num_mods * pcfg.num_procs, "id {} >= {}", dst.id(pcfg), pcfg.num_procs * pcfg.num_mods);

                if dst == src {
                    // same proc don't have to check anything
                } else if (dst.module == src.module) && (dst.proc != src.proc) {
                    // same module
                    if nw.is_busy(dst.id(pcfg), pcfg.inter_proc_dep_lat()) {
                        schedulable = false;
                        break;
                    }
                } else if dst.module != src.module {
                    // found already existing path to dst module
                    if dst_mod_paths.contains_key(&dst.module) {
                        let path = dst_mod_paths.get(&dst.module).unwrap();
                        let path_usable = nw_path_usable(&nw, &src, &dst, &path, pcfg);
                        if path_usable {
                            continue;
                        }
                    }

                    let paths = pcfg.topology.inter_mod_paths(src, dst);
                    assert!(paths.len() > 0, "No inter module path from {:?} to {:?}", src, dst);

                    // TODO: search for short paths first
                    // no path exists yet, search for a new path
                    let mut path_exists = false;
                    for p in paths.iter() {
                        let path_usable = nw_path_usable(&nw, &src, &dst, p, pcfg);
                        if path_usable {
                            dst_mod_paths.insert(dst.module, *p);
                            path_exists = true;
                            break;
                        }
                    }

                    if !path_exists {
                        schedulable = false;
                        break;
                    }
                }
            }

            if schedulable {
                let child_edges = circuit.graph.edges_directed(**nidx, Outgoing);
                for cedge in child_edges {
                    let cnode = circuit.graph.node_weight(cedge.target()).unwrap();
                    let dst = cnode.get_info().coord;
                    if dst.module != src.module {
                        let path = dst_mod_paths.get(&dst.module).unwrap();
                        set_new_path(&mut nw, &src, &dst, path, pcfg);
                        global_edges_scheduled.push((cedge.id(), *path));
                    } else if dst.proc != src.proc {
                        nw.set_busy(dst.id(pcfg), pcfg.inter_proc_dep_lat());
                    }
                }

                global_nodes_scheduled.push(**nidx);
                scheduled_map.visit(**nidx);
                rank_order
                    .get_mut(src.module as usize)
                    .unwrap()
                    .get_mut(src.proc as usize)
                    .unwrap()
                    .schedule();
            }
        }

        for nidx in global_nodes_scheduled.iter_mut() {
            let node = circuit.graph.node_weight_mut(*nidx).unwrap();
            node.set_info(NodeInfo {
                pc: pc,
                scheduled: true,
                ..node.get_info()
            });
        }

        for (eidx, path) in global_edges_scheduled.iter_mut() {
            let edge = circuit.graph.edge_weight_mut(*eidx).unwrap();
            edge.set_path(*path);
        }

        global_pruned += global_candidates.len() - global_nodes_scheduled.len();

        let mut local_criticality_vec: Vec<(&NodeIndex, &u32)> = local_candidates.iter().collect();
        local_criticality_vec.sort_by(|a, b| b.1.cmp(a.1));

        let mut local_nodes_scheduled: Vec<NodeIndex> = vec![];
        for (nidx, _criticality) in local_criticality_vec.iter() {
            let mut schedulable = true;
            let node = circuit.graph.node_weight(**nidx).unwrap();
            let src = node.get_info().coord;
            let childs = circuit.graph.neighbors_directed(**nidx, Outgoing);

            for cidx in childs {
                let child = circuit.graph.node_weight(cidx).unwrap();
                let dst = child.get_info().coord;

                assert!(dst.module == src.module, "local node has child in different module");

                if dst.proc != src.proc &&
                   nw.is_busy(dst.id(pcfg), pcfg.inter_proc_dep_lat()) {
                    schedulable = false;
                    break;
                }
            }

            if schedulable {
                let childs = circuit.graph.neighbors_directed(**nidx, Outgoing);
                for cidx in childs {
                    let child = circuit.graph.node_weight(cidx).unwrap();
                    let dst = child.get_info().coord;
                    if dst.proc != src.proc {
                        nw.set_busy(dst.id(pcfg), pcfg.inter_proc_dep_lat());
                    }
                }

                local_nodes_scheduled.push(**nidx);
                scheduled_map.visit(**nidx);
                rank_order
                        .get_mut(src.module as usize)
                        .unwrap()
                        .get_mut(src.proc as usize)
                        .unwrap()
                        .schedule();
            }
        }

        for nidx in local_nodes_scheduled.iter_mut() {
            let node = circuit.graph.node_weight_mut(*nidx).unwrap();
            node.set_info(NodeInfo {
                pc: pc,
                scheduled: true,
                ..node.get_info()
            });
        }

        let cand_cnt = local_candidates.len() + global_candidates.len();
        let sched_cnt = local_nodes_scheduled.len() + global_nodes_scheduled.len();

        local_pruned += local_candidates.len() - local_nodes_scheduled.len();
        candidate_cnt += cand_cnt;

        busy_procs_vec.push(busy_procs as f64 / (pcfg.num_mods * pcfg.num_procs) as f64);
        candidate_vec.push(cand_cnt as f64 / (pcfg.num_mods * pcfg.num_procs) as f64);
        scheduled_vec.push(sched_cnt as f64 / (pcfg.num_mods * pcfg.num_procs) as f64);

        let mut cand_rank_cnt: IndexMap<u32, u32> = IndexMap::new();
        for cand in schedule_candidates.iter() {
            let node = circuit.graph.node_weight(*cand).unwrap();
            let rank = node.get_info().rank;
            if !cand_rank_cnt.contains_key(&rank) {
                cand_rank_cnt.insert(rank, 0);
            }
            *cand_rank_cnt.get_mut(&rank).unwrap() += 1;
        }
        let mut sched_rank_cnt: IndexMap<u32, u32> = IndexMap::new();
        for sched in global_nodes_scheduled.iter() {
            let node = circuit.graph.node_weight(*sched).unwrap();
            let rank = node.get_info().rank;
            if !sched_rank_cnt.contains_key(&rank) {
                sched_rank_cnt.insert(rank, 0);
            }
            *sched_rank_cnt.get_mut(&rank).unwrap() += 1;
        }
        for sched in local_nodes_scheduled.iter() {
            let node = circuit.graph.node_weight(*sched).unwrap();
            let rank = node.get_info().rank;
            if !sched_rank_cnt.contains_key(&rank) {
                sched_rank_cnt.insert(rank, 0);
            }
            *sched_rank_cnt.get_mut(&rank).unwrap() += 1;
        }

        for r in 0..max_rank {
            let cand_cnt = if cand_rank_cnt.contains_key(&r) {
                *cand_rank_cnt.get(&r).unwrap()
            } else {
                0
            };
            per_round_candidates_by_rank.get_mut(&r).unwrap().push(cand_cnt);

            let sched_cnt = if sched_rank_cnt.contains_key(&r) {
                *sched_rank_cnt.get(&r).unwrap()
            } else {
                0
            };
            per_round_scheduled_by_rank.get_mut(&r).unwrap().push(sched_cnt);
        }

        nw.step();
        pc += 1;

        // TODO: consider global networking lat
        if pc + 1 + circuit.platform_cfg.pc_sdm_offset() >= circuit.platform_cfg.max_steps {
            assert!(false, "Schedule failed {} nodes out of {} nodes scheduled",
                    scheduled_map.count_ones(..),
                    scheduled_map.len());
        }
    }

    // TODO: consider global networking lat
    circuit.emul.host_steps = pc + 1 + circuit.platform_cfg.pc_sdm_offset();

    let total_steps = circuit.emul.host_steps * circuit.emul.used_mods * circuit.platform_cfg.num_procs;
    let busy_plot      = lowcharts::plot::XyPlot::new(busy_procs_vec.as_slice(), 80, 30, None);
    let candidate_plot = lowcharts::plot::XyPlot::new(candidate_vec.as_slice(),  80, 30, None);
    let sched_plot     = lowcharts::plot::XyPlot::new(scheduled_vec.as_slice(),  80, 30, None);
    println!("{}", busy_plot);
    println!("{}", candidate_plot);
    println!("{}", sched_plot);
    println!("Machine ({} / {}) = {:.2} %, host_steps = {} global pruned {:.2} % local pruned {:.2} % candidates {}",
             circuit.graph.node_count(),
             total_steps,
             circuit.graph.node_count() as f32 / total_steps as f32 * 100f32,
             circuit.emul.host_steps,
             global_pruned as f32 / candidate_cnt as f32 * 100f32,
             local_pruned  as f32 / candidate_cnt as f32 * 100f32,
             candidate_cnt);

    let title = format!("{}/rank-by-scheduling-round.png", circuit.compiler_cfg.output_dir);
    let root = BitMapBackend::new(
        &title,
        (2560, 1920)).into_drawing_area();
    let _ = root.fill(&WHITE);
    let mut chart = ChartBuilder::on(&root)
        .caption("Scheduling Progress", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..circuit.emul.host_steps as f32,
                            0f32..(circuit.platform_cfg.num_procs * circuit.platform_cfg.num_mods) as f32).unwrap();
    let _ = chart.configure_mesh().draw();

    for (r, data) in per_round_candidates_by_rank.iter() {
        chart
            .draw_series(LineSeries::new(
                (0..).zip(data.iter()).map(|(a, b)| (a as f32, *b as f32)),
                &Palette99::pick(*r as usize),
            )).unwrap()
            .label(format!("Cand-{}", r))
             .legend(move |(x, y)| {
                Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)],
                &Palette99::pick(*r as usize))
            });
    }
    for (r, data) in per_round_scheduled_by_rank.iter() {
        chart
            .draw_series(LineSeries::new(
                (0..).zip(data.iter()).map(|(a, b)| (a as f32, *b as f32)),
                &Palette99::pick(*r as usize),
            )).unwrap()
            .label(format!("Sched-{}", r))
             .legend(move |(x, y)| {
                Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)],
                &Palette99::pick(*r as usize))
            });
    }
    let _ = chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw();
    let _ = root.present();
}
