use bee::common::config::*;
use bee::testing::fsim::test_emulator;
use bee::testing::fsim::ReturnCode;
use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    match test_emulator(args) {
        Ok(ReturnCode::TestSuccess) => { println!("Test Success!"); }
        _ => { println!("Test Failed"); }
    }
    Ok(())
}
