mod fsim;
mod passes;
mod primitives;

use crate::fsim::module;
use crate::passes::parser;
use crate::passes::runner;
use crate::primitives::Context;
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
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
