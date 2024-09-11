use crate::primitives::*;
use crate::utils::save_graph_pdf;
use full_palette::RED;
use indexmap::{IndexMap, IndexSet};
use petgraph::{
    graph::{EdgeIndex, EdgeReference, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing}
};
use fixedbitset::FixedBitSet;
use plotters::prelude::*;
use std::collections::BTreeSet;
use std::cmp::Ordering;

#[derive(Debug, Default, Clone)]
struct NetworkPorts {
    nbits: u32,
    busy: IndexMap<u32, FixedBitSet>
}

impl NetworkPorts {
    fn new(nbits: u32) -> Self {
        let mut ret = NetworkPorts::default();
        ret.nbits = nbits;
        ret.busy = IndexMap::new();
        return ret;
    }

    fn add_pc_if_empty(self: &mut Self, pc: u32) {
        if !self.busy.contains_key(&pc) {
            self.busy.insert(pc, FixedBitSet::with_capacity(self.nbits as usize));
        }
    }

    fn is_busy(self: &mut Self, idx: u32, pc: u32) -> bool {
        self.add_pc_if_empty(pc);
        return self.busy.get(&pc).unwrap().contains(idx as usize);
    }

    fn set_busy(self: &mut Self, idx: u32, pc: u32) {
        self.add_pc_if_empty(pc);
        self.busy.get_mut(&pc).unwrap().set(idx as usize, true);
    }

    fn cnt_busy(self: &mut Self, pc: u32) -> u32 {
        self.add_pc_if_empty(pc);
        return self.busy.get(&pc).unwrap().count_ones(..) as u32;
    }
}

#[derive(Debug, Default, Clone)]
struct NetworkAvailability {
    iports: NetworkPorts,
    oports: NetworkPorts
}

