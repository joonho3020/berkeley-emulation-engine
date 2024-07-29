mod fsim;
mod instruction;
mod passes;
mod primitives;

use crate::fsim::common::*;
use crate::fsim::module::*;
use crate::passes::parser;
use crate::passes::runner;
use crate::primitives::Configuration;
use indexmap::IndexMap;
use std::cmp::max;
use std::env;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let res = parser::parse_blif_file(&file_path);
    let mut parsed_circuit = match res {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            return Err(e);
        }
    };

    let cfg = Configuration {
        gates_per_partition: 128,
        network_latency: 1,
    };
    parsed_circuit.set_cfg(cfg);

    let mapped_circuit = runner::run_compiler_passes(parsed_circuit);
    // println!("{:?}", mapped_circuit);

    let mut module = Module::from_circuit(mapped_circuit);
    let inputs = IndexMap::from([
        ("io_value1[0]",     [0, 0, 1, 1, 1, 1, 1, 1]),
        ("io_value1[1]",     [0, 0, 1, 1, 1, 1, 1, 1]),
        ("io_value2[0]",     [0, 0, 1, 1, 1, 1, 1, 1]),
        ("io_value2[1]",     [0, 0, 0, 0, 0, 0, 0, 0]),
        ("io_loadingValues", [0, 0, 1, 0, 0, 0, 0, 0]),
    ]);
    let cycles = inputs.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        // poke inputs
        for key in inputs.keys() {
            let val = inputs[key].get(cycle);
            match val {
                Some(b) => {
                    module.poke(key.to_string(), *b as Bit)?;
                }
                None => {}
            }
        }

        // run a cycle
        module.run_cycle();
        println!("outputs: {:?}", module.get_outputs());
    }
    return Ok(());
}
