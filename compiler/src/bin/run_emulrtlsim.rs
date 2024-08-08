use blif_parser::passes::parser;
use blif_parser::passes::runner;
use blif_parser::primitives::Configuration;
use blif_parser::rtlsim::emul_rtlsim_testharness::generate_emulator_testbench;
use blif_parser::rtlsim::rtlsim_utils::*;
use blif_parser::utils::write_string_to_file;
use std::env;
use std::fs;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!("Usage: cargo run --bin blif-parser -- <sv input path> <top module name> <input stimuli file> <blif input path>");
        return Ok(());
    }

    let sv_file_path = &args[1];
    let top_mod = &args[2];
    let input_stimuli_path = &args[3];
    let blif_file_path = &args[4];

    let verilog_str = match fs::read_to_string(sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(std::io::Error::other(format!(
                "Error while parsing:\n{}",
                e
            )));
        }
    };

    // convert input stimuli to bit-blasted input stimuli
    let ports = get_io(verilog_str.to_string(), top_mod.to_string());
    let input_stimuli = get_input_stimuli(input_stimuli_path);
    let input_stimuli_blasted = bitblast_input_stimuli(&input_stimuli, &ports);

    let res = parser::parse_blif_file(&blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    circuit.set_cfg(Configuration::default());
    runner::run_compiler_passes(&mut circuit);
    let tb = generate_emulator_testbench(&input_stimuli_blasted, &circuit);

    circuit.save_emulator_info("emulation-info".to_string())?;
    write_string_to_file(tb, "TestHarness.sv")?;

    Ok(())
}
