use crate::common::{
    circuit::Circuit,
    primitive::*
};
use itertools::Itertools;
use petgraph::Direction::{Incoming, Outgoing};

pub fn check_connectivity(circuit: &Circuit) {
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        let childs  = circuit.graph.neighbors_directed(nidx, Outgoing);
        let parents = circuit.graph.neighbors_directed(nidx, Incoming);

        let ccnt =  childs.clone().collect_vec().len();
        let pcnt = parents.clone().collect_vec().len();
        match node.is() {
            Primitive::Lut => {
                assert!(ccnt > 0, "Lut {:?} does not have a output", node);

                if let CircuitPrimitive::Lut { inputs, output:_, table } = &node.prim {
                    if inputs.len() != pcnt {
                        println!(
                            "Lut {:?} number of parents {} does not match number of table entries {}",
                            node, pcnt, table.len());

                        let pn = parents.map(|idx| circuit.graph.node_weight(idx).unwrap().name().to_string()).collect_vec();
                        let missing = inputs.into_iter().filter(|x| !pn.contains(*x)).collect_vec();

                        println!("Missing: {:?}",  missing);

                        for x in circuit.graph.node_indices() {
                            let xnode = circuit.graph.node_weight(x).unwrap();
                            for m in missing.iter() {
                                if **m == xnode.name().to_string() {
                                    let xnode_childs = circuit.graph.neighbors_directed(x, Outgoing);
                                    for c in xnode_childs {
                                        let cnode = circuit.graph.node_weight(c).unwrap();
                                        println!("Childs of missing node: {:?}", cnode);
                                    }
                                }
                            }
                        }
                        assert!(false);
                    }
                }
            }
            Primitive::Latch |
            Primitive::Gate => {
                assert!(pcnt == 1, "Latch/Gate should have 1 input, got {}", pcnt);
                assert!(ccnt > 0, "Latch/Gate with no outputs: {:?}", node);
            }
            Primitive::SRAMRdEn         |
                Primitive::SRAMWrEn     |
                Primitive::SRAMRdAddr   |
                Primitive::SRAMWrAddr   |
                Primitive::SRAMWrMask   |
                Primitive::SRAMWrData   |
                Primitive::SRAMRdWrEn   |
                Primitive::SRAMRdWrMode |
                Primitive::SRAMRdWrAddr => {
                assert!(pcnt == 1,
                    "SRAM input should have 1 input, got {}, node: {:?}",
                    pcnt, node);
            }
            Primitive::SRAMRdData => {
                assert!(ccnt > 0, "SRAM output w/ no output {:?}", node);
            }
            _ => { }
        }
    }
}
