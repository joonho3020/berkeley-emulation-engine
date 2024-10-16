use bee::common::config::*;
use bee::test_emulator;
use bee::ReturnCode;
use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    match test_emulator(args) {
        Ok(ReturnCode::TestSuccess) => {
            println!("Test Success!");
        }
        _ => {
            println!("Test Failed");
        }
    }
    Ok(())
}

