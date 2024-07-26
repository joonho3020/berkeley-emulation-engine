use crate::primitives::*;
use petgraph::Direction::{Incoming, Outgoing};
use std::cmp::max;

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
        node_inst.opcode = node.is();

        // assign operands
        let mut parents = graph.neighbors_directed(nidx, Incoming).detach();
        while let Some(pidx) = parents.next_node(&graph) {
            let parent = graph.node_weight(pidx).unwrap();

            if parent.get_info().proc == node.get_info().proc {
                node_inst.operands.push(Operand {
                    valid: true,
                    rs: parent.get_info().pc,
                    local: true,
                });
            } else {
                node_inst.operands.push(Operand {
                    valid: true,
                    rs: parent.get_info().pc, // TODO: add network latency consideration
                    local: false,
                });
            }
        }

        // assign sin
        let mut childs = graph.neighbors_directed(nidx, Outgoing).detach();
        while let Some(cidx) = childs.next_node(&graph) {
            let child = graph.node_weight(cidx).unwrap();

            if child.get_info().proc != node.get_info().proc {
                let child_insts = all_insts.get_mut(child.get_info().proc as usize).unwrap();
                let child_inst = child_insts.get_mut(node.get_info().pc as usize).unwrap();
                child_inst.sin.valid = true;
                child_inst.sin.idx = node.get_info().pc; // TODO: add network latency consideration
            }
        }
    }

    return Circuit {
        instructions: all_insts,
        graph: graph,
        ..circuit
    };
}
