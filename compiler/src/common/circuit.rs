use crate::common::config::*;
use crate::common::mapping::*;
use crate::common::utils::*;
use crate::common::primitive::*;
use crate::common::hwgraph::*;
use crate::fsim::board::Board;
use std::fmt::Debug;
use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, VisitMap, Visitable},
    Direction::{Incoming, Outgoing},
};


#[derive(Default, Clone)]
pub struct Circuit {
    pub compiler_cfg: CompilerConfig,
    pub platform_cfg: PlatformConfig,
    pub kaminpar_cfg: KaMinParConfig,
    pub graph: HWGraph,
    pub emul:  EmulatorMapping
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

    /// #debug_graph
    /// Given a `dbg_node` in the graph, search for all parents nodes up until
    /// it reaches Gate, Latch or Input.
    /// It will also print the bit-value associated with the node
    /// computed by the emulation processor.
    pub fn debug_graph(&self, dbg_node: NodeIndex, board: &Board) -> String {
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
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\n",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap),
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
                format!("{} {:?}\nmod: {} proc: {} pc: {}\nasap: {} alap: {}\n",
                        node.name(),
                        node.is(),
                        node.info().coord.module,
                        node.info().coord.proc,
                        node.info().pc,
                        node.info().rank.asap,
                        node.info().rank.alap),
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
