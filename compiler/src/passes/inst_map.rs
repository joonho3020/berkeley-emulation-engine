use crate::common::*;
use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::visit::EdgeRef;
use petgraph::Direction::{Incoming, Outgoing};

/// # `map_instructions`
/// - After the instructions are scheduled, set the appropriate registers and
/// network input values
pub fn map_instructions(circuit: &mut Circuit) {
//     for (_, mmap) in circuit.emul.module_mappings.iter_mut() {
//         for pi in 0..mmap.used_procs {
//             mmap.proc_mappings.insert(pi, ProcessorMapping {
//                 instructions: vec![Instruction::default(); circuit.emul.host_steps as usize],
//                 signal_map: IndexMap::new()
//             });
//         }
//     }
// 
//     let pcfg = &circuit.platform_cfg;
//     for nidx in circuit.graph.node_indices() {
//         let node = circuit.graph.node_weight(nidx).unwrap();
//         let coord = node.info().coord;
//         let node_inst = circuit.emul
//             .module_mappings.get_mut(&coord.module).unwrap()
//             .proc_mappings.get_mut(&coord.proc).unwrap()
//             .instructions.get_mut(node.info().pc as usize).unwrap();
// 
//         // assign opcode
//         node_inst.valid = true;
//         node_inst.opcode = node.is();
// 
//         // build LUT table
//         if node.is() == Primitives::Lut {
//             let lut_node = node.get_lut().unwrap();
//             let table_vec = &lut_node.table;
//             let mut table: u64 = 0;
//             let mut ops: u32 = 0;
//             for entry in table_vec.iter() {
//                 let mut x = 0;
//                 ops = entry.len() as u32;
//                 for (i, e) in entry.iter().enumerate() {
//                     x = x + (e << i);
//                 }
//                 table = table | (1 << x);
//                 assert!(x < 64,
//                     "Can support up to 6 operands with u64, node {} {:?} {:?}",
//                     node.name(), node.is(), node.info());
//             }
//             let mut table_repeated: u64 = table;
//             let nops = pcfg.lut_inputs - ops;
//             for i in 0..(1 << nops) {
//                 table_repeated |= table << ((1 << ops) * i);
//             }
//             node_inst.lut = table_repeated;
//         }
// 
//         // assign operands
//         let pedges = circuit.graph.edges_directed(nidx, Incoming);
//         for pedge in pedges {
//             let pnode = circuit.graph.node_weight(pedge.source()).unwrap();
// 
//             let mut op_idx = 0;
//             if node.is() == Primitives::Lut {
//                 let lut_inputs = node.get_lut().unwrap().inputs;
//                 op_idx = lut_inputs.iter().position(|n| n == pnode.name()).unwrap();
//             }
// 
//             let pcoord = pnode.info().coord;
//             let coord =  node.info().coord;
//             let use_ldm = pcoord == coord;
// 
//             let pc_offset = if use_ldm {
//                 0
//             } else {
//                 match pedge.weight().route {
//                     Some(r) => {
//                         pcfg.nw_route_lat(&r)
//                     }
//                     None => {
//                         pcfg.inter_proc_dep_lat()
//                     }
//                 }
//             };
//             node_inst.operands.push(Operand {
//                 rs: pnode.info().pc + pc_offset,
//                 local: use_ldm,
//                 idx: op_idx as u32
//             })
//         }
//         node_inst.operands.sort_by(|a, b| a.idx.cmp(&b.idx));
// 
//         // assign sin
//         let cedges = circuit.graph.edges_directed(nidx, Outgoing);
//         for cedge in cedges {
//             match cedge.weight().route {
//                 Some(route) => {
//                     let mut cur_route = NetworkRoute::new();
//                     for (i, path) in route.iter().enumerate() {
//                         cur_route.push_back(*path);
//                         let dst_recv_pc = node.info().pc + pcfg.nw_route_lat(&cur_route);
//                         let inst = circuit.emul
//                             .module_mappings.get_mut(&path.dst.module).unwrap()
//                             .proc_mappings.get_mut(&path.dst.proc).unwrap()
//                             .instructions.get_mut(dst_recv_pc as usize).unwrap();
//                         let local = path.src.module == path.dst.module;
//                         inst.valid = true;
//                         inst.sinfo.idx = path.src.proc;
//                         inst.sinfo.recv_local = path.src.module == path.dst.module;
//                         inst.sinfo.fwd = i != (route.len() - 1);
//                     }
//                 }
//                 None => {
//                     let cnode = circuit.graph.node_weight(cedge.target()).unwrap();
//                     let ccoord = cnode.info().coord;
//                     assert!(cnode.info().coord.module == node.info().coord.module,
//                         "No path between parent and child in different modules");
// 
//                     let cnode_recv_pc = node.info().pc + pcfg.remote_sin_lat();
//                     if cnode.info().coord.proc != node.info().coord.proc {
//                         let child_inst = circuit.emul
//                             .module_mappings.get_mut(&ccoord.module).unwrap()
//                             .proc_mappings.get_mut(&coord.proc).unwrap()
//                             .instructions.get_mut(cnode_recv_pc as usize).unwrap();
//                         child_inst.valid = true;
//                         child_inst.sinfo.idx = node.info().coord.proc;
//                         child_inst.sinfo.recv_local = true;
//                         child_inst.sinfo.fwd = false;
//                     }
//                 }
//             }
//         }
// 
//         let nodemap = NodeMapInfo {
//             info: node.info(),
//             idx: nidx,
//         };
//         circuit.emul
//             .module_mappings.get_mut(&coord.module).unwrap()
//             .proc_mappings.get_mut(&coord.proc).unwrap()
//             .signal_map.insert(node.name().to_string(), nodemap);
//     }
}
