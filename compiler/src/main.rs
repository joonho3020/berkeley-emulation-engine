mod common;
mod fsim;
mod passes;
mod primitives;
mod rtlsim;

use crate::common::*;
use crate::fsim::module::*;
use crate::passes::parser;
use crate::passes::runner;
use crate::primitives::Configuration;
use crate::rtlsim::testbench::*;
use crate::rtlsim::vcdparser::*;
use indexmap::IndexMap;
use std::cmp::max;
use std::io::Write;
use std::{env, fs};

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

    let res = parser::parse_blif_file(&blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    let cfg = Configuration {
        gates_per_partition: 128,
        network_latency: 1,
    };
    circuit.set_cfg(cfg);

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
    let sim_dir = format!("sim-dir-{}", top_mod);
    let sim_output_file = format!("{}-simulation.out", top_mod);
    run_rtl_simulation(
        sv_file_path,
        top_mod,
        input_stimuli_path,
        &sim_dir,
        &sim_output_file,
    )?;

    let mut cwd = env::current_dir()?;
    cwd.push(sim_dir.clone());

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
                                let debug_graph = circuit.debug_graph(nodeidx, &module);
                                let debug_graph_file =
                                    format!("after-cycle-{}-signal-{}.dot", cycle, signal_name);
                                let mut debug_out_file = fs::File::create(format!(
                                    "{}/{}",
                                    cwd.to_str().unwrap(),
                                    debug_graph_file
                                ))?;
                                debug_out_file.write(debug_graph.as_bytes())?;

                                let debug_graph = circuit.debug_graph(nodeidx, &module_lag);
                                let debug_graph_file =
                                    format!("before-cycle-{}-signal-{}.dot", cycle, signal_name);
                                let mut debug_out_file = fs::File::create(format!(
                                    "{}/{}",
                                    cwd.to_str().unwrap(),
                                    debug_graph_file
                                ))?;
                                debug_out_file.write(debug_graph.as_bytes())?;
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
            return Ok(());
        }

        module_lag.run_cycle();

        for opb in output_ports_blasted.iter() {
            let output = module.peek(opb).unwrap();
            output_blasted.get_mut(opb).unwrap().push(output as u64);
        }
    }
    let output_values = output_value_fmt(&aggregate_bitblasted_values(&ports, &mut output_blasted));

    let emul_output_file = format!("{}-emulation.out", top_mod);
    let mut emulation_out_file =
        fs::File::create(format!("{}/{}", cwd.to_str().unwrap(), emul_output_file))?;
    emulation_out_file.write(output_values.as_bytes())?;

    let mut graph_file = fs::File::create(format!("{}/{}.dot", cwd.to_str().unwrap(), top_mod))?;
    graph_file.write(format!("{:?}", &circuit).as_bytes())?;

    println!("Test success!");

    return Ok(());
}
