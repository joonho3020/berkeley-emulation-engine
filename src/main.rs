use petgraph::dot::{Config, Dot};

mod parser;
mod passes;
mod primitives;

use crate::passes::runner;
use crate::primitives::Context;

fn main() {
    let res = parser::parse_blif_file("examples/Adder.lut.blif");
    match res {
        Ok(c) => {
            let ctx = Context {
                num_partitions: 2,
                kaminpar_epsilon: 0.03,
                kaminpar_seed: 123,
                kaminpar_nthreads: 6,
            };
            let c2 = runner::run_compiler_passes(c, ctx);
            let output = format!("{:?}", Dot::with_config(&c2.graph, &[Config::EdgeNoLabel]));
            println!("{}", output);
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