impl NetworkAvailability {
    fn new(nbits: u32) -> Self {
        NetworkAvailability {
            iports: NetworkPorts::new(nbits),
            oports: NetworkPorts::new(nbits)
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Copy)]
struct NodeIndexMobility {
    index: NodeIndex,
    mob: u32
}

impl NodeIndexMobility {
    fn new(index: NodeIndex, mob: u32) -> Self {
        NodeIndexMobility {
            index: index,
            mob: mob
        }
    }
}

impl Ord for NodeIndexMobility {
    fn cmp(&self, other: &Self) -> Ordering {
        self.mob.cmp(&other.mob).then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for NodeIndexMobility {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn schedule_instructions(circuit: &mut Circuit) {
    schedule_instructions_3(circuit);
}

fn print_tail_graph(
    circuit: &Circuit,
    per_pc_scheduled: &Vec<u32>,
    debug_scheduled_nodes: &Vec<NodeIndex>,
    pc_min: u32,
    rank: u32)
{
    let tail_length = circuit.compiler_cfg.dbg_tail_length;
    let tail_threshold = circuit.compiler_cfg.dbg_tail_threshold;
    let mut print_nodes: IndexMap<Coordinate, Vec<NodeIndex>> = IndexMap::new();
    let mut print_nodes_set: IndexSet<NodeIndex> = IndexSet::new();
    let mut tail_start_pc = pc_min;

    for (i, w) in per_pc_scheduled.windows(tail_length as usize).enumerate() {
        let is_tail = w.iter()
            .map(|x| *x <= tail_threshold)
            .reduce(|a, b| a && b)
            .unwrap();

        if is_tail {
            tail_start_pc = i as u32 + pc_min;
            for nidx in debug_scheduled_nodes.iter() {
                let node = circuit.graph.node_weight(*nidx).unwrap();
                if node.info().pc >= tail_start_pc  &&
                   node.info().pc <  tail_start_pc + tail_length as u32 {
                    if !print_nodes.contains_key(&node.info().coord) {
                        print_nodes.insert(node.info().coord, vec![]);
                    }
                    print_nodes.get_mut(&node.info().coord).unwrap().push(*nidx);
                    print_nodes_set.insert(*nidx);

                    let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
                    for c in childs {
                        let cnode = circuit.graph.node_weight(c).unwrap();
                        if !print_nodes.contains_key(&cnode.info().coord) {
                            print_nodes.insert(cnode.info().coord, vec![]);
                        }
                        print_nodes.get_mut(&cnode.info().coord).unwrap().push(c);
                        print_nodes_set.insert(c);
                    }
                }
            }
            // just print the first tail
            break;
        }
    }

    if print_nodes_set.is_empty() {
        return;
    }

    let indent: &str = "    ";
    let mut outstring = "digraph {\n".to_string();
    outstring.push_str(&format!("{}graph [fontsize=10 compound=true];\n", indent));
    outstring.push_str(&format!("{}node  [fontsize=4];\n", indent));

    for (coord, node_indices) in print_nodes.iter() {
        outstring.push_str(&format!("{}subgraph cluster_{}_{} {{\n",
                                    indent, coord.module, coord.proc));

        outstring.push_str(&format!("{}{}label=\"{}-{}\"\n",
                                    indent, indent, coord.module, coord.proc));

        for nidx in node_indices {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            outstring.push_str(&format!("{}{} {} [ label = {:?} ]\n",
                                        indent, indent, nidx.index(),
                                        format!("{} {:?}\nasap: {} alap: {} pc: {}",
                                                node.name(),
                                                node.is(),
                                                node.info().rank.asap,
                                                node.info().rank.alap,
                                                node.info().pc)));
        }
        outstring.push_str(&format!("{}}}\n", indent));
    }

    for eidx in circuit.graph.edge_indices() {
        let edge = circuit.graph.edge_endpoints(eidx).unwrap();
        if print_nodes_set.contains(&edge.0) && print_nodes_set.contains(&edge.1) {
            outstring.push_str(&format!("{}{} -> {};\n", indent, edge.0.index(), edge.1.index()));
        }
    }
    outstring.push_str("}");

    let dot = format!("{}/tail-graph-rank-{}-pc-{}.dot",
                      circuit.compiler_cfg.output_dir, rank, tail_start_pc);
    let pdf = format!("{}/tail-graph-rank-{}-pc-{}.pdf",
                      circuit.compiler_cfg.output_dir, rank, tail_start_pc);
    let _ = save_graph_pdf(&outstring, &dot, &pdf);
}

fn print_scheduling_stats(
    circuit: &Circuit,
    must_scheduled_data: Vec<u32>,
    be_scheduled_data: Vec<u32>,
    nw_utilization: Vec<u32>)
{
    let title = format!("{}/scheduling-progress.png", circuit.compiler_cfg.output_dir);
    let root = BitMapBackend::new(&title, (2560, 1920)).into_drawing_area();
    let _ = root.fill(&WHITE);

    let mut chart = ChartBuilder::on(&root)
        .caption("Scheduling Progress", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..circuit.emul.host_steps as f32,
                            0f32..circuit.platform_cfg.total_procs() as f32).unwrap();

    let _ = chart.configure_mesh().draw();

    chart
        .draw_series(LineSeries::new(
            (0..).zip(must_scheduled_data.iter()).map(|(a, b)| (a as f32, *b as f32)),
            &RED
        )).unwrap()
        .label("must-scheduled".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &RED)
        });

    chart
        .draw_series(LineSeries::new(
            (0..).zip(be_scheduled_data.iter()).map(|(a, b)| (a as f32, *b as f32)),
            &BLUE
        )).unwrap()
        .label("be-scheduled".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &BLUE)
        });

