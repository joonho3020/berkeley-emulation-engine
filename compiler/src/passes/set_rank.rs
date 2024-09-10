use crate::primitives::*;
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::{
    graph::NodeIndex, prelude::Dfs, visit::{VisitMap, Visitable}, Direction::{Incoming, Outgoing}, Undirected
};
use plotters::prelude::*;
use std::{cmp::{max, min}, collections::VecDeque};

fn set_rank_asap(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info();
    let new_rank = max(info.rank.asap, rank);
    node.set_info(NodeInfo {
        rank: RankInfo {
            asap: new_rank,
            ..node.info().rank
        },
        ..info
    })
}

fn set_rank_alap(graph: &mut HWGraph, nidx: NodeIndex, rank: u32) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info();
    node.set_info(NodeInfo {
        rank: RankInfo {
            alap: rank,
            ..node.info().rank
        },
        ..info
    })
}

pub fn find_rank_order(circuit: &mut Circuit) {
    find_asap_rank_order(circuit);
    find_alap_rank_order(circuit);
    print_rank_stats(circuit);
}

fn find_asap_rank_order(circuit: &mut Circuit) {
    let mut max_rank: u32 = 0;

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
    let mut visited = 0;
    let mut vis_map = circuit.graph.visit_map();
    for curidx in circuit.graph.node_indices() {
        if vis_map.is_visited(&curidx) {
            continue;
        }

        // Found new connected component
        // DFS to search for all the relevant nodes
        let mut ff_nodes: Vec<NodeIndex> = vec![];
        let mut in_nodes: Vec<NodeIndex> = vec![];

        let mut dfs = Dfs::new(&undir_graph, curidx);
        while let Some(nx) = dfs.next(&undir_graph) {
            vis_map.visit(nx);

            let node = circuit.graph.node_weight(nx).unwrap();
            match node.is() {
                Primitives::Latch | Primitives::Gate => {
                    ff_nodes.push(nx);
                }
                Primitives::Input => {
                    in_nodes.push(nx);
                }
                _ => {
                }
            }
        }

        // Start topological sort
        let mut q: VecDeque<NodeIndex> = VecDeque::new();
        for nidx in in_nodes.iter() {
            q.push_back(*nidx);
            set_rank_asap(&mut circuit.graph, *nidx, 0);
        }
        for nidx in ff_nodes.iter() {
            q.push_back(*nidx);
            set_rank_asap(&mut circuit.graph, *nidx, 0);
        }

        // BFS
        let mut topo_sort_order: Vec<NodeIndex> = vec![];
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
                if !topo_vis_map.is_visited(&cidx) &&
                    cnode.is() != Primitives::Gate &&
                    cnode.is() != Primitives::Latch &&
                    cnode.is() != Primitives::Input {
                    *indeg.get_mut(&cidx).unwrap() -= 1;
                    if *indeg.get(&cidx).unwrap() == 0 {
                        q.push_back(cidx);
                    }
                }
            }
        }

        // Set rank based on the topo sorted order
        for nidx in topo_sort_order.iter() {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            if node.is() != Primitives::Gate &&
               node.is() != Primitives::Latch &&
               node.is() != Primitives::Input {
                let mut max_parent_rank = 0;
                let parents = circuit.graph.neighbors_directed(*nidx, Incoming);
                for pidx in parents {
                    let parent = circuit.graph.node_weight(pidx).unwrap();
                    max_parent_rank = max(max_parent_rank, parent.info().rank.asap);
                }
                set_rank_asap(&mut circuit.graph, *nidx, max_parent_rank + 1);
                if max_parent_rank + 1 > max_rank {
                    max_rank = max_parent_rank + 1;
                }
            }
        }
        visited += topo_sort_order.len();
    }
    circuit.emul.max_rank = max_rank;

    println!("Max rank of this graph: {}", max_rank);
    assert!(
        visited == vis_map.len(),
        "Visited {} nodes out of {} nodes while topo sorting",
        visited,
        vis_map.len());
}

