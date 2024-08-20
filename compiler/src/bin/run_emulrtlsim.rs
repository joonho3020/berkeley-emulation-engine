use blif_parser::passes::parser;
use blif_parser::passes::runner;
use blif_parser::primitives::Configuration;
use blif_parser::rtlsim::emul_rtlsim_testharness::generate_emulator_testbench;
use blif_parser::rtlsim::rtlsim_utils::*;
use blif_parser::utils::*;
use std::fs;
use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let verilog_str = match fs::read_to_string(args.sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(std::io::Error::other(format!(
                "Error while parsing:\n{}",
                e
            )));
        }
    };

    // convert input stimuli to bit-blasted input stimuli
    let ports = get_io(verilog_str.to_string(), args.top_mod.to_string());
    let input_stimuli = get_input_stimuli(&args.input_stimuli_path);
    let input_stimuli_blasted = bitblast_input_stimuli(&input_stimuli, &ports);

    let res = parser::parse_blif_file(&args.blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    circuit.set_cfg(Configuration{
        max_steps: 8,
        module_sz: 8,
        ..Configuration::default()
    });
    runner::run_compiler_passes(&mut circuit);
    let tb = generate_emulator_testbench(&input_stimuli_blasted, &circuit);

    circuit.save_emulator_info("emulation-info".to_string())?;
    write_string_to_file(tb, "TestHarness.sv")?;

    Ok(())
}
