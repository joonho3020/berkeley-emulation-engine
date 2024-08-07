use blif_parser::passes::parser;
use blif_parser::passes::runner;
use blif_parser::primitives::Configuration;
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run --bin instgen -- <blif file> <output file>");
        return Ok(());
    }
    let blif_file_path = &args[1];
    let output_file = &args[2];
    let res = parser::parse_blif_file(&blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };
    println!("parsing blif file done");

    let cfg = Configuration {
        gates_per_partition: 128,
        network_latency: 1,
    };
    circuit.set_cfg(cfg);

    runner::run_compiler_passes(&mut circuit);
    circuit.save_emulator_info(output_file.to_string())?;
    Ok(())
}
