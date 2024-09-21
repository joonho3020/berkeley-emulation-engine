use indexmap::IndexMap;
use petgraph::graph::NodeIndex;
use crate::common::*;
use blif_parser::{parser::parse_blif_file, primitives::ParsedPrimitive};

fn module_to_circuit(module: &ParsedPrimitive, circuit: &mut Circuit) {
    let mut net_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();
    let mut out_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();

    if let ParsedPrimitive::Module { name:_, inputs, outputs, elems } = module {
        // Parse inputs
        for i in inputs.iter() {
            let nidx = circuit.graph.add_node(HWNode::new(ParsedPrimitive::Input { name: i.to_string() }));
            net_to_nodeidx.insert(i.to_string(), nidx);
        }

        // Parse outputs
        for o in outputs.iter() {
            let nidx = circuit.graph.add_node(HWNode::new(ParsedPrimitive::Output { name: o.to_string() }));
            out_to_nodeidx.insert(o.to_string(), nidx);
        }

        // Add nodes to graph
        for e in elems.iter() {
            let nidx = circuit.graph.add_node(HWNode::new(e.clone()));
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
                        circuit
                            .graph
                            .add_edge(*src_nidx, *dst_nidx, HWEdge::new(inet.to_string()));
                    }
                }
                ParsedPrimitive::Gate { c:_, d, q, r:_, e } => {
                    let d_idx = net_to_nodeidx.get(d).unwrap();
                    let q_idx = net_to_nodeidx.get(q).unwrap();
                    circuit.graph.add_edge(*d_idx, *q_idx, HWEdge::new(d.to_string()));

                    match &e {
                        Some(e) => {
                            let e_idx = net_to_nodeidx.get(e).unwrap();
                            circuit.graph.add_edge(*e_idx, *q_idx, HWEdge::new(e.to_string()));
                        }
                        None => (),
                    };
                }
                ParsedPrimitive::Latch { input, output, .. } => {
                    let d_idx = net_to_nodeidx.get(input).unwrap();
                    let q_idx = net_to_nodeidx.get(output).unwrap();
                    circuit
                        .graph
                        .add_edge(*d_idx, *q_idx, HWEdge::new(input.to_string()));
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
            circuit.graph.add_edge(*src_nidx, *dst_nidx, HWEdge::new(o.to_string()));
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
