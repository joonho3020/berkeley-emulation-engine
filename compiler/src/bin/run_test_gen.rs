use bee::rtlsim::rtlsim_utils::{generate_random_test_data, get_io};
use clap::Parser;
use std::fs;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct TestGenArgs {
    /// SystemVerilog file path
    #[arg(short, long)]
    pub sv_file_path: String,

    /// Name of the top module
    #[arg(short, long)]
    pub top_mod: String,

    /// Number of cycles
    #[arg(long, default_value_t = 50)]
    pub cycles: u32,
}

fn main() -> std::io::Result<()> {
    let args = TestGenArgs::parse();
    let verilog_str = match fs::read_to_string(&args.sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(std::io::Error::other(format!(
                "Error while parsing:\n{}",
                e
            )));
        }
    };

    let ports = get_io(verilog_str.to_string(), args.top_mod.to_string());
    let output_file = format!("{}.input", args.top_mod);
    generate_random_test_data(&output_file, &ports, args.cycles)?;
    Ok(())
}
