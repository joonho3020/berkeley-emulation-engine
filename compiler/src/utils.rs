use std::fs;
use std::io::Write;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// SystemVerilog file path
    #[arg(short, long, default_value = "")]
    pub sv_file_path: String,

    /// Name of the top module
    #[arg(short, long, default_value = "")]
    pub top_mod: String,

    /// Input value file path
    #[arg(short, long, default_value = "")]
    pub input_stimuli_path: String,

    /// Blif file path
    #[arg(short, long, default_value = "")]
    pub blif_file_path: String
}

pub fn write_string_to_file(input: String, file_path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(file_path)?;
    file.write(input.as_bytes())?;
    Ok(())
}
