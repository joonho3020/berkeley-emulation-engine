use blif_parser::common::*;
use blif_parser::fsim::module::*;
use blif_parser::passes::parser;
use blif_parser::passes::runner;
use blif_parser::primitives::Configuration;
use blif_parser::rtlsim::ref_rtlsim_testharness::*;
use blif_parser::rtlsim::rtlsim_utils::*;
use blif_parser::rtlsim::vcdparser::*;
use blif_parser::utils;
use indexmap::IndexMap;
use std::cmp::max;
use std::{env, fs};
use std::process::Command;

#[derive(Debug, PartialEq)]
enum ReturnCode {
    TestSuccess,
    TestFailed,
}

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

    let _ = test_emulator(sv_file_path, top_mod, input_stimuli_path, blif_file_path);

    Ok(())
}

fn test_emulator(
    sv_file_path: &str,
    top_mod: &str,
    input_stimuli_path: &str,
    blif_file_path: &str,
) -> std::io::Result<ReturnCode> {
    let sim_dir = format!("sim-dir-{}", top_mod);
    let mut cwd = env::current_dir()?;
    cwd.push(sim_dir.clone());
    Command::new("mkdir")
        .arg(&cwd)
        .status()?;

    let res = parser::parse_blif_file(&blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };
    println!("parsing blif file done");

    circuit.set_cfg(Configuration {
        max_steps: 16,
        module_sz: 8,
        lut_inputs: 3,
        network_lat: 0,
        compute_lat: 0
    });
    runner::run_compiler_passes(&mut circuit);
    let verilog_str = match fs::read_to_string(sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(std::io::Error::other(format!(
                "Error while parsing:\n{}",
                e
            )));
        }
    };
    utils::write_string_to_file(
        format!("{:?}", &circuit),
        &format!("{}/{}.dot", cwd.to_str().unwrap(), top_mod),
    )?;
    println!("Compiler passes finished");

    // convert input stimuli to bit-blasted input stimuli
    let ports = get_io(verilog_str.to_string(), top_mod.to_string());
    let input_stimuli = get_input_stimuli(input_stimuli_path);
    let input_stimuli_blasted = bitblast_input_stimuli(&input_stimuli, &ports);

    // bit-blasted output stimuli
    let output_ports = ports
        .iter()
        .filter(|x| !x.input)
        .map(|x| x.clone())
        .collect();
    let output_ports_blasted = bitblasted_port_names(&output_ports);
    let mut output_blasted: IndexMap<String, Vec<u64>> = IndexMap::new();
    for opb in output_ports_blasted.iter() {
        output_blasted.insert(opb.to_string(), vec![]);
    }

    // Run reference RTL simulation
    let sim_output_file = format!("{}-simulation.out", top_mod);
    run_rtl_simulation(
        sv_file_path,
        top_mod,
        input_stimuli_path,
        &sim_dir,
        &sim_output_file,
    )?;
    println!("Reference RTL simulation finished");

    let mut waveform_path = sim_dir.clone();
    waveform_path.push_str("/build/sim.vcd");
    let mut waveform_db = WaveformDB::new(waveform_path);

    let mut module = Module::from_circuit(&circuit);
    let mut module_lag = Module::from_circuit(&circuit);

    let cycles = input_stimuli_blasted
        .values()
        .fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        // poke inputs
        for key in input_stimuli_blasted.keys() {
            let val = input_stimuli_blasted[key].get(cycle);
            match val {
                Some(b) => {
                    let _ = module.poke(key.to_string(), *b as Bit);
                    let _ = module_lag.poke(key.to_string(), *b as Bit);
                }
                None => {}
            }
        }

        // run a cycle
        module.run_cycle();

        let mut found_mismatch = false;

        let ref_signals = waveform_db.signal_values_at_cycle((cycle * 2 + 8) as u32);
        for (signal_name, four_state_bit) in ref_signals.iter() {
            let peek = module.peek(signal_name);
            match (peek, four_state_bit.to_bit()) {
                (Some(bit), Some(ref_bit)) => {
                    if bit != ref_bit {
                        found_mismatch = true;
                        println!(
                            "cycle {} signal {} expected {} get {}",
                            cycle, signal_name, ref_bit, bit
                        );

                        match module.nodeindex(signal_name) {
                            Some(nodeidx) => {
                                utils::write_string_to_file(
                                    circuit.debug_graph(nodeidx, &module),
                                    &format!(
                                        "{}/after-cycle-{}-signal-{}.dot",
                                        cwd.to_str().unwrap(),
                                        cycle,
                                        signal_name
                                    ),
                                )?;

                                utils::write_string_to_file(
                                    circuit.debug_graph(nodeidx, &module_lag),
                                    &format!(
                                        "{}/before-cycle-{}-signal-{}.dot",
                                        cwd.to_str().unwrap(),
                                        cycle,
                                        signal_name
                                    ),
                                )?;
                            }
                            None => {}
                        }
                    }
                }
                _ => {}
            }
        }

        if found_mismatch {
            println!("Test failed");
            return Ok(ReturnCode::TestFailed);
        }

        module_lag.run_cycle();

        for opb in output_ports_blasted.iter() {
            let output = module.peek(opb).unwrap();
            output_blasted.get_mut(opb).unwrap().push(output as u64);
        }
    }

    circuit.save_emulator_instructions(format!("{}/instructions", cwd.to_str().unwrap()))?;

    utils::write_string_to_file(
        output_value_fmt(&aggregate_bitblasted_values(&ports, &mut output_blasted)),
        &format!("{}/{}-emulation.out", cwd.to_str().unwrap(), top_mod),
    )?;
    println!("Test success!");

    return Ok(ReturnCode::TestSuccess);
}

#[cfg(test)]
pub mod emulation_tester {
    use super::*;

    fn perform_test(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
    ) -> bool {
        let ret = test_emulator(sv_file_path, top_mod, input_stimuli_path, blif_file_path);
        match ret {
            Ok(rc) => {
                return rc == ReturnCode::TestSuccess;
            }
            Err(_) => {
                return false;
            }
        }
    }

//    #[test]
//    pub fn test_fir() {
//        assert_eq!(
//            perform_test(
//                "../examples/Fir.sv",
//                "Fir",
//                "../examples/Fir.input",
//                "../examples/Fir.lut.blif"
//            ),
//            true
//        );
//    }
//
//    #[test]
//    pub fn test_gcd() {
//        assert_eq!(
//            perform_test(
//                "../examples/GCD.sv",
//                "GCD",
//                "../examples/GCD.input",
//                "../examples/GCD-2bit.lut.blif"
//            ),
//            true
//        );
//    }
//
//    #[test]
//    pub fn test_queue() {
//        assert_eq!(
//            perform_test(
//                "../examples/MyQueue.sv",
//                "MyQueue",
//                "../examples/MyQueue.input",
//                "../examples/MyQueue.lut.blif"
//            ),
//            true
//        );
//    }

    #[test]
    pub fn test_adder() {
        assert_eq!(
            perform_test(
                "../examples/Adder.sv",
                "Adder",
                "../examples/Adder.input",
                "../examples/Adder.lut.blif"
            ),
            true
        );
    }
}
