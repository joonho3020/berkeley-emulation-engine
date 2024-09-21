use crate::common::*;
use indexmap::IndexMap;
use blif_parser::primitives::Primitive;
use petgraph::visit::EdgeRef;
use petgraph::Direction::{Incoming, Outgoing};

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
        let node_inst = circuit.emul
            .module_mappings.get_mut(&coord.module).unwrap()
            .proc_mappings.get_mut(&coord.proc).unwrap()
            .instructions.get_mut(node.info().pc as usize).unwrap();

        // assign opcode
        node_inst.valid = true;
        node_inst.opcode = node.is();

        // build LUT table
        if node.is() == Primitive::Lut {
            let table_vec = node.get_lut_table().unwrap();
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

        // assign operands
        let pedges = circuit.graph.edges_directed(nidx, Incoming);
        for pedge in pedges {
            let pnode = circuit.graph.node_weight(pedge.source()).unwrap();

            let mut op_idx = 0;
            if node.is() == Primitive::Lut {
                let lut_inputs = node.get_lut_inputs().unwrap();
                op_idx = lut_inputs.iter().position(|n| n == pnode.name()).unwrap();
            }

            let pcoord = pnode.info().coord;
            let coord =  node.info().coord;
            let use_ldm = pcoord == coord;

            let pc_offset = match &pedge.weight().route {
                Some(r) => pcfg.nw_route_lat(&r),
                None    => 0
            };
            node_inst.operands.push(Operand {
                rs: pnode.info().pc + pc_offset,
                local: use_ldm,
                idx: op_idx as u32
            })
        }
        node_inst.operands.sort_by(|a, b| a.idx.cmp(&b.idx));

        // assign switch info
        let cedges = circuit.graph.edges_directed(nidx, Outgoing);
        for cedge in cedges {
            match &cedge.weight().route {
                Some(route) => {
                    let mut cur_route = NetworkRoute::new();
                    for (i, path) in route.iter().enumerate() {
                        let src_send_pc = if i == 0 {
                            node.info().pc
                        } else {
                            node.info().pc + pcfg.nw_route_dep_lat(&cur_route)
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
                                src_inst.valid = true;
                                if src_inst.sinfo.fwd_set {
                                    assert!(src_inst.sinfo.fwd == (i != 0),
                                        "node: {} coord {:?} pc: {} already set, but overwritten",
                                        node.name(), path.src, src_send_pc);
                                }
                                src_inst.sinfo.fwd = i != 0;
                                src_inst.sinfo.fwd_set = true;
                            }
                        }

                        cur_route.push_back(*path);
                        let dst_recv_pc = node.info().pc + pcfg.nw_route_lat(&cur_route);
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