fn find_alap_rank_order(circuit: &mut Circuit) {
    let mut odeg: IndexMap<NodeIndex, u32> = IndexMap::new();
    let max_rank = circuit.emul.max_rank;

    for nidx in circuit.graph.node_indices() {
        odeg.insert(nidx, 0);
    }
    for eidx in circuit.graph.edge_indices() {
        let e = circuit.graph.edge_endpoints(eidx).unwrap();
        let src = e.0;
        *odeg.get_mut(&src).unwrap() += 1;
    }

    let undir_graph = circuit.graph.clone().into_edge_type::<Undirected>();
    let mut visited = 0;
    let mut vis_map = circuit.graph.visit_map();
    for curidx in circuit.graph.node_indices() {
        if vis_map.is_visited(&curidx) {
            continue;
        }

        let mut q: VecDeque<NodeIndex> = VecDeque::new();
        let mut dfs = Dfs::new(&undir_graph, curidx);
        while let Some(nx) = dfs.next(&undir_graph) {
            vis_map.visit(nx);

            let node = circuit.graph.node_weight(nx).unwrap();
            match node.is() {
                Primitives::Latch | Primitives::Gate => {
                    q.push_back(nx);
                    set_rank_alap(&mut circuit.graph, nx, 0);
                }
                Primitives::Output => {
                    q.push_back(nx);
                    set_rank_alap(&mut circuit.graph, nx, max_rank);
                }
                _ => {
                }
            }
        }

        // BFS
        let mut topo_sort_order: Vec<NodeIndex> = vec![];
        let mut topo_vis_map = circuit.graph.visit_map();
        while !q.is_empty() {
            let nidx = q.pop_front().unwrap();
            if topo_vis_map.is_visited(&nidx) {
                continue;
            }
            topo_vis_map.visit(nidx);
            topo_sort_order.push(nidx);

            let parents = circuit.graph.neighbors_directed(nidx, Incoming);
            for pidx in parents {
                let pnode = circuit.graph.node_weight(pidx).unwrap();
                if !topo_vis_map.is_visited(&pidx) &&
                   pnode.is() != Primitives::Gate  ||
                   pnode.is() != Primitives::Latch ||
                   pnode.is() != Primitives::Input {
                   *odeg.get_mut(&pidx).unwrap() -= 1;
                    if *odeg.get(&pidx).unwrap() == 0 {
                        q.push_back(pidx);
                    }
                }
            }
        }

        // Set rank based on the topo sorted order
        for nidx in topo_sort_order.iter() {
            let node = circuit.graph.node_weight(*nidx).unwrap();
            if node.is() != Primitives::Gate &&
               node.is() != Primitives::Latch &&
               node.is() != Primitives::Input {
                let mut min_child_rank = circuit.emul.max_rank + 1;
                let childs = circuit.graph.neighbors_directed(*nidx, Outgoing);
                for cidx in childs {
                    let child = circuit.graph.node_weight(cidx).unwrap();
                    if child.is() == Primitives::Gate ||
                       child.is() == Primitives::Latch {
                        min_child_rank = min(min_child_rank, circuit.emul.max_rank + 1);
                    } else {
                        min_child_rank = min(min_child_rank, child.info().rank.alap);
                    };
                }
                set_rank_alap(&mut circuit.graph, *nidx, min_child_rank - 1);
            }
        }
        visited += topo_sort_order.len();
    }
    assert!(
        visited == vis_map.len(),
        "Visited {} nodes out of {} nodes while topo sorting",
        visited,
        vis_map.len());
}

fn print_dist(name: &str, dist_map: &IndexMap<u32, u32>) {
    let dist_vec = dist_map.values().map(|x| *x as f64).collect_vec();
    let dist_plot = lowcharts::plot::XyPlot::new(dist_vec.as_slice(),  80, 30, None);
    println!("================  {}  ===============", name);
    println!("{}", dist_plot);
}

