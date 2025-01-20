use crate::common::{
    circuit::Circuit,
    primitive::*,
    network::*,
    hwgraph::*,
    config::*,
    utils::save_graph_pdf
};
use full_palette::{ORANGE, RED};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use petgraph::{
    graph::{EdgeReference, NodeIndex},
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing}
};
use fixedbitset::FixedBitSet;
use plotters::prelude::*;
use std::collections::BTreeSet;
use std::cmp::Ordering;
use std::cmp::max;
use std::fmt::Debug;


#[derive(Default, Clone)]
struct ScheduleStats {
    coord: u32,
    inputs: u32,
    network: u32,
    inputs_proc_internal: u32,
    inputs_local_nw: u32,
    inputs_global_nw: u32,
    inputs_unsched: u32
}

impl Debug for ScheduleStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total = self.coord + self.inputs + self.network;

        let coord_ratio   = self.coord   as f32 / total as f32 * 100.0;
        let inputs_ratio  = self.inputs  as f32 / total as f32 * 100.0;
        let network_ratio = self.network as f32 / total as f32 * 100.0;

        writeln!(f, "coord: {} ({} %) inputs: {} ({} %) network: {} ({} %)",
            self.coord, coord_ratio,
            self.inputs, inputs_ratio,
            self.network, network_ratio)?;

        let inputs_local_nw_ratio  = self.inputs_local_nw  as f32 / total as f32 * 100.0;
        let inputs_global_nw_ratio = self.inputs_global_nw as f32 / total as f32 * 100.0;
        let inputs_proc_internal_ratio = self.inputs_proc_internal as f32 / total as f32 * 100.0;
        let inputs_unsched_ratio   = self.inputs_unsched   as f32 / total as f32 * 100.0;
        writeln!(f, "- inputs internal: {} ({} %) local nw: {} ({} %) global nw: {} ({} %) unsched: {} ({} %)",
            self.inputs_proc_internal,
            inputs_proc_internal_ratio,
            self.inputs_local_nw,
            inputs_local_nw_ratio,
            self.inputs_global_nw,
            inputs_global_nw_ratio,
            self.inputs_unsched,
            inputs_unsched_ratio)
    }
}



/// Bitmap containing information about network port activity
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

    /// Port `idx` is busy at step `pc`
    fn is_busy(self: &mut Self, idx: u32, pc: u32) -> bool {
        self.add_pc_if_empty(pc);
        return self.busy.get(&pc).unwrap().contains(idx as usize);
    }

    /// Set port `idx` to busy at step `pc`
    fn set_busy(self: &mut Self, idx: u32, pc: u32) {
        self.add_pc_if_empty(pc);
        self.busy.get_mut(&pc).unwrap().set(idx as usize, true);
    }

    /// Number of busy ports at step `pc`
    fn cnt_busy(self: &mut Self, pc: u32) -> u32 {
        self.add_pc_if_empty(pc);
        return self.busy.get(&pc).unwrap().count_ones(..) as u32;
    }
}

#[derive(Debug, Default, Clone)]
struct NetworkAvailability {
    /// Network availability of ports going into each processor
    iports: NetworkPorts,

    /// Network availability of ports comming out from each processor
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

/// `NodeIndex` tagged with mobility `mob`. Used to sort the `NodeIndex`
/// w.r.t to mobility during scheduling
#[derive(Debug, Default, Clone, Eq, PartialEq, Copy)]
struct SchedCandidate {
    /// NodeIndex of the candidate node
    index: NodeIndex,

    /// mobility (ALAP - ASAP)
    mob: u32,