    chart
        .draw_series(LineSeries::new(
            (0..).zip(nw_utilization.iter()).map(|(a, b)| (a as f32, *b as f32)),
            &GREEN
        )).unwrap()
        .label("nw-utilization".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &GREEN)
        });

    let _ = chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw();
    let _ = root.present();
    let _ = save_graph_pdf( 
        &format!("{:?}", circuit),
        &format!("{}/{}.dot", circuit.compiler_cfg.output_dir, circuit.compiler_cfg.top_module),
        &format!("{}/{}.pdf", circuit.compiler_cfg.output_dir, circuit.compiler_cfg.top_module));
}

fn dependency_resolved(
    cur: &Box<dyn HWNode>,
    par: &Box<dyn HWNode>,
    pedge: EdgeReference<HWEdge, u32>,
    pcfg: &PlatformConfig,
    pc: u32
) -> bool {
    let mut unresolved_dep = false;
    let pi = par.info();
    let dst = pi.coord;

    let ni = cur.info();
    let src = ni.coord;

    // check for deps within a module
    if !pi.scheduled ||
       ((dst.module == src.module) && (dst.proc == src.proc) && (pi.pc + pcfg.intra_proc_dep_lat() > pc)) ||
       ((dst.module == src.module) && (dst.proc != src.proc) && (pi.pc + pcfg.inter_proc_dep_lat() > pc)) {
        unresolved_dep = true;
   }

    // check for deps between modules
   match &pedge.weight().route {
       Some(route) => {
           if pi.pc + pcfg.nw_route_dep_lat(&route) > pc {
              unresolved_dep = true;
           }
       }
       None => {
           if pi.scheduled {
               assert!(dst.module == src.module,
                       "Parent scheduled, in a different module, but no path set");
           } else {
               assert!(unresolved_dep == true);
           }
       }
   }
   return !unresolved_dep;
}

fn set_new_route(
    nw: &mut NetworkAvailability,
    src: &Coordinate,
    dst: &Coordinate,
    subroute: &NetworkRoute,
    pc: u32,
    pcfg: &PlatformConfig) -> NetworkRoute
{
    let start = subroute.front().unwrap().src;
    let end   = subroute.back().unwrap().dst;
    let src2start = NetworkPath::new(*src, start);
    let end2dst   = NetworkPath::new(end, *dst);

    let mut new_route = subroute.clone();
    if *src != start {
        new_route.push_front(src2start);
    }
    if *dst != end {
        new_route.push_back(end2dst);
    }

    let mut cur_route = NetworkRoute::new();
    for path in new_route.iter() {
        nw.oports.set_busy(path.src.id(pcfg), pc + pcfg.nw_route_dep_lat(&cur_route));
        cur_route.push_back(*path);
        nw.iports.set_busy(path.dst.id(pcfg), pc + pcfg.nw_route_lat(&cur_route));
    }
    return new_route;
}

fn nw_route_usable(
    nw: &mut NetworkAvailability,
    src: &Coordinate,
    dst: &Coordinate,
    subroute: &NetworkRoute,
    pc: u32,
    pcfg: &PlatformConfig
) -> bool {
    let mut usable = true;

    let start = subroute.front().unwrap().src;
    let end   = subroute.back().unwrap().dst;
    let src2start = NetworkPath::new(*src, start);
    let end2dst   = NetworkPath::new(end, *dst);

    let mut new_route = subroute.clone();
    if *src != start {
        new_route.push_front(src2start);
    }
    if *dst != end {
        new_route.push_back(end2dst);
    }

    let mut cur_route = NetworkRoute::new();
    for path in new_route.iter() {
        if nw.oports.is_busy(path.src.id(pcfg), pc + pcfg.nw_route_dep_lat(&cur_route)) {
            usable = false;
            break;
        }

        cur_route.push_back(*path);
        if nw.iports.is_busy(path.dst.id(pcfg), pc + pcfg.nw_route_lat(&cur_route)) {
            usable = false;
            break;
        }
    }
    return usable;
}

