use crate::instruction::*;
use crate::primitives::*;
use petgraph::Direction::{Incoming, Outgoing};
use std::cmp::max;

/// # `map_instructions`
/// - After the instructions are scheduled, set the appropriate registers and
/// network input values
pub fn map_instructions(circuit: Circuit) -> Circuit {
    let graph = circuit.graph;

    let mut max_proc = 0;
    for nidx in graph.node_indices() {
        let node = graph.node_weight(nidx).unwrap();
        max_proc = max(max_proc, node.clone().get_info().proc);
    }

    let mut all_insts: Vec<Vec<Instruction>> = vec![];
    for _ in 0..(max_proc + 1) {
        let mut insts: Vec<Instruction> = vec![];
        for _ in 0..circuit.ctx.gates_per_partition {
            insts.push(Instruction::default());
        }
        all_insts.push(insts);
    }

    for nidx in graph.node_indices() {
        let node = graph.node_weight(nidx).unwrap();
        let node_insts = all_insts.get_mut(node.get_info().proc as usize).unwrap();
        let node_inst = node_insts.get_mut(node.get_info().pc as usize).unwrap();

        // assign opcode
        node_inst.valid = true;
        node_inst.opcode = node.is();

        if node.is() == Primitives::Lut {
            let lut_node = node.get_lut().unwrap();
            let table_vec = lut_node.table;
            let mut table: u64 = 0;
            assert!(
                table_vec.len() <= 6,
                "can support up to 6 operands with u64"
            );

            for entry in table_vec.iter() {
                let mut x = 0;
                for (i, e) in entry.iter().enumerate() {
                    x = x + (e << i);
                }
                table = table | (1 << x);
            }
            node_inst.lut = table;
        }

        // assign operands
        let mut parents = graph.neighbors_directed(nidx, Incoming).detach();
        while let Some(pidx) = parents.next_node(&graph) {
            let edge_idx = graph.find_edge(pidx, nidx).unwrap();
            let mut op_idx = 0;
            if node.is() == Primitives::Lut {
                let lut_inputs = node.get_lut().unwrap().inputs;
                let edge_name = graph.edge_weight(edge_idx).unwrap();
                op_idx = lut_inputs.iter().position(|n| n == edge_name).unwrap();
            }

            let parent = graph.node_weight(pidx).unwrap();

            if parent.get_info().proc == node.get_info().proc {
                node_inst.operands.push(Operand {
                    rs: parent.get_info().pc,
                    local: true,
                    idx: op_idx as u32,
                });
            } else {
                node_inst.operands.push(Operand {
                    rs: parent.get_info().pc + circuit.ctx.network_latency,
                    local: false,
                    idx: op_idx as u32,
                });
            }
        }
        node_inst.operands.sort_by(|a, b| a.idx.cmp(&b.idx));

        // assign sin
        let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&graph) {
            let child = graph.node_weight(cidx).unwrap();

            if child.get_info().proc != node.get_info().proc {
                let child_insts = all_insts.get_mut(child.get_info().proc as usize).unwrap();
                let child_inst = child_insts
                    .get_mut((node.get_info().pc + circuit.ctx.network_latency) as usize)
                    .unwrap();
                child_inst.valid = true;
                child_inst.sin.valid = true;
                child_inst.sin.idx = node.get_info().proc;
            }
        }
    }

    return Circuit {
        instructions: all_insts,
        graph: graph,
        ..circuit
    };
}