    /// fanout of this node
    odeg: u32,
}

impl SchedCandidate {
    fn new(index: NodeIndex, mob: u32, odeg: u32) -> Self {
        SchedCandidate {
            index: index,
            mob: mob,
            odeg: odeg
        }
    }
}

impl Ord for SchedCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.mob.cmp(&other.mob)
            .then_with(|| other.odeg.cmp(&self.odeg))
            .then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for SchedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Given a `Vec` containing the number of scheduled nodes per pc,
/// if the number is small for a long time which represents a pathological
/// scheduling scenario, print the nodes that are scheduled during that period
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

/// Print the number of nodes as well as network utilization until all
/// nodes are scheduled
fn print_scheduling_stats(
    circuit: &Circuit,
    must_scheduled_data: Vec<u32>,
    be_scheduled_data: Vec<u32>,
    ex_scheduled_data: Vec<u32>,
    nw_utilization: Vec<u32>)
{
    let title = format!("{}/scheduling-progress.png", circuit.compiler_cfg.output_dir);
    let root = BitMapBackend::new(&title, (2560, 1920)).into_drawing_area();
    let _ = root.fill(&WHITE);

    let mut chart = ChartBuilder::on(&root)
        .caption("Scheduling Progress", ("sans-serif", 50).into_font())
        .margin(30)
        .x_label_area_size(200)
        .y_label_area_size(200)
        .build_cartesian_2d(0f32..circuit.emul.host_steps as f32,
                            0f32..circuit.platform_cfg.total_procs() as f32).unwrap();

    let _ = chart
        .configure_mesh()
        .x_label_style(("sans-serif", 60))
        .y_label_style(("sans-serif", 60))
        .draw();

    chart
        .draw_series(LineSeries::new(
            (0..).zip(must_scheduled_data.iter()).map(|(a, b)| (a as f32, *b as f32)),
            RED.stroke_width(3)
        )).unwrap()
        .label("must-scheduled".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 10, y - 10), (x + 10, y + 10)], RED.filled())
        });

    chart
        .draw_series(LineSeries::new(
            (0..).zip(be_scheduled_data.iter()).map(|(a, b)| (a as f32, *b as f32)),
            BLUE.stroke_width(3)
        )).unwrap()
        .label("be-scheduled".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 10, y - 10), (x + 10, y + 10)], BLUE.filled())
        });

    chart
        .draw_series(LineSeries::new(
            (0..).zip(ex_scheduled_data.iter()).map(|(a, b)| (a as f32, *b as f32)),
            ORANGE.stroke_width(3)
        )).unwrap()
        .label("ex-scheduled".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 10, y - 10), (x + 10, y + 10)], ORANGE.filled())
        });

    chart
        .draw_series(LineSeries::new(
            (0..).zip(nw_utilization.iter()).map(|(a, b)| (a as f32, *b as f32)),
            GREEN.stroke_width(3)
        )).unwrap()
        .label("nw-utilization".to_string())
        .legend(move |(x, y)| {
            Rectangle::new([(x - 10, y - 10), (x + 10, y + 10)], GREEN.filled())
        });

    let _ = chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(BLACK.stroke_width(4))
        .label_font(("sans-serif", 60))
        .position(SeriesLabelPosition::UpperRight)
        .margin(20)
        .legend_area_size(40)
        .draw();
    let _ = root.present();
}

/// Input bit arrived & usable from a particular parent node
fn input_arrived(
    circuit: &Circuit,
    edge: EdgeReference<HWEdge, u32>,
    pc: &u32,
    stats: &mut ScheduleStats
) -> bool {
    let mut unresolved_dep = false;
    let pcfg = &circuit.platform_cfg;
    let parent = circuit.graph.node_weight(edge.source()).unwrap();

    // check for deps with parents
   match &edge.weight().route {
       Some(route) => {
           if parent.info().pc + pcfg.nw_route_dep_lat(&route) > *pc {
               match pcfg.nw_route_type(route) {
                   PathTypes::ProcessorInternal => {
                       stats.inputs_proc_internal += 1;
                   }
                   PathTypes::InterProcessor => {
                       stats.inputs_local_nw += 1;
                   }
                   PathTypes::InterModule => {
                       stats.inputs_global_nw += 1;
                   }
               }
               unresolved_dep = true;
           }
       }
       None => {
           assert_eq!(parent.info().scheduled, false,
               "{:?} scheduled w/o NetworkRoute set", parent.info());
          unresolved_dep = true;
          stats.inputs_unsched += 1;
       }
   }
   return !unresolved_dep;
}

