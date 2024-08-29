use crate::common::*;
use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::Direction::{Incoming, Outgoing};

/// # `map_instructions`
/// - After the instructions are scheduled, set the appropriate registers and
/// network input values
pub fn map_instructions(circuit: &mut Circuit) {
    let mut signal_map: IndexMap<String, NodeMapInfo> = IndexMap::new();
    let mut all_insts: Vec<Vec<Instruction>> = vec![];
    for _ in 0..circuit.emulator.used_procs {
        let mut insts: Vec<Instruction> = vec![];
        for _ in 0..circuit.emulator.cfg.max_steps {
            insts.push(Instruction::default());
        }
        all_insts.push(insts);
    }

    let cfg = &circuit.emulator.cfg;
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let node_insts = all_insts.get_mut(node.get_info().proc as usize).unwrap();
        let node_inst = node_insts.get_mut(node.get_info().pc   as usize).unwrap();

        // assign opcode
        node_inst.valid = true;
        node_inst.opcode = node.is();

        // build LUT table
        if node.is() == Primitives::Lut {
            let lut_node = node.get_lut().unwrap();
            let table_vec = &lut_node.table;
            let mut table: u64 = 0;
            let mut ops: u32 = 0;
            for entry in table_vec.iter() {
                let mut x = 0;
                ops = entry.len() as u32;
                for (i, e) in entry.iter().enumerate() {
                    x = x + (e << i);
                }
                table = table | (1 << x);

                assert!(x < 64,
                    "Can support up to 6 operands with u64, node {} {:?} {:?}",
                    node.name(), node.is(), node.get_info());
            }
            let mut table_repeated: u64 = table;
            let nops = cfg.lut_inputs - ops;
            for i in 0..(1 << nops) {
                table_repeated |= table << ((1 << ops) * i);
            }
            node_inst.lut = table_repeated;
        }

        // assign operands
        let mut parents = circuit.graph.neighbors_directed(nidx, Incoming).detach();
        while let Some(pidx) = parents.next_node(&circuit.graph) {
            let edge_idx = circuit.graph.find_edge(pidx, nidx).unwrap();
            let mut op_idx = 0;
            if node.is() == Primitives::Lut {
                let lut_inputs = node.get_lut().unwrap().inputs;
                let edge_name = circuit.graph.edge_weight(edge_idx).unwrap();
                op_idx = lut_inputs.iter().position(|n| n == edge_name).unwrap();
            }

            let parent = circuit.graph.node_weight(pidx).unwrap();

            if parent.get_info().proc == node.get_info().proc {
                node_inst.operands.push(Operand {
                    rs: parent.get_info().pc,
                    local: true,
                    idx: op_idx as u32,
                });
            } else {
                node_inst.operands.push(Operand {
                    rs: parent.get_info().pc,
                    local: false,
                    idx: op_idx as u32,
                });
            }
        }
        node_inst.operands.sort_by(|a, b| a.idx.cmp(&b.idx));

        // assign sin
        let mut childs = circuit.graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&circuit.graph) {
            let child = circuit.graph.node_weight(cidx).unwrap();

            if child.get_info().proc != node.get_info().proc {
                let child_insts = all_insts
                    .get_mut(child.get_info().proc as usize).unwrap();
                let child_inst = child_insts
                    .get_mut((node.get_info().pc + cfg.remote_sin_lat()) as usize)
                    .unwrap();
                child_inst.valid = true;
                child_inst.sin.idx = node.get_info().proc;
            }
        }

        // add to signal map
        let nodemap = NodeMapInfo {
            info: node.get_info(),
            idx: nidx,
        };
        signal_map.insert(node.name().to_string(), nodemap);
    }
    circuit.emulator.signal_map = signal_map;
    circuit.emulator.instructions = all_insts;
}
