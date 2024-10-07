use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::*
};
use indexmap::IndexMap;
use petgraph::graph::NodeIndex;
use blif_parser::{parser::parse_blif_file, primitives::ParsedPrimitive};

fn extract_index(input: &str) -> Option<u32> {
    if let (Some(start), Some(end)) = (input.find('['), input.find(']')) {
        let index_str = &input[start + 1..end];
        index_str.parse::<u32>().ok()
    } else {
        None
    }
}

/// Checks if the parent node is a SRAM and sets the SignalType accordingly
fn signal_type(src: &NodeIndex, circuit: &Circuit, wire: &str) -> SignalType {
    let node = circuit.graph.node_weight(*src).unwrap();
    if let CircuitPrimitive::SRAMNode { name:_, conns } = &node.prim {
        // Parent node is a SRAM
        // Signal type should be a SRAMRdData as it is the only output port from SRAM nodes
        let wire_to_port: IndexMap<String, String> =
            conns
                .iter()
                .map(|(k, v)| (v.clone(), k.clone()))
                .collect();

        let port = wire_to_port.get(wire).unwrap();
        let bidx = extract_index(port);
        assert!(bidx.is_some(),
            "SRAM Read Data Port ({}) does not contain a index",
            port);

        return SignalType::SRAMRdData { name: wire.to_string(), idx: bidx.unwrap() };
    } else {
        return SignalType::Wire { name: wire.to_string() };
    }
}