/// All input bits arrived & usable from parent nodes
fn all_inputs_arrived(circuit: &Circuit, nidx: &NodeIndex, pc: &u32, stats: &mut ScheduleStats) -> bool {
    let mut arrived = true;
    let node = circuit.graph.node_weight(*nidx).unwrap();
    let parent_edges = circuit.graph.edges_directed(*nidx, Incoming);
    if node.is() != Primitive::Input    &&
       node.is() != Primitive::ConstLut &&
       node.is() != Primitive::Latch    &&
       node.is() != Primitive::Gate {
        for pedge in parent_edges {
            if !input_arrived(circuit, pedge, pc, stats) {
                arrived = false;
                break;
            }
        }
    }
    return arrived;
}

fn child_ff_scheduled(circuit: &Circuit, nidx: &NodeIndex) -> bool {
    let mut can_schedule = true;
    let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
    for cidx in childs {
        let cnode = circuit.graph.node_weight(cidx).unwrap();
        if cnode.is() == Primitive::Latch || cnode.is() == Primitive::Gate {
            if !cnode.info().scheduled {
                can_schedule = false;
                break;
            }
        }
    }
    return can_schedule;
}

/// When shipping a bit starting at `pc`, the `route` doesn't have any
/// contention
fn route_usable(
    nw: &mut NetworkAvailability,
    route: &NetworkRoute,
    pc: &u32,
    pcfg: &PlatformConfig
)-> bool {
    let mut usable = true;
    let mut cur_route = NetworkRoute::new();
    for (i, path) in route.iter().enumerate() {
        if i == 0 {
            if nw.oports.is_busy(path.src.id(pcfg), *pc + pcfg.nw_route_lat(&cur_route)) {
                usable = false;
                break;
            }
        } else {
            if nw.oports.is_busy(path.src.id(pcfg), *pc + pcfg.nw_route_dep_lat(&cur_route)) {
                usable = false;
                break;
            }
        }

        cur_route.push_back(*path);

        if nw.iports.is_busy(path.dst.id(pcfg), pc + pcfg.nw_route_lat(&cur_route)) {
            usable = false;
            break;
        }
    }
    return usable;
}

fn route_add_front(src: &Coordinate, route: &NetworkRoute) -> NetworkRoute {
    assert!(route.len() > 0);
    check_route(route, "route_add_front route");
    let mut new_route = route.clone();
    let start = new_route.front().unwrap().src;
    if *src != start {
        new_route.push_front(NetworkPath::new(*src, start));
    }
    check_route(&new_route, "route_add_front new_route");
    return new_route;
}

fn route_add_back(dst: &Coordinate, route: &NetworkRoute) -> NetworkRoute {
    assert!(route.len() > 0);
    check_route(route, "route_add_back route");
    let mut new_route = route.clone();
    let end = route.back().unwrap().dst;
    if *dst != end {
        new_route.push_back(NetworkPath::new(end, *dst));
    }
    check_route(&new_route, "route_add_back new_route");
    return new_route;
}

