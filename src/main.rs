use petgraph::dot::{Config, Dot};

mod parser;
mod passes;
mod primitives;

use crate::passes::runner;
use crate::primitives::Context;

fn main() {
    let res = parser::parse_blif_file("examples/GCD.lut.blif");
    match res {
        Ok(c) => {
            let ctx = Context {
                gates_per_partition: 128,
            };
            let c2 = runner::run_compiler_passes(c, ctx);
            let filtered_graph = c2.graph.filter_map(
                |_, y| if y.clone().get_info().proc == 17 { Some(y) } else { None },
                |_, y| Some(y));

            let output = format!("{:?}", Dot::with_config(&filtered_graph, &[Config::EdgeNoLabel]));
            println!("{}", output);
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
