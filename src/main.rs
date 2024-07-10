
use std::{
    error::Error,
    env,
};

mod primitives;
mod parser;
use crate::parser::parse_blif_file;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let res = parse_blif_file(&file_path);
    match res {
        Ok(c) => {
            println!("Parsing blif file succeeded");
// println!("Circuit: {:?}", c);
        }
        Err(err) => {
            println!("blif file parsing error:\n{}", err);
        }
    }

// println!("modules\n{:?}", modules);

    Ok(())
}
