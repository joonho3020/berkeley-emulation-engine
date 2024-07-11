use petgraph::{
    dot::{Dot, Config}
};

mod parser;
mod primitives;

fn main() {
    let res = parser::parse_blif_file("examples/Adder.lut.blif");
    match res {
        Ok(c) => {
            let output = format!("{:?}", Dot::with_config(&c.graph, &[Config::EdgeNoLabel]));
            println!("{}", output);
        }
        Err(_) => {
            println!("ERROR");
        }
    }
}
