use crate::common::config::*;
use crate::common::mapping::*;
use crate::common::utils::*;
use crate::common::primitive::*;
use crate::common::hwgraph::*;
use crate::fsim::board::Board;
use crate::rtlsim::vcdparser::FourStateBit;
use indexmap::IndexMap;
use std::fmt::Debug;
use std::collections::VecDeque;
use petgraph::{
    graph::NodeIndex,
    Undirected,
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
    prelude::Dfs
};


#[derive(Default, Clone)]
pub struct Circuit {
    pub compiler_cfg: CompilerConfig,
    pub platform_cfg: PlatformConfig,
    pub kaminpar_cfg: KaMinParConfig,
    pub graph: HWGraph,
    pub emul:  EmulatorMapping
}

fn set_debug(graph: &mut HWGraph, nidx: NodeIndex, check: NodeCheckState) {
    let node = graph.node_weight_mut(nidx).unwrap();
    let info = node.info_mut();
    info.debug.check = check;
}

impl Circuit {
    pub fn set_cfg(&mut self, pcfg: PlatformConfig, ccfg: CompilerConfig) {
        self.platform_cfg = pcfg;
        self.compiler_cfg = ccfg;
    }

    pub fn save_emulator_info(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.info",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);
        for (i, mapping) in self.emul.module_mappings.iter() {
            let mut out = file_path.clone();
            out.push_str(&format!("-{}", i));
            write_string_to_file(serde_json::to_string_pretty(&mapping)?, &out)?;
        }
        Ok(())
    }

    pub fn save_emulator_instructions(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.insts",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);
        let mut inst_str = "".to_string();
        let total_insns = self.emul.host_steps * self.platform_cfg.total_procs();
        let mut total_nops = 0;
        for (i, mapping) in self.emul.module_mappings.iter() {
            inst_str.push_str(&format!("============ module {} ============\n", i));

            for (pi, pmap) in mapping.proc_mappings.iter() {
                inst_str.push_str(&format!("------------ processor {} ------------\n", pi));
                for (i, inst) in pmap.instructions.iter().enumerate() {
                    if (i as u32) < self.emul.host_steps {
                        inst_str.push_str(&format!("{} {:?}\n", i, inst));
                        match inst.opcode {
                            Opcode::NOP => total_nops += 1,
                            _ => ()
                        };
                    } else {
                        break;
                    }
                }
            }
        }
        inst_str.push_str(&format!("Overall stats\nNOPs: {}\nTotal insts: {}\nUtilization: {}%\n",
                                   total_nops,
                                   total_insns,
                                   ((total_insns - total_nops) as f32)/(total_insns as f32) * 100 as f32));
        write_string_to_file(inst_str, &file_path)?;
        Ok(())
    }

    pub fn save_emulator_sigmap(&self) -> std::io::Result<()> {
        let file_path = format!("{}/{}.signals",
                                self.compiler_cfg.output_dir,
                                self.compiler_cfg.top_module);

        let mut ret_str = "".to_string();
        for (_mi, mmap) in self.emul.module_mappings.iter() {
            for (_pi, pmap) in mmap.proc_mappings.iter() {
                ret_str.push_str(&format!("{:#?}", pmap.signal_map));
            }
        }
        write_string_to_file(ret_str, &file_path)?;
        Ok(())
    }