/// There is some path from node `nidx` to `cidx` that is not busy
/// - `nidx` and `cidx` are placed in the same processor
/// - `nidx` and `cidx` are placed in the same module
/// - `nidx` and `cidx` are placed in different modules
///     - Check for routes that are resuable from shipping bits to other child nodes
///       (`inter_mod_routes`)
///     - Search for direct inter-module path
///     - Search for one-hop inter-module path
fn child_reachable(
    circuit: &Circuit,
    nidx: &NodeIndex,
    cidx: &NodeIndex,
    nw: &mut NetworkAvailability,
    pc: &u32,
    inter_mod_routes: &mut IndexMap<u32, Vec<NetworkRoute>>
) -> Option<NetworkRoute> {
    let node = circuit.graph.node_weight(*nidx).unwrap();
    let cnode = circuit.graph.node_weight(*cidx).unwrap();
    let pcfg = &circuit.platform_cfg;

    let src =  node.info().coord;
    let dst = cnode.info().coord;

    if dst == src {
        // same proc, don't have to check anything
        return Some(NetworkRoute::from([NetworkPath::new(src, dst)]));
    } else if (dst.module == src.module) && (dst.proc != src.proc) {
        // same module
        let route = NetworkRoute::from([NetworkPath::new(src, dst)]);
        if !route_usable(nw, &route, pc, &pcfg) {
            return None;
        } else {
            return Some(route);
        }
    } else {
        assert!(dst.module != src.module);

        // found already existing inter-module path to dst module
        if inter_mod_routes.contains_key(&dst.module) {
            let routes = inter_mod_routes.get(&dst.module).unwrap();
            for route in routes {
                let new_route = route_add_back(&dst, &route_add_front(&src, route));
                if route_usable(nw, &new_route, pc, &pcfg) {
                    return Some(new_route);
                }
            }
        }

        // no path exists yet, search for a new path
        let paths = pcfg.topology.inter_mod_paths(src, dst);
        assert!(paths.len() > 0, "No inter module path from {:?} to {:?}", src, dst);
        for p in paths.iter() {
            let route = NetworkRoute::from([*p]);
            let new_route = route_add_back(&dst, &route_add_front(&src, &route));
            if route_usable(nw, &new_route, pc, &pcfg) {
                if !inter_mod_routes.contains_key(&dst.module) {
                    inter_mod_routes.insert(dst.module, vec![]);
                }
                inter_mod_routes.get_mut(&dst.module).unwrap().push(route);
                return Some(new_route);
            }
        }

        // no direct path exists between src & dst modules, go multi-hop
        let routes = pcfg.topology.inter_mod_routes(src, dst);
        for route in routes.iter() {
            let new_route = route_add_back(&dst, &route_add_front(&src, route));
            if route_usable(nw, &new_route, pc, &pcfg) {
                if !inter_mod_routes.contains_key(&dst.module) {
                    inter_mod_routes.insert(dst.module, vec![]);
                }
                inter_mod_routes.get_mut(&dst.module).unwrap().push(route.clone());
                return Some(new_route);
            }
        }

        // no paths
        return None;
    }
}

fn coalesce_paths(
    routes: &IndexMap<NodeIndex, NetworkRoute>,
    pcfg: &PlatformConfig
) -> IndexMap<NodeIndex, NetworkRoute> {
    let mut ret: IndexMap<NodeIndex, NetworkRoute> = IndexMap::new();
    let mut visited: IndexMap<(Coordinate, u32), NetworkRoute> = IndexMap::new();

    for (nidx, route) in routes.iter() {
        let mut cur_route = NetworkRoute::new();
        let mut merge_idx = 0;
        let mut merge_lat = 0;
        let mut merge_coord = None;
        for (i, path) in route.iter().enumerate() {
            cur_route.push_back(*path);
            let lat = pcfg.nw_route_lat(&cur_route);
            if visited.contains_key(&(path.dst, lat)) {
                merge_coord = Some(path.dst);
                merge_idx = i;
                merge_lat = lat;
            }
        }

        match merge_coord {
            Some(c) => {
                let mut existing_route = visited.get(&(c, merge_lat)).unwrap().clone();
                for (i, path) in route.iter().enumerate() {
                    if i > merge_idx {
                        existing_route.push_back(*path);
                        let lat = pcfg.nw_route_lat(&existing_route);
                        visited.insert((path.dst, lat), existing_route.clone());
                    }
                }
                ret.insert(*nidx, existing_route);
            }
            None => {
                ret.insert(*nidx, route.clone());

                let mut cur_route = NetworkRoute::new();
                for path in route.iter() {
                    cur_route.push_back(*path);
                    let lat = pcfg.nw_route_lat(&cur_route);
                    visited.insert((path.dst, lat), cur_route.clone());
                }
            }
        }
    }
    return ret;
}

/// All child nodes are reachable from `nidx` without network contention
fn all_childs_reachable(
    circuit: &Circuit,
    nw: &mut NetworkAvailability,
    nidx: &NodeIndex,
    pc: &u32
) -> (bool, IndexMap<NodeIndex, NetworkRoute>) {
    let mut reachable = true;
    let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);

    let mut inter_mod_routes: IndexMap<u32, Vec<NetworkRoute>> = IndexMap::new();
    let mut child_routes: IndexMap<NodeIndex, NetworkRoute> = IndexMap::new();

    for cidx in childs {
        match child_reachable(circuit, nidx, &cidx, nw, pc, &mut inter_mod_routes) {
            Some(route) => {
                child_routes.insert(cidx, route);
            }
            None => {
                reachable = false;
                break;
            }
        }
    }

    let coalesced_paths = coalesce_paths(&child_routes, &circuit.platform_cfg);
    return (reachable, coalesced_paths);
}

