use bee::common::config::*;
use bee::compare_blif_sim_to_fsim;
use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    match compare_blif_sim_to_fsim(args) {
        Ok(_) => { println!("Test Success!"); }
        _     => { println!("Test Failed!");  }
    }
    Ok(())
}