    /// # print_given_nodes
    /// - Given a `Vec<NodeIndex>` prints a subgraph containing those nodes
    /// and the edges between them
    pub fn print_given_nodes(
        &self,
        nodes: &Vec<NodeIndex>
    ) -> String {
        let mut outstring = "digraph {\n".to_string();
        let indent: &str = "    ";

        // print nodes
        for nidx in nodes.iter() {
            let node = self.graph.node_weight(*nidx).unwrap();
            let color = "purple";
            match &node.prim {
                CircuitPrimitive::Lut { inputs:_, output:_, table } => {
                    outstring.push_str(&format!(
                        "{}{} [ label = {:?} color = \"{}\"]\n",
                        indent,
                        nidx.index(),
                        format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nlut: {:?} val: {}",
                                node.name(),
                                node.is(),
                                node.info().coord.module,
                                node.info().coord.proc,
                                node.info().rank.asap,
                                node.info().rank.alap,
                                node.info().pc,
                                table,
                                node.info().debug.val),
                        color));
                }
                _ => {
                    outstring.push_str(&format!(
                        "{}{} [ label = {:?} color = \"{}\"]\n",
                        indent,
                        nidx.index(),
                        format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nval: {}",
                                node.name(),
                                node.is(),
                                node.info().coord.module,
                                node.info().coord.proc,
                                node.info().rank.asap,
                                node.info().rank.alap,
                                node.info().pc,
                                node.info().debug.val),
                        color));
                }
            }
        }

        // print edges
        for nidx in nodes.iter() {
            let node = self.graph.node_weight(*nidx).unwrap();

            let mut childs = self.graph.neighbors_directed(*nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&self.graph) {
                let cnode = self.graph.node_weight(cidx).unwrap();
                let mut op_idx = 0;
                match &cnode.prim {
                    CircuitPrimitive::Lut { inputs, .. } => {
                        let lut_inputs = inputs.to_vec();
                        op_idx = lut_inputs.iter().position(|n| n == node.name()).unwrap();
                    }
                    _ => { }
                }

                if nodes.contains(&cidx) {
                    outstring.push_str(&format!("{}{} {} {} ",
                        indent, nidx.index(), "->", cidx.index()));
                    outstring.push_str(&format!("[ label=\"{}\" ]", op_idx));
                }
            }
        }
        outstring.push_str("}");

        return outstring;
    }

    /// #debug_graph
    /// - Given a `dbg_node` in the graph and reference signal values `rs` from
    /// a VCD file print a debug graph.
    /// - Perform a BFS and compare each node to the `rs` signals. If it matches or
    /// if all of its parent node matches, we mark it as `NodeCheckState::Match`
    /// and print it in green.
    /// - Unknown nodes are purple and mismatched nodes are red.
    pub fn debug_graph(
        &mut self,
        dbg_node: NodeIndex,
        board: &Board,
        rs: &IndexMap<String, FourStateBit>
    ) -> String {
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight_mut(nidx).unwrap();
            if !rs.contains_key(node.name()) {
                continue;
            }
            let four_state_bit = rs.get(node.name()).unwrap();
            match (board.peek(node.name()), four_state_bit.to_bit()) {
                (Some(b), Some(rb)) => {
                    if b == rb {
                        node.info_mut().debug.check = NodeCheckState::Match;
                    } else {
                        node.info_mut().debug.check = NodeCheckState::Mismatch;
                    }
                }
                _ => { }
            }
        }

        // compute indeg for the entire graph
        let mut indeg: IndexMap<NodeIndex, u32> = IndexMap::new();
        for nidx in self.graph.node_indices() {
            indeg.insert(nidx, 0);
        }
        for eidx in self.graph.edge_indices() {
            let e = self.graph.edge_endpoints(eidx).unwrap();
            let dst = e.1;
            *indeg.get_mut(&dst).unwrap() += 1;
        }

        let undir_graph = self.graph.clone().into_edge_type::<Undirected>();
        let mut vis_map = self.graph.visit_map();
        for curidx in self.graph.node_indices() {
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

                let node = self.graph.node_weight(nx).unwrap();
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
            for nidx in in_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in ff_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in sr_nodes.iter() {
                q.push_back(*nidx);
            }

            // BFS
            let mut topo_sort_order: Vec<NodeIndex> = vec![];
            let mut topo_vis_map = self.graph.visit_map();
            while !q.is_empty() {
                let nidx = q.pop_front().unwrap();
                if topo_vis_map.is_visited(&nidx) {
                    continue;
                }

                topo_vis_map.visit(nidx);
                topo_sort_order.push(nidx);

                let childs = self.graph.neighbors_directed(nidx, Outgoing);
                for cidx in childs {
                    let cnode = self.graph.node_weight(cidx).unwrap();
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

            // Set rank based on the topo sorted order
            for nidx in topo_sort_order.iter() {
                let node = self.graph.node_weight(*nidx).unwrap();
                if node.info().debug.check != NodeCheckState::Unknown {
                    continue;
                }
                if node.is() != Primitive::Gate     &&
                   node.is() != Primitive::Latch    &&
                   node.is() != Primitive::Input    &&
                   node.is() != Primitive::ConstLut &&
                   node.is() != Primitive::SRAMRdData {
                    let mut found_mismatch = false;
                    let mut found_unknown = false;
                    let parents = self.graph.neighbors_directed(*nidx, Incoming);
                    for pidx in parents {
                        let parent = self.graph.node_weight(pidx).unwrap();
                        match parent.info().debug.check {
                            NodeCheckState::Unknown => {
                                found_unknown = true;
                                break;
                            }
                            NodeCheckState::Mismatch => {
                                found_mismatch = true;
                                break;
                            }
                            _ => { }
                        }
                    }
                    if !found_mismatch && !found_unknown {
                        set_debug(&mut self.graph, *nidx, NodeCheckState::Match);
                    }
                } else if node.is() == Primitive::ConstLut {
                    set_debug(&mut self.graph, *nidx, NodeCheckState::Match);
                }
            }
        }

        let mut vis_map = self.graph.visit_map();
        let mut q = vec![];
        q.push(dbg_node);
        let mut root = true;

        let mut node_cnt = 0;
        while !q.is_empty() {
            let nidx = q.remove(0);
            vis_map.visit(nidx);

            let node = self.graph.node_weight(nidx).unwrap();
            if node.info().debug.check == NodeCheckState::Match {
                if !root {
                    continue;
                } else {
                    root = false;
                }
            }

            node_cnt += 1;
            if node_cnt >= 300 {
                break;
            }

            let mut parents = self.graph.neighbors_directed(nidx, Incoming).detach();
            while let Some(pidx) = parents.next_node(&self.graph) {
                q.push(pidx);
            }
        }

        let mut outstring = "digraph {\n".to_string();
        let indent: &str = "    ";

        // print nodes
        for nidx in self.graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                let node = self.graph.node_weight(nidx).unwrap();
                let val = match board.peek(node.name()) {
                    Some(v) => v,
                    None    => Bit::MAX
                };
                let color = match node.info().debug.check {
                    NodeCheckState::Unknown  => { "purple" }
                    NodeCheckState::Mismatch => { "red"    }
                    NodeCheckState::Match    => { "green"  }
                };
                match &node.prim {
                    CircuitPrimitive::Lut { inputs:_, output:_, table } => {
                        outstring.push_str(&format!(
                            "{}{} [ label = {:?} color = \"{}\"]\n",
                            indent,
                            nidx.index(),
                            format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nlut: {:?} val: {}",
                                    node.name(),
                                    node.is(),
                                    node.info().coord.module,
                                    node.info().coord.proc,
                                    node.info().rank.asap,
                                    node.info().rank.alap,
                                    node.info().pc,
                                    table,
                                    val),
                            color));
                    }
                    _ => {
                        outstring.push_str(&format!(
                            "{}{} [ label = {:?} color = \"{}\"]\n",
                            indent,
                            nidx.index(),
                            format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nval: {}",
                                    node.name(),
                                    node.is(),
                                    node.info().coord.module,
                                    node.info().coord.proc,
                                    node.info().rank.asap,
                                    node.info().rank.alap,
                                    node.info().pc,
                                    val),
                            color));
                    }
                }
            }
        }

        // print edges
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight(nidx).unwrap();

            if vis_map.is_visited(&nidx) {
                let mut childs = self.graph.neighbors_directed(nidx, Outgoing).detach();
                while let Some(cidx) = childs.next_node(&self.graph) {
                    let cnode = self.graph.node_weight(cidx).unwrap();
                    let mut op_idx = 0;
                    match &cnode.prim {
                        CircuitPrimitive::Lut { inputs, .. } => {
                            let lut_inputs = inputs.to_vec();
                            op_idx = lut_inputs.iter().position(|n| n == node.name()).unwrap();
                        }
                        _ => { }
                    }

                    if vis_map.is_visited(&cidx) {
                        outstring.push_str(&format!("{}{} {} {} ",
                            indent, nidx.index(), "->", cidx.index()));
                        outstring.push_str(&format!("[ label=\"{}\" ]", op_idx));
                    }
                }
            }
        }
        outstring.push_str("}");

        return outstring;
    }

    /// #debug_graph_2
    /// - Given a `dbg_node` in the graph, search for all parents nodes up until
    /// it reaches Gate, Latch or Input.
    /// - It will also print the bit-value associated with the node
    /// computed by the emulation processor.
    pub fn debug_graph_2(&self, dbg_node: NodeIndex, board: &Board) -> String {
        let mut node_cnt = 0;
        let indent: &str = "    ";
        let mut vis_map = self.graph.visit_map();
        let mut q = vec![];
        q.push(dbg_node);
        let mut root = true;

        while !q.is_empty() {
            let nidx = q.remove(0);
            vis_map.visit(nidx);

            let node = self.graph.node_weight(nidx).unwrap();
            if node.is() == Primitive::Gate || node.is() == Primitive::Latch {
                if !root {
                    continue;
                } else {
                    root = false;
                }
            }
            node_cnt += 1;

            if node_cnt > 300 {
                break;
            }

            let mut parents = self.graph.neighbors_directed(nidx, Incoming).detach();
            while let Some(pidx) = parents.next_node(&self.graph) {
                q.push(pidx);
            }
        }

        let mut outstring = "digraph {\n".to_string();

        // print nodes
        for nidx in self.graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                let node = self.graph.node_weight(nidx).unwrap();
                let val = match board.peek(node.name()) {
                    Some(v) => v,
                    None    => Bit::MAX
                };
                match &node.prim {
                    CircuitPrimitive::Lut { inputs:_, output:_, table } => {
                        outstring.push_str(&format!(
                            "{}{} [ label = {:?} ]\n",
                            indent,
                            nidx.index(),
                            format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nlut: {:?} val: {}",
                                    node.name(),
                                    node.is(),
                                    node.info().coord.module,
                                    node.info().coord.proc,
                                    node.info().rank.asap,
                                    node.info().rank.alap,
                                    node.info().pc,
                                    table,
                                    val)));
                    }
                    _ => {
                        outstring.push_str(&format!(
                            "{}{} [ label = {:?} ]\n",
                            indent,
                            nidx.index(),
                            format!("{} {:?}\nmod: {} proc: {}\nasap: {} alap: {} pc: {}\nval: {}",
                                    node.name(),
                                    node.is(),
                                    node.info().coord.module,
                                    node.info().coord.proc,
                                    node.info().rank.asap,
                                    node.info().rank.alap,
                                    node.info().pc,
                                    val)));
                    }
                }
            }
        }

        // print edges
        for nidx in self.graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                let mut childs = self.graph.neighbors_directed(nidx, Outgoing).detach();
                while let Some(cidx) = childs.next_node(&self.graph) {
                    if vis_map.is_visited(&cidx) {
                        outstring.push_str(&format!(
                            "{}{} {} {} \n",
                            indent,
                            nidx.index(),
                            "->",
                            cidx.index()
                        ));
                    }
                }
            }
        }
        outstring.push_str("}");
        return outstring;
    }

    /// Returns a vector of `NodeIndex` for nodes of `nodetype`
    pub fn get_nodes_type(self: &Self, nodetype: Primitive) -> Vec<NodeIndex> {
        let mut nodes: Vec<NodeIndex> = vec![];
        for nidx in self.graph.node_indices() {
            let node = self.graph.node_weight(nidx).unwrap();
            if node.is() == nodetype {
                nodes.push(nidx);
            }
        }
        return nodes;
    }

    /// Save the graph in a pdf form
    /// ** Use only for small graphs **
    pub fn save_graph(self: &Self, pfx: &str) -> std::io::Result<()> {
        let ccfg = &self.compiler_cfg;
        return save_graph_pdf(
            &format!("{:?}", self),
            &format!("{}/{}.{}.dot", ccfg.output_dir, ccfg.top_module, pfx),
            &format!("{}/{}.{}.pdf", ccfg.output_dir, ccfg.top_module, pfx));
    }
}