fn print_stacked_bar_chart(data: &IndexMap<u32, IndexMap<u32, u32>>, circuit: &Circuit) {
    let max_height: u32 = data.values().map(|imap| imap.values().sum()).max().unwrap();

    let title = format!("{}/rank-indeg-distribution.png", circuit.compiler_cfg.output_dir);
    let root = BitMapBackend::new(&title, (2560, 1920)).into_drawing_area();
    let _ = root.fill(&WHITE);
    let mut chart = ChartBuilder::on(&root)
        .caption("Rank indegree distribution", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..circuit.platform_cfg.num_mods as f32,
                            0f32..max_height as f32).unwrap();
    let _ = chart.configure_mesh().draw();
    for (m, y_stack) in data.iter() {
        let mut cumul_y = 0;
        for (i, y) in y_stack.values().enumerate() {
            let _ = chart.draw_series(std::iter::once(
                Rectangle::new(
                    [
                        (*m       as f32, cumul_y        as f32),
                        ((*m + 1) as f32, (cumul_y + *y) as f32),
                    ],
                    Palette99::pick(i as usize).filled(),
                ),
            )).unwrap()
            .label(format!("rank-{}", i))
            .legend(move |(x, y)| {
                Rectangle::new([(x, y - 5), (x + 10, y + 5)], Palette99::pick(i as usize).filled())
            });
            cumul_y += *y;
        }
    }

    // Configure and position the series labels (legend)
    let _ = chart
        .configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.8))
        .position(SeriesLabelPosition::UpperRight)
        .draw();

    let _ = root.present();
}

fn print_rank_stats(circuit: &Circuit) {
    let mut asap_map: IndexMap<u32, u32> = IndexMap::new();
    let mut alap_map: IndexMap<u32, u32> = IndexMap::new();
    let mut mob_map:  IndexMap<u32, u32> = IndexMap::new();
    let mut cns = 0;
    let mut ff_cnt = 0;

    let mut per_rank_indeg: IndexMap<u32, IndexMap<u32, u32>> = IndexMap::new();
    for m in 0..circuit.platform_cfg.num_mods {
        per_rank_indeg.insert(m, IndexMap::new());
        for r in 0..(circuit.emul.max_rank + 1) {
            per_rank_indeg.get_mut(&m).unwrap().insert(r, 0);
        }
    }

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() == Primitives::Gate || node.is() == Primitives::Latch {
            ff_cnt += 1;
        }

        let asap = node.info().rank.asap;
        let alap = node.info().rank.alap;
        let mob = alap - asap;

        if asap == alap {
            cns += 1;
        }

        if !asap_map.contains_key(&asap) {
            asap_map.insert(asap, 0);
        }
        *asap_map.get_mut(&asap).unwrap() += 1;

        if !alap_map.contains_key(&alap) {
            alap_map.insert(alap, 0);
        }
        *alap_map.get_mut(&alap).unwrap() += 1;

        if !mob_map.contains_key(&mob) {
            mob_map.insert(mob, 0);
        }
        *mob_map.get_mut(&mob).unwrap() += 1;

        let parents = circuit.graph.neighbors_directed(nidx, Incoming);
        for pidx in parents {
            let par = circuit.graph.node_weight(pidx).unwrap();
            if node.info().coord.module != par.info().coord.module {
                *per_rank_indeg
                    .get_mut(&node.info().coord.module)
                    .unwrap()
                    .get_mut(&node.info().rank.asap)
                    .unwrap() += 1;
            }
        }
    }

    println!("Number of ff nodes: {} critical nodes: {} ({:.2} %), non-ff critical nodes: {} ({:.2} %) total nodes: {}",
             ff_cnt,
             cns,
             cns as f32 / circuit.graph.node_count() as f32 * 100f32,
             cns - ff_cnt,
             (cns - ff_cnt) as f32 / circuit.graph.node_count() as f32 * 100f32,
             circuit.graph.node_count());
    asap_map.sort_keys();
    alap_map.sort_keys();
    mob_map.sort_keys();
    print_dist("ASAP distribution", &asap_map);
    print_dist("ALAP distribution", &alap_map);
    print_dist("MOB distribution",  &mob_map);
    print_stacked_bar_chart(&per_rank_indeg, circuit);
}