fn overrides_ff_input(
    circuit: &Circuit,
    nidx: &NodeIndex,
    pc: &u32) -> bool {
    let mut overrides = false;

    let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
    for cidx in childs {
        let cnode = circuit.graph.node_weight(cidx).unwrap();
        let scheduled = cnode.info().scheduled;
        if (!scheduled ||
            (scheduled && *pc <= cnode.info().pc)) &&
           (cnode.is() == Primitive::Gate     ||
            cnode.is() == Primitive::Latch    ||
            cnode.is() == Primitive::Input    ||
            cnode.is() == Primitive::ConstLut ||
            cnode.is() == Primitive::SRAMRdData) {
               overrides = true;
        }
    }

    return overrides;
}

fn mark_nw_busy(
    nw: &mut NetworkAvailability,
    pc: &u32,
    route: &NetworkRoute,
    pcfg: &PlatformConfig
) {
    let mut cur_route = NetworkRoute::new();
    for (i, path) in route.iter().enumerate() {
        if i == 0 {
            nw.oports.set_busy(path.src.id(pcfg), *pc + pcfg.nw_route_lat(&cur_route));
        } else {
            nw.oports.set_busy(path.src.id(pcfg), pc + pcfg.nw_route_dep_lat(&cur_route));
        }
        cur_route.push_back(*path);
        nw.iports.set_busy(path.dst.id(pcfg), pc + pcfg.nw_route_lat(&cur_route));
    }
}

fn schedule_candidates_at_pc(
    circuit: &mut Circuit,
    candidates: &mut BTreeSet<SchedCandidate>,
    scheduled_coordinates: &mut IndexSet<Coordinate>,
    nw: &mut NetworkAvailability,
    pc: &u32,
    stats: &mut ScheduleStats
) -> Vec<SchedCandidate> {
    let pcfg = &circuit.platform_cfg;
    let mut remove_nodes: Vec<SchedCandidate> = vec![];
    for cand in candidates.iter() {
        let node = circuit.graph.node_weight(cand.index).unwrap();

        // Cannot schedule a SRAM read until pc >= pcfg.sram_rd_en_step
        if node.is() == Primitive::SRAMRdData && *pc < pcfg.sram_rd_en_step() {
            continue;
        }

        // Cannot schedule a `Input` or a `SRAMRdData` if it is directly
        // connected to a `Latch`/`Gate` until the `Latch`/`Gate` is scheduled
        if (node.is() == Primitive::SRAMRdData || node.is() == Primitive::Input) &&
            !child_ff_scheduled(circuit, &cand.index) {
            continue;
        }

        // Node already scheduled at pc for this Coordinate
        if scheduled_coordinates.contains(&node.info().coord) {
            stats.coord += 1;
            continue;
        }

        // Check if inputs to the node are ready
        if !all_inputs_arrived(circuit, &cand.index, pc, stats) {
            stats.inputs += 1;
            continue;
        }

        // Check if routes to child nodes are ready
        let (reachable, routes) = all_childs_reachable(circuit, nw, &cand.index, pc);
        if !reachable {
            stats.network += 1;
            continue;
        }

        // Check if the produced bit doesn't override a unscheduled FF input
        if overrides_ff_input(circuit, &cand.index, pc) {
            continue;
        }

        // Node is schedulable
        scheduled_coordinates.insert(node.info().coord);
        remove_nodes.push(*cand);

        let node = circuit.graph.node_weight_mut(cand.index).unwrap();
        let info = node.info_mut();
        info.pc = *pc;
        info.scheduled = true;

        let edge_indices: Vec<_> = circuit.graph
            .edges_directed(cand.index, Outgoing)
            .map(|e| (e.id(), e.target()))
            .collect();

        for (eidx, cidx) in edge_indices {
            let edge = circuit.graph.edge_weight_mut(eidx).unwrap();
            let route = routes.get(&cidx).unwrap();
            edge.set_routing(route.clone());
        }

        let pcfg = &circuit.platform_cfg;
        for (_, route) in routes.iter() {
            mark_nw_busy(nw, pc, route, &pcfg);
        }
    }

    for rm in remove_nodes.iter() {
        candidates.remove(rm);
    }
    return remove_nodes;
}

