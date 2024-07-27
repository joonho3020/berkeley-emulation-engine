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
            };
            let c2 = runner::run_compiler_passes(c, ctx);
            // let _ = c2.save_all_subgraphs(file_path.to_string());
            let _ = c2.save_insts(file_path.to_string());
            println!("{:?}", c2);

            let all_insts = c2.instructions;
            let nprocs = all_insts.len();
            let mut max_steps = 0;
            for insts in all_insts.iter() {
                max_steps = max(max_steps, insts.len());
            }

            let mut module = Module::new(nprocs, max_steps);
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
                let output = module.run_cycle(input.to_vec());
                println!("----- cycle: {} -------", cycle);
                println!("input: {:?}", input);
                println!("output: {:?}", output);
            }
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