impl Debug for Circuit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent: &str = "    ";
        let graph = &self.graph;
        let io_i = self.get_nodes_type(Primitive::Input);

        // Push Input nodes
        let mut q: Vec<NodeIndex> = vec![];
        for nidx in io_i.iter() {
            q.push(*nidx);
        }

        write!(f, "digraph {{\n")?;

        // Print nodes in BFS order
        let mut vis_map = graph.visit_map();
        while !q.is_empty() {
            let nidx = q.remove(0);
            if vis_map.is_visited(&nidx) {
                continue;
            }
            vis_map.visit(nidx);
            let node = graph.node_weight(nidx).unwrap();
            // red, blue, green, white, purple
            let proc = node.clone().info().coord.proc % 5;
            let color = match proc {
                0 => "red",
                1 => "blue",
                2 => "green",
                3 => "orange",
                4 => "purple",
                _ => "white",
            };
            write!(
                f,
                "{}{} [ label = {:?} color = \"{}\"]\n",
                indent,
                nidx.index(),
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\ndbg val: {}",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap,
                        node.info().debug.val),
                color
            )?;

            let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
            while let Some(cidx) = childs.next_node(&graph) {
                if !vis_map.is_visited(&cidx) {
                    q.push(cidx);
                }
            }
        }

        // Print the unvisited nodes
        for nidx in graph.node_indices() {
            if vis_map.is_visited(&nidx) {
                continue;
            }
            let node = graph.node_weight(nidx).unwrap();

            // red, blue, green, white, purple
            let proc = node.clone().info().coord.proc % 5;
            let color = match proc {
                0 => "red",
                1 => "blue",
                2 => "green",
                3 => "orange",
                4 => "purple",
                _ => "white",
            };
            write!(
                f,
                "{}{} [ label = {:?} color = \"{}\"]\n",
                indent,
                nidx.index(),
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\ndbg val: {}",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap,
                        node.info().debug.val),
                color
            )?;
        }

        for (_, edge) in graph.edge_references().enumerate() {
            write!(f, "{}{} {} {} ",
                indent, edge.source().index(), "->", edge.target().index())?;
            writeln!(f, "[ label=\"{:?}\" ]", edge.weight().signal)?;
        }

        write!(f, "}}")
    }
}
