use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::NodeMapInfo,
    mapping::*,
    instruction::*,
    network::*
};
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::{
    visit::EdgeRef,
    Direction::{Incoming, Outgoing}
};

/// # `map_instructions`
/// - After the instructions are scheduled, set the appropriate registers and
/// network input values
pub fn map_instructions(circuit: &mut Circuit) {
    let pcfg = &circuit.platform_cfg;

    for (_, mmap) in circuit.emul.module_mappings.iter_mut() {
        for pi in 0..pcfg.num_procs {
            mmap.proc_mappings.insert(pi, ProcessorMapping {
                instructions: vec![Instruction::default(); circuit.emul.host_steps as usize],
                signal_map: IndexMap::new()
            });
        }
    }

    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let coord = node.info().coord;
        let module_mapping = circuit.emul
            .module_mappings.get_mut(&coord.module).unwrap();
        let node_inst = module_mapping
            .proc_mappings.get_mut(&coord.proc).unwrap()
            .instructions.get_mut(node.info().pc as usize).unwrap();

        // assign opcode
        node_inst.valid = true;
        node_inst.opcode = Opcode::from(&node.is());

        // build LUT table
        match &node.prim {
            // Normal LUT
            CircuitPrimitive::Lut { inputs:_, output:_, table } => {
                let table_vec = table.to_vec();
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
                        node.name(), node.is(), node.info());
                }
                let mut table_repeated: u64 = table;
                let nops = pcfg.lut_inputs - ops;
                for i in 0..(1 << nops) {
                    table_repeated |= table << ((1 << ops) * i);
                }
                node_inst.lut = table_repeated;
            }
            // Constant LUT
            CircuitPrimitive::ConstLut { val, .. } => {
                let mut table: u64 = 0;
                let num_bits = 1 << pcfg.lut_inputs;
                for i in 0..num_bits {
                    table |= (*val as u64) << (i as u64);
                }
                node_inst.lut = table;
            }
            _ => { }
        }

        // assign operands
        let pedges = circuit.graph.edges_directed(nidx, Incoming);
        for pedge in pedges {
            let pnode = circuit.graph.node_weight(pedge.source()).unwrap();

            let mut op_idx = 0;
            match &node.prim {
                CircuitPrimitive::Lut { inputs, .. } => {
                    let lut_inputs = inputs.to_vec();
                    op_idx = lut_inputs.iter().position(|n| n == pnode.name()).unwrap();
                }
                _ => { }
            }

            let pcoord = pnode.info().coord;
            let coord =  node.info().coord;
            let use_ldm = pcoord == coord;

            let pc_offset = match &pedge.weight().route {
                Some(r) => pcfg.nw_route_lat(&r) - pcfg.fetch_decode_lat(),
                None    => 0
            };
            node_inst.operands.push(Operand {
                rs: pnode.info().pc + pc_offset,
                local: use_ldm,
                idx: op_idx as u32
            })
        }

        // Node is input to the SRAM processor
        if node.is() == Primitive::SRAMRdEn     ||
           node.is() == Primitive::SRAMWrEn     ||
           node.is() == Primitive::SRAMRdAddr   ||
           node.is() == Primitive::SRAMWrAddr   ||
           node.is() == Primitive::SRAMWrMask   ||
           node.is() == Primitive::SRAMWrData   ||
           node.is() == Primitive::SRAMRdWrAddr ||
           node.is() == Primitive::SRAMRdWrMode ||
           node.is() == Primitive::SRAMRdWrEn {
            assert!(node_inst.operands.len() == 1,
                "Number of operands for operator {:?} should be 1 but got {}",
                node.is(), node_inst.operands.len());

            // Mark this instruction as a memory operation
            node_inst.mem = true;

            // Set the operands
            let op_bits = pcfg.index_bits();
            let max_op_bits = (op_bits * (pcfg.lut_inputs - 1)) as u64;
            let max_op_num = (1u64 << max_op_bits) - 1u64;
            let uidx = node.prim.unique_sram_input_idx(pcfg);
            assert!(uidx as u64 <= max_op_num,
                "SRAM unique input idx {} > max_op_num {}", uidx, max_op_num);

            for i in 1..pcfg.lut_inputs {
                let sl = (i - 1) * op_bits;
                let rs = uidx >> sl;
                node_inst.operands.push(Operand {
                    rs: rs,
                    local: false,
                    idx: i
                });
            }

            // Set SRAM mapping
            if node.is() == Primitive::SRAMRdWrAddr ||
               node.is() == Primitive::SRAMRdWrMode ||
               node.is() == Primitive::SRAMRdWrEn {
                module_mapping.sram_mapping.port_type = SRAMPortType::SinglePortSRAM;
            } else {
                module_mapping.sram_mapping.port_type = SRAMPortType::OneRdOneWrPortSRAM;
            }

            if node.is() == Primitive::SRAMWrMask {
                module_mapping.sram_mapping.wmask_bits += 1;
            } else if node.is() == Primitive::SRAMWrData {
                module_mapping.sram_mapping.width_bits += 1;
            }
        }

        // Node is the output from the SRAM processor
        if node.is() == Primitive::SRAMRdData {
            assert!(node_inst.operands.len() == 0,
                "Number of operands for operator {:?} should be 0 but got {}",
                node.is(), node_inst.operands.len());

            // We only set mem when it is a SRAM input operation
            node_inst.mem = false;

            // Push a dummy node (SRAM indexing starts from operands[1:])
            node_inst.operands.push(Operand::default());

            // Set the operands
            let op_bits = pcfg.index_bits();
            let max_op_bits = (op_bits * (pcfg.lut_inputs - 1)) as u64;
            let max_op_num = (1u64 << max_op_bits) - 1u64;
            let uidx = node.prim.unique_sram_output_idx(pcfg);
            assert!(uidx as u64 <= max_op_num,
                "SRAM unique input idx {} > max_op_num {}", uidx, max_op_num);

            for i in 1..pcfg.lut_inputs {
                let sl = (i - 1) * op_bits;
                let rs = uidx >> sl;
                node_inst.operands.push(Operand {
                    rs: rs,
                    local: false,
                    idx: i
                });
            }
        }

        // Sort the operands in order
        node_inst.operands.sort_by(|a, b| a.idx.cmp(&b.idx));

        // assign switch info
        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            match &cedge.weight().route {
                Some(route) => {
                    let mut cur_route = NetworkRoute::new();
                    for (i, path) in route.iter().enumerate() {
                        let src_send_pc = if i == 0 {
                            node.info().pc + pcfg.nw_route_lat(&cur_route) - pcfg.fetch_decode_lat()
                        } else {
                            node.info().pc + pcfg.nw_route_dep_lat(&cur_route) - pcfg.fetch_decode_lat()
                        };

                        assert!(src_send_pc < circuit.emul.host_steps,
                                "src_send_pc {} >= host_steps {}",
                                src_send_pc, circuit.emul.host_steps);

                        let src_inst = circuit.emul
                            .module_mappings.get_mut(&path.src.module).unwrap()
                            .proc_mappings.get_mut(&path.src.proc).unwrap()
                            .instructions.get_mut(src_send_pc as usize).unwrap();

                        match path.tpe {
                            PathTypes::ProcessorInternal => {
                                // Do nothing
                            }
                            PathTypes::InterProcessor | PathTypes::InterModule => {
                                if src_inst.sinfo.fwd_set {
                                    assert!(src_inst.sinfo.fwd == (i != 0),
                                        "node: {} coord {:?} pc: {} already set, but overwritten",
                                        node.name(), path.src, src_send_pc);
                                }
                                // fwd is set when we after `nw_route_dep_lat`.
                                // i.e., when we can read the `sin_fwd_bit` register
                                src_inst.valid = true;
                                src_inst.sinfo.fwd = i != 0;
                                src_inst.sinfo.fwd_set = true;
                            }
                        }

                        cur_route.push_back(*path);

                        let dst_recv_pc = node.info().pc + pcfg.nw_route_lat(&cur_route) - pcfg.fetch_decode_lat();
                        let dst_inst = circuit.emul
                            .module_mappings.get_mut(&path.dst.module).unwrap()
                            .proc_mappings.get_mut(&path.dst.proc).unwrap()
                            .instructions.get_mut(dst_recv_pc as usize).unwrap();

                        match path.tpe {
                            PathTypes::ProcessorInternal => {
                                // Do nothing
                            }
                            PathTypes::InterProcessor | PathTypes::InterModule => {
                                if dst_inst.sinfo.local_set {
                                    assert!(dst_inst.sinfo.local == (path.src.module == path.dst.module),
                                        "node: {} coord {:?} pc: {} already set, but overwritten paths: {}",
                                        node.name(), path.dst, dst_recv_pc, node.name().to_string());
                                }
                                dst_inst.valid = true;
                                dst_inst.sinfo.idx = path.src.proc;
                                dst_inst.sinfo.local = path.src.module == path.dst.module;
                                dst_inst.sinfo.local_set = true;
                            }
                        }
                    }
                }
                None => {
                    assert!(false, "Edge with no NetworkRoute: {:?}", cedge);
                }
            }
        }

        if node.is() != Primitive::SRAMRdEn   &&
           node.is() != Primitive::SRAMWrEn   &&
           node.is() != Primitive::SRAMRdAddr &&
           node.is() != Primitive::SRAMWrAddr &&
           node.is() != Primitive::SRAMWrMask &&
           node.is() != Primitive::SRAMWrData &&
           node.is() != Primitive::SRAMRdData {
            let nodemap = NodeMapInfo {
                info: node.info().clone(),
                idx: nidx,
            };
            circuit.emul
                .module_mappings.get_mut(&coord.module).unwrap()
                .proc_mappings.get_mut(&coord.proc).unwrap()
                .signal_map.insert(node.name().to_string(), nodemap);
        } else {
            // HACK: Check if the SRAM is in the top of the module hierarchy
            // If it is, there may be aliases to the node which can mess things up
            // Otherwise, it is safe to add as a signal mapping
            if node.name().split(".").collect_vec().len() > 1 {
                let nodemap = NodeMapInfo {
                    info: node.info().clone(),
                    idx: nidx,
                };
                circuit.emul
                    .module_mappings.get_mut(&coord.module).unwrap()
                    .proc_mappings.get_mut(&coord.proc).unwrap()
                    .signal_map.insert(node.name().to_string(), nodemap);
                }
        }
    }
}
