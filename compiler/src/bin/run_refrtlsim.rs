use blif_parser::rtlsim::ref_rtlsim_testharness;
use blif_parser::utils::*;
use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let sim_dir = format!("sim-dir-{}", args.top_mod);
    let sim_output_file = format!("{}-simulation.out", args.top_mod);

    ref_rtlsim_testharness::run_rtl_simulation(
        &args.sv_file_path,
        &args.top_mod,
        &args.input_stimuli_path,
        &sim_dir,
        &sim_output_file,
    )?;
    Ok(())
}