// nidx  wire
//   o  ------> o ----> o ---->
fn module_to_circuit(module: &ParsedPrimitive, circuit: &mut Circuit) {
    let mut net_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();
    let mut out_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();
    let mut sram_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();

    if let ParsedPrimitive::Module { name:_, inputs, outputs, elems } = module {
        // Parse inputs
        for i in inputs.iter() {
            let nidx = circuit.graph.add_node(
                HWNode::new(CircuitPrimitive::Input { name: i.to_string() }));
            net_to_nodeidx.insert(i.to_string(), nidx);
        }

        // Parse outputs
        for o in outputs.iter() {
            let nidx = circuit.graph.add_node(
                HWNode::new(CircuitPrimitive::Output { name: o.to_string() }));
            out_to_nodeidx.insert(o.to_string(), nidx);
        }

        // Add nodes to graph
        for e in elems.iter() {
            let nidx = circuit.graph.add_node(
                HWNode::new(CircuitPrimitive::from(e)));
            match e {
                ParsedPrimitive::Lut { inputs:_, output, .. } => {
                    net_to_nodeidx.insert(output.to_string(), nidx);
                }
                ParsedPrimitive::Gate { c:_, d:_, q, .. } => {
                    net_to_nodeidx.insert(q.to_string(), nidx);
                }
                ParsedPrimitive::Latch { input:_, output, .. } => {
                    net_to_nodeidx.insert(output.to_string(), nidx);
                }
                ParsedPrimitive::Subckt { name, conns } => {
                    sram_to_nodeidx.insert(name.to_string(), nidx);
                    for (port, wire) in conns.iter() {
                        if port.contains("R0_data") ||
                           port.contains("RW0_rdata") {
                            net_to_nodeidx.insert(wire.to_string(), nidx);
                        }
                    }
                }
                _ => {
                    assert!(false, "Unrecoginzed primitive: {:?}", e);
                }
            }
        }

        // Add edges to graph
        for elem in elems.iter() {
            match elem {
                ParsedPrimitive::Lut { inputs, output, .. } => {
                    for inet in inputs.iter() {
                        let src_nidx = net_to_nodeidx.get(inet).unwrap();
                        let dst_nidx = net_to_nodeidx.get(output).unwrap();
                        let sig = signal_type(src_nidx, circuit, inet);
                        circuit
                            .graph
                            .add_edge(*src_nidx, *dst_nidx, HWEdge::new(sig));
                    }
                }
                ParsedPrimitive::Gate { c:_, d, q, r:_, e } => {
                    let d_idx = net_to_nodeidx.get(d).unwrap();
                    let q_idx = net_to_nodeidx.get(q).unwrap();
                    let sig = signal_type(d_idx, circuit, d);
                    circuit.graph.add_edge(*d_idx, *q_idx, HWEdge::new(sig));

                    match &e {
                        Some(e) => {
                            let e_idx = net_to_nodeidx.get(e).unwrap();
                            let e_sig = signal_type(e_idx, circuit, e);
                            circuit.graph.add_edge(*e_idx, *q_idx, HWEdge::new(e_sig));
                        }
                        None => (),
                    };
                }
                ParsedPrimitive::Latch { input, output, .. } => {
                    let d_idx = net_to_nodeidx.get(input).unwrap();
                    let q_idx = net_to_nodeidx.get(output).unwrap();
                    let sig = signal_type(d_idx, circuit, input);
                    circuit
                        .graph
                        .add_edge(*d_idx, *q_idx, HWEdge::new(sig));
                }
                ParsedPrimitive::Subckt { name, conns } => {
                    let sram_idx = sram_to_nodeidx.get(name).unwrap();
                    for (port, wire) in conns.iter() {
                        let bidx = extract_index(port);
                        let p_idx = net_to_nodeidx.get(wire).unwrap();

                        let sig = if port.contains("RW0_addr") {
                            Some(SignalType::SRAMRdWrAddr {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("RW0_en") {
                            Some(SignalType::SRAMRdWrEn {
                                name: wire.to_string()
                            })
                        } else if port.contains("RW0_wmode") {
                            Some(SignalType::SRAMRdWrMode {
                                name: wire.to_string()
                            })
                        } else if port.contains("RW0_mask") {
                            Some(SignalType::SRAMWrMask {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("RW0_wdata") {
                            Some(SignalType::SRAMWrData {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("R0_addr") {
                            Some(SignalType::SRAMRdAddr {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("R0_en") {
                            Some(SignalType::SRAMRdEn {
                                name: wire.to_string()
                            })
                        } else if port.contains("W0_en") {
                            Some(SignalType::SRAMWrEn {
                                name: wire.to_string()
                            })
                        } else if port.contains("W0_mask") {
                            Some(SignalType::SRAMWrMask {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("W0_addr") {
                            Some(SignalType::SRAMWrAddr {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else if port.contains("W0_data") {
                            Some(SignalType::SRAMWrData {
                                name: wire.to_string(), idx: bidx.unwrap()
                            })
                        } else {
                            None
                        };
                        match sig {
                            Some(s) => {
                                circuit.graph.add_edge(*p_idx, *sram_idx, HWEdge::new(s));
                            }
                            None => {}
                        }
                    }
                }
                _ => {
                    assert!(false, "Unrecoginzed primitive: {:?}", elem);
                }
            }
        }

        // Add edges connecting to the Output nodes
        for o in outputs.iter() {
            let src_nidx = net_to_nodeidx.get(&o.to_string()).unwrap();
            let dst_nidx = out_to_nodeidx.get(&o.to_string()).unwrap();
            let sig = signal_type(src_nidx, circuit, o);
            circuit.graph.add_edge(*src_nidx, *dst_nidx, HWEdge::new(sig));
        }
    }
}

pub fn blif_to_circuit(input_file_path: &str) -> Result<Circuit, String> {
    let mut circuit = Circuit::default();
    let blif_file = parse_blif_file(input_file_path);
    match blif_file {
        Ok(modules) => {
            assert!(modules.len() == 1, "Can only ingest flattened design for now");
            for module in modules.iter() {
                match module {
                    ParsedPrimitive::Module { .. } => {
                        module_to_circuit(module, &mut circuit);
                    }
                    _ => {
                        assert!(false, "None module found");
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Error while reading the file:\n{}", e).to_string());
        }
    }
    return Ok(circuit);
}
