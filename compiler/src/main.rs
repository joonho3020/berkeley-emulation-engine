mod fsim;
mod instruction;
mod passes;
mod primitives;

use crate::fsim::module::*;
use crate::passes::parser;
use crate::passes::runner;
use crate::primitives::Context;
use std::cmp::max;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let res = parser::parse_blif_file(&file_path);
    let mut parsed_circuit = match res {
        Ok(c) => {
            c
        }
        _ => {
            println!("ERROR: blif parsing failed!");
            return;
        }
    };

    let ctx = Context {
        gates_per_partition: 128,
        network_latency: 1,
    };
    parsed_circuit.set_ctx(ctx);

    let mapped_circuit = runner::run_compiler_passes(parsed_circuit);
    let mut module = Module::from_circuit(mapped_circuit);
    let inputs = [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [1, 1, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 1, 1],
    ];
    for (cycle, input) in inputs.iter().enumerate() {
        println!("----- cycle: {} -------", cycle);
        println!("input: {:?}", input);
        let output = module.run_cycle(input.to_vec());
        println!("output: {:?}", output);
    }
}
