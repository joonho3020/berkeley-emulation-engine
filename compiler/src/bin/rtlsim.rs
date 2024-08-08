use blif_parser::rtlsim::fmodeltestharness;
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Usage: cargo run --bin rtlsim -- <sv input path> <top module name> <input stimuli file>");
        return Ok(());
    }

    let sv_file_path = &args[1];
    let top_mod = &args[2];
    let input_stimuli_path = &args[3];
    let sim_dir = format!("sim-dir-{}", top_mod);
    let sim_output_file = format!("{}-simulation.out", top_mod);
    fmodeltestharness::run_rtl_simulation(
        sv_file_path,
        top_mod,
        input_stimuli_path,
        &sim_dir,
        &sim_output_file,
    )?;

    Ok(())
}