pub fn schedule_instructions(circuit: &mut Circuit) {
    schedule_instructions_internal(circuit);
    check_schedule(circuit);
}

/// Implements the modified list scheduling algorithm.
/// For each node, identify the ASAP & ALAP ranks.
/// Nodes with ASAP == ALAP are critical nodes.
/// For each round of scheduling, schedule the critical nodes & nodes with
/// mobility (rank - ASAP) == 0 first. We can increment the PC while doing so.
/// Then, for the PC range, slot in the nodes with mobility != 0 as much as
/// possible.
fn schedule_instructions_internal(circuit: &mut Circuit) {
    let mut cpn: IndexSet<NodeIndex> = IndexSet::new();
    for nidx in circuit.graph.node_indices() {
        let rank = &circuit.graph.node_weight(nidx).unwrap().info().rank;
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
    let mut ex_schedule_data:   Vec<u32> = vec![];
    let mut nw_util_data:       Vec<u32> = vec![];

    let mut must_schedule_stats = ScheduleStats::default();
    let mut be_schedule_stats   = ScheduleStats::default();
    let mut ex_schedule_stats   = ScheduleStats::default();

    for cur_rank in 0..(max_rank + 1) {
        println!("============================");
        println!("Current rank to schedule: {}", cur_rank);
        println!("============================");

        let mut must_schedule_candidates:        BTreeSet<SchedCandidate> = BTreeSet::new();
        let mut best_effort_schedule_candidates: BTreeSet<SchedCandidate> = BTreeSet::new();
        let mut extra_effort_schedule_candidates: BTreeSet<SchedCandidate> = BTreeSet::new();

        let mut debug_scheduled_nodes: Vec<NodeIndex> = vec![];
        let mut per_pc_scheduled: Vec<u32> = vec![];

        // Search for all the nodes to schedule in this round
        for nidx in circuit.graph.node_indices() {
            let odeg = circuit.graph.neighbors_directed(nidx, Outgoing).count() as u32;
            let node = circuit.graph.node_weight_mut(nidx).unwrap();
            let info = node.info_mut();
            if cur_rank <= info.rank.alap && !info.scheduled {
                let mob = info.rank.alap - cur_rank;
                info.rank = RankInfo { mob: mob, ..info.rank };
                if info.rank.asap <= cur_rank && (cpn.contains(&nidx) || mob == 0) {
                    must_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                } else if info.rank.asap <= cur_rank {
                    best_effort_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                } else {
                    extra_effort_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                }
            }
        }

        pc_min = pc;

        // Schedule the nodes that must be scheduled in the current rank
        // (i.e. nodes with mobility 0). Increment the PC until all the nodes
        // are scheduled.
        let mut scheduled_coordinates_by_pc: IndexMap<u32, IndexSet<Coordinate>> = IndexMap::new();
        while !must_schedule_candidates.is_empty() {
            if !scheduled_coordinates_by_pc.contains_key(&pc) {
                scheduled_coordinates_by_pc.insert(pc, IndexSet::new());
            }
            let mut scheduled_coordinates = scheduled_coordinates_by_pc.get_mut(&pc).unwrap();
            let scheduled = schedule_candidates_at_pc(
                circuit,
                &mut must_schedule_candidates,
                &mut scheduled_coordinates,
                &mut nw,
                &pc,
                &mut must_schedule_stats);

            // For analysis
            println!("pc: {} successful must scheduled: {}", pc, scheduled.len());
            must_schedule_data.push(scheduled.len() as u32);
            per_pc_scheduled.push(scheduled.len() as u32);
            for s in scheduled {
                scheduled_map.visit(s.index);
                debug_scheduled_nodes.push(s.index);
            }

            pc += 1;
        }

        // For the PC ranging set by scheduling the "must schedule" nodes,
        // try to slot in as much nodes as possible. If scheduling is unsucessful,
        // punt to the next round of scheduling.
        for try_pc in pc_min..pc {
            let mut scheduled_coordinates = scheduled_coordinates_by_pc.get_mut(&try_pc).unwrap();
            let scheduled = schedule_candidates_at_pc(
                circuit,
                &mut best_effort_schedule_candidates,
                &mut scheduled_coordinates,
                &mut nw,
                &try_pc,
                &mut be_schedule_stats);

            // For analysis
            println!("pc: {} successful best effort scheduled: {}", try_pc, scheduled.len());
            be_schedule_data.push(scheduled.len() as u32);
            per_pc_scheduled[(try_pc - pc_min) as usize] += scheduled.len() as u32;
            for s in scheduled {
                scheduled_map.visit(s.index);
                debug_scheduled_nodes.push(s.index);
            }
        }

        for try_pc in pc_min..pc {
            let mut scheduled_coordinates = scheduled_coordinates_by_pc.get_mut(&try_pc).unwrap();
            let scheduled = schedule_candidates_at_pc(
                circuit,
                &mut extra_effort_schedule_candidates,
                &mut scheduled_coordinates,
                &mut nw,
                &try_pc,
                &mut ex_schedule_stats);

            // For analysis
            println!("pc: {} successful extra effort scheduled: {}", try_pc, scheduled.len());
            nw_util_data.push(nw.iports.cnt_busy(try_pc));
            ex_schedule_data.push(scheduled.len() as u32);
            per_pc_scheduled[(try_pc - pc_min) as usize] += scheduled.len() as u32;
            for s in scheduled {
                scheduled_map.visit(s.index);
                debug_scheduled_nodes.push(s.index);
            }
        }

        print_tail_graph(circuit, &per_pc_scheduled, &debug_scheduled_nodes, pc_min, cur_rank);

        assert!(pc < circuit.platform_cfg.max_steps,
                "Schedule failed {} nodes out of {} nodes scheduled, pc {} max_steps {}",
                scheduled_map.count_ones(..),
                scheduled_map.len(),
                pc,
                circuit.platform_cfg.max_steps);
    }

    let mut max_nw_route_dep_lat = 0;
    for eidx in circuit.graph.edge_indices() {
        match &circuit.graph.edge_weight(eidx) {
            Some(e) => {
                match &e.route {
                    Some(r) => {
                        max_nw_route_dep_lat = max(max_nw_route_dep_lat,
                                                   circuit.platform_cfg.nw_route_dep_lat(&r));
                    }
                    None => {
                        assert!(false, "Edge with unassigned NetworkRoute");
                    }
                }
            }
            None => {
                assert!(false, "Edge with unassigned NetworkRoute");
            }
        }
    }

    circuit.emul.host_steps = pc + 1 +                               // <base>
                              max(max_nw_route_dep_lat,              // NW
                                  circuit.platform_cfg.sram_ip_pl);  // SRAM

    let total_steps = circuit.emul.host_steps * circuit.platform_cfg.total_procs();
    println!("Machine ({} / {}) = {:.2} %, host_steps = {}",
          circuit.graph.node_count(),
          total_steps,
          circuit.graph.node_count() as f32 / total_steps as f32 * 100f32,
          circuit.emul.host_steps);

    assert!(scheduled_map.count_ones(..) == scheduled_map.len(),
        "{} out of {} scheduled",
        scheduled_map.count_ones(..), scheduled_map.len());

    print_scheduling_stats(circuit,
        must_schedule_data,
        be_schedule_data,
        ex_schedule_data,
        nw_util_data);

    println!("Must schedule failed reasons: {:?}",         must_schedule_stats);
    println!("Best effort schedule failed reasons: {:?}",  be_schedule_stats);
    println!("Extra effort schedule failed reasons: {:?}", ex_schedule_stats);
}

fn check_route(route: &NetworkRoute, msg: &str) {
    for (a, b) in route.iter().tuple_windows() {
        assert_eq!(a.dst, b.src, "{} Route is not connected {:?}", msg, route);
    }
}

fn check_schedule(circuit: &Circuit) {
    for eidx in circuit.graph.edge_indices() {
        let edge = circuit.graph.edge_weight(eidx).unwrap();
        match &edge.route {
            Some(route) => {
                check_route(route, "Final check");
            }
            None => {
            }
        }
    }
}
