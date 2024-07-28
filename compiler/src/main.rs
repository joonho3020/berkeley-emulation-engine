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
    // env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let res = parser::parse_blif_file(&file_path);
    match res {
        Ok(c) => {
            let ctx = Context {
                gates_per_partition: 128,
                network_latency: 1,
            };
            let c2 = runner::run_compiler_passes(c, ctx.clone());
            let all_insts = c2.instructions;
            let nprocs = all_insts.len();
            let mut max_pc = 0;
            for nidx in c2.graph.node_indices() {
                let node = c2.graph.node_weight(nidx).unwrap();
                max_pc = max(max_pc, node.get_info().pc + ctx.network_latency);
            }

            let mut module = Module::new(nprocs, (max_pc + 1) as usize);
            module.set_insts(all_insts);
            // io_value2[1]
            // io_loadingValues
            // io_value1[0]
            // io_value1[1]
            // io_value2[0]
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
        Err(_) => {
            println!("ERROR");
        }
    }
}
