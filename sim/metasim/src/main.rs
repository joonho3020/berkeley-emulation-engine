use clap::Parser;
use bee::common::config::Args;
use fpgatopsim::start_test;

fn main() {
    let args = Args::parse();
    match start_test(&args) {
        Ok(_) => { println!("Test Success!"); }
        Err(emsg) => { println!("Test Failed {:?}", emsg); }
    }
}