fn nw_available(
    nw: &mut NetworkAvailability,
    node:  &Box<dyn HWNode>,
    cnode: &Box<dyn HWNode>,
    inter_mod_routes: &mut IndexMap<u32, NetworkRoute>,
    pc: u32,
    pcfg: &PlatformConfig
) -> bool {
    let src = node.info().coord;
    let dst = cnode.info().coord;
    if dst == src {
        // same proc, don't have to check anything
        return true;
    } else if (dst.module == src.module) && (dst.proc != src.proc) {
        // same module
        if nw.iports.is_busy(dst.id(pcfg), pc + pcfg.nw_path_lat(&NetworkPath::new(src, dst))) ||
           nw.oports.is_busy(src.id(pcfg), pc) {
            return false;
        } else {
            return true;
        }
    } else if dst.module != src.module {
        // found already existing path to dst module
        if inter_mod_routes.contains_key(&dst.module) {
            let sub_route = inter_mod_routes.get(&dst.module).unwrap();
            if nw_route_usable(nw, &src, &dst, &sub_route, pc, pcfg) {
                return true;
            }
        }
        let paths = pcfg.topology.inter_mod_paths(src, dst);
        assert!(paths.len() > 0, "No inter module path from {:?} to {:?}", src, dst);

        // no path exists yet, search for a new path
        for p in paths.iter() {
            let route = NetworkRoute::from([*p]);
            if nw_route_usable(nw, &src, &dst, &route, pc, pcfg) {
                inter_mod_routes.insert(dst.module, route);
                return true;
            }
        }

        // no direct path exists between src & dst modules, go multi-hop
        let routes = pcfg.topology.inter_mod_routes(src, dst);
        for r in routes.iter() {
            if nw_route_usable(nw, &src, &dst, r, pc, pcfg) {
                inter_mod_routes.insert(dst.module, r.clone());
                return true;
            }
        }

        // no paths
        return false;
    } else {
        return true;
    }
}

fn schedule_candidates_at_pc(
    circuit: &mut Circuit,
    candidates: &BTreeSet<NodeIndexMobility>,
    coord_scheduled_by_pc: &mut IndexMap<u32, IndexSet<Coordinate>>,
    nw: &mut NetworkAvailability,
    pc: u32
) -> (Vec<NodeIndexMobility>, Vec<(EdgeIndex, NetworkRoute)>) {
    let pcfg = &circuit.platform_cfg;

    let mut nodes_scheduled: Vec<NodeIndexMobility> = vec![];
    let mut edges_scheduled: Vec<(EdgeIndex, NetworkRoute)> = vec![];

    if !coord_scheduled_by_pc.contains_key(&pc) {
        coord_scheduled_by_pc.insert(pc, IndexSet::new());
    }

    // Set of coordinates that are scheduled at pc
    let coord_scheduled = coord_scheduled_by_pc.get_mut(&pc).unwrap();

    for cand in candidates.iter() {
        let nidx = cand.index;
        let node = circuit.graph.node_weight(nidx).unwrap();

        // Node already scheduled as pc for this Coordinate
        if coord_scheduled.contains(&node.info().coord) {
            continue;
        }

        let mut unresolved_dep = false;
        let parent_edges = circuit.graph.edges_directed(nidx, Incoming);
        if node.is() != Primitives::Input &&
           node.is() != Primitives::Latch &&
           node.is() != Primitives::Gate {
            for pedge in parent_edges {
                let pnode = circuit.graph.node_weight(pedge.source()).unwrap();
                if !dependency_resolved(node, pnode, pedge, pcfg, pc) {
                    unresolved_dep = true;
                    break;
                }
            }
        }

        if unresolved_dep {
            continue;
        }

        let mut schedulable = true;
        let mut inter_mod_routes: IndexMap<u32, NetworkRoute> = IndexMap::new();

        let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
        for cidx in childs {
            let child = circuit.graph.node_weight(cidx).unwrap();
            if !nw_available(nw, node, child, &mut inter_mod_routes, pc, pcfg) {
                schedulable = false;
                break;
            }
        }

        if schedulable {
            let src = node.info().coord;
            let child_edges = circuit.graph.edges_directed(nidx, Outgoing);
            for cedge in child_edges {
                let cnode = circuit.graph.node_weight(cedge.target()).unwrap();
                let dst = cnode.info().coord;
                if dst.module != src.module {
                    let subroute = inter_mod_routes.get(&dst.module).unwrap();
                    let new_route = set_new_route(nw, &src, &dst, subroute, pc, pcfg);
                    edges_scheduled.push((cedge.id(), new_route));
                } else if dst.proc != src.proc {
                    nw.iports.set_busy(dst.id(pcfg), pc + pcfg.nw_path_lat(&NetworkPath::new(src, dst)));
                    nw.oports.set_busy(src.id(pcfg), pc);
                }
            }
            nodes_scheduled.push(*cand);
            coord_scheduled.insert(node.info().coord);
        }
    }
    return (nodes_scheduled, edges_scheduled);
}

