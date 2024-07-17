use petgraph::dot::{Config, Dot};

mod parser;
mod passes;
mod primitives;

use crate::passes::runner;

fn main() {
    let res = parser::parse_blif_file("examples/Adder.lut.blif");
    match res {
        Ok(c) => {
            let c2 = runner::run_compiler_passes(c);
            let output = format!("{:?}", Dot::with_config(&c2.graph, &[Config::EdgeNoLabel]));
            println!("{}", output);
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
