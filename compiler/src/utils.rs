use std::fs;
use std::process::{Command, Stdio};
use std::io::Write;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

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
    pub blif_file_path: String,

    /// number of modules
    #[arg(long, default_value_t = 1)]
    pub num_mods: u32,

    /// number of processors in a module
    #[arg(long, default_value_t = 8)]
    pub num_procs: u32,

    /// maximum number of instructions per processor
    #[arg(long, default_value_t = 128)]
    pub max_steps: u32,

    /// lut inputs
    #[arg(long, default_value_t = 3)]
    pub lut_inputs: u32,

    /// network latency between procs in a module
    #[arg(long, default_value_t = 0)]
    pub inter_proc_nw_lat: u32,

    /// network latency between modules
    #[arg(long, default_value_t = 0)]
    pub inter_mod_nw_lat: u32,

    /// imem latency
    #[arg(long, default_value_t = 0)]
    pub imem_lat: u32,

    /// dmem rd latency
    #[arg(long, default_value_t = 0)]
    pub dmem_rd_lat: u32, 

    /// dmem wr latency
    #[arg(long, default_value_t = 1)]
    pub dmem_wr_lat: u32,

    /// debug tail length
    #[arg(long, default_value_t = 10)]
    pub dbg_tail_length: u32,

    /// debug tail threshold
    #[arg(long, default_value_t = 5)]
    pub dbg_tail_threshold: u32, 
}

pub fn write_string_to_file(input: String, file_path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(file_path)?;
    file.write(input.as_bytes())?;
    Ok(())
}

pub fn save_graph_pdf(input: &str, dot_file: &str, pdf_file: &str) -> std::io::Result<()> {
    write_string_to_file(
        input.to_string(),
        dot_file)?;

    let file = fs::File::create(pdf_file).unwrap();
    let stdio = Stdio::from(file);
    Command::new("dot")
        .arg(dot_file)
        .arg("-Tpdf")
        .stdout(stdio)
        .status()?;

    Ok(())
}