/// Implements the modified list scheduling algorithm.
/// For each node, identify the ASAP & ALAP ranks.
/// Nodes with ASAP == ALAP are critical nodes.
/// For each round of scheduling, schedule the critical nodes & nodes with
/// mobility (rank - ASAP) == 0 first. We can increment the PC while doing so.
/// Then, for the PC range, slot in the nodes with mobility != 0 as much as
/// possible.
fn schedule_instructions_3(circuit: &mut Circuit) {
    let mut cpn: IndexSet<NodeIndex> = IndexSet::new();
    for nidx in circuit.graph.node_indices() {
        let rank = circuit.graph.node_weight(nidx).unwrap().info().rank;
        if rank.asap == rank.alap {
            cpn.insert(nidx);
        }
    }

    let max_rank = circuit.emul.max_rank;
    let mut pc_min = 0;
    let mut pc = 0;
    let mut nw = NetworkAvailability::new(circuit.platform_cfg.total_procs());
    let mut scheduled_map = circuit.graph.visit_map();

    let mut must_schedule_data: Vec<u32> = vec![];
    let mut be_schedule_data:   Vec<u32> = vec![];
    let mut nw_util_data:       Vec<u32> = vec![];

    for cur_rank in 0..(max_rank + 1) {
        println!("============================");
        println!("Current rank to schedule: {}", cur_rank);
        println!("============================");

        let mut must_schedule_candidates:        BTreeSet<NodeIndexMobility> = BTreeSet::new();
        let mut best_effort_schedule_candidates: BTreeSet<NodeIndexMobility> = BTreeSet::new();

        let mut debug_scheduled_nodes: Vec<NodeIndex> = vec![];
        let mut per_pc_scheduled: Vec<u32> = vec![];

        // Search for all the nodes to schedule in this round
        for nidx in circuit.graph.node_indices() {
            let node = circuit.graph.node_weight_mut(nidx).unwrap();
            let rank = node.info().rank;
            if rank.asap <= cur_rank && cur_rank <= rank.alap && !node.info().scheduled {
                let mob = rank.alap - cur_rank;
                node.set_info(NodeInfo {
                    rank: RankInfo {
                        mob: mob,
                        ..rank
                    },
                    ..node.info()
                });
                if cpn.contains(&nidx) || rank.alap - cur_rank == 0 {
                    must_schedule_candidates.insert(NodeIndexMobility::new(nidx, mob));
                } else {
                    best_effort_schedule_candidates.insert(NodeIndexMobility::new(nidx, mob));
                }
            }
        }

        pc_min = pc;

        // Schedule the nodes that must be scheduled in the current rank
        // (i.e. nodes with mobility 0). Increment the PC until all the nodes
        // are scheduled.
        let mut coord_scheduled_by_pc: IndexMap<u32, IndexSet<Coordinate>> = IndexMap::new();
        while !must_schedule_candidates.is_empty() {
            let (nodes_scheduled, edges_scheduled) = schedule_candidates_at_pc(
                circuit,
                &must_schedule_candidates,
                &mut coord_scheduled_by_pc,
                &mut nw,
                pc);

            per_pc_scheduled.push(nodes_scheduled.len() as u32);

            println!("pc: {} successful must scheduled: {}", pc, nodes_scheduled.len());

            for nm in nodes_scheduled.iter() {
                let nidx = nm.index;
                let node = circuit.graph.node_weight_mut(nidx).unwrap();
                node.set_info(NodeInfo {
                    pc: pc,
                    scheduled: true,
                    ..node.info()
                });
                assert_eq!(must_schedule_candidates.remove(nm), true);
                scheduled_map.visit(nidx);
                debug_scheduled_nodes.push(nidx);
            }

            for (eidx, path) in edges_scheduled.iter() {
                let edge = circuit.graph.edge_weight_mut(*eidx).unwrap();
                edge.set_routing(path.clone());
            }

            must_schedule_data.push(nodes_scheduled.len() as u32);

            pc += 1;
        }

        // For the PC ranging set by scheduling the "must schedule" nodes,
        // try to slot in as much nodes as possible. If scheduling is unsucessful,
        // punt to the next round of scheduling.
        for try_pc in pc_min..pc {
            let (nodes_scheduled, edges_scheduled) = schedule_candidates_at_pc(
                circuit,
                &best_effort_schedule_candidates,
                &mut coord_scheduled_by_pc,
                &mut nw,
                try_pc);

            let idx = (try_pc - pc_min) as usize;
            per_pc_scheduled[idx] += nodes_scheduled.len() as u32;
            println!("pc: {} successful best effort {}", try_pc, nodes_scheduled.len());

            for nm in nodes_scheduled.iter() {
                let nidx = nm.index;
                let node = circuit.graph.node_weight_mut(nidx).unwrap();
                node.set_info(NodeInfo {
                    pc: try_pc,
                    scheduled: true,
                    ..node.info()
                });
                best_effort_schedule_candidates.remove(nm);
                scheduled_map.visit(nidx);
                debug_scheduled_nodes.push(nidx);
            }
            for (eidx, path) in edges_scheduled.iter() {
                let edge = circuit.graph.edge_weight_mut(*eidx).unwrap();
                edge.set_routing(path.clone());
            }

            be_schedule_data.push(nodes_scheduled.len() as u32);
            nw_util_data.push(nw.iports.cnt_busy(try_pc));
        }

        print_tail_graph(circuit, &per_pc_scheduled, &debug_scheduled_nodes, pc_min, cur_rank);

        // TODO: consider global networking lat
        assert!(pc + 1 + circuit.platform_cfg.pc_sdm_offset() < circuit.platform_cfg.max_steps,
                "Schedule failed {} nodes out of {} nodes scheduled, pc {} max_steps {}",
                scheduled_map.count_ones(..),
                scheduled_map.len(),
                pc,
                circuit.platform_cfg.max_steps);
    }
    // TODO: consider global networking lat
    circuit.emul.host_steps = pc + 1 + circuit.platform_cfg.pc_sdm_offset();

    let total_steps = circuit.emul.host_steps * circuit.emul.used_mods * circuit.platform_cfg.num_procs;
    println!("Machine ({} / {}) = {:.2} %, host_steps = {}",
          circuit.graph.node_count(),
          total_steps,
          circuit.graph.node_count() as f32 / total_steps as f32 * 100f32,
          circuit.emul.host_steps);

    print_scheduling_stats(circuit, must_schedule_data, be_schedule_data, nw_util_data);

    assert!(scheduled_map.count_ones(..) == scheduled_map.len(), "{} out of {} scheduled",
        scheduled_map.count_ones(..), scheduled_map.len());
}
