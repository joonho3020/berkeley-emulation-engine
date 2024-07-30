mod fsim;
mod instruction;
mod passes;
mod primitives;
mod rtlsim;

use crate::rtlsim::testbench::*;
use crate::fsim::common::*;
use crate::fsim::module::*;
use crate::passes::parser;
use crate::passes::runner;
use crate::primitives::Configuration;
use indexmap::IndexMap;
use std::cmp::max;
use std::fs;
use std::env;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!("Usage: cargo run --bin blif-parser -- <sv input path> <top module name> <input stimuli file> <blif input path>");
        return Ok(());
    }

    let sv_file_path = &args[1];
    let top_mod = &args[2];
    let input_stimuli = get_input_stimuli(&args[3]);
    let file_path = &args[4];

    let res = parser::parse_blif_file(&file_path);
    let mut parsed_circuit = match res {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            return Err(e);
        }
    };

    let cfg = Configuration {
        gates_per_partition: 128,
        network_latency: 1,
    };
    parsed_circuit.set_cfg(cfg);

    let mapped_circuit = runner::run_compiler_passes(parsed_circuit);
    let verilog_str = match fs::read_to_string(sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Error while parsing:\n{}", e).to_string());
        }
    };

    // convert input stimuli to bit-blasted input stimuli
    let ports = get_io(verilog_str.to_string(), top_mod.to_string());
    let input_stimuli_blasted = bitblast_input_stimuli(&input_stimuli, &ports);

    // bit-blasted output stimuli
    let output_ports = ports.iter().filter(|x| !x.input).map(|x| x.clone()).collect();
    let output_ports_blasted = bitblasted_port_names(&output_ports);
    let mut output_blasted: IndexMap<String, Vec<u64>> = IndexMap::new();
    for opb in output_ports_blasted.iter() {
        output_blasted.insert(opb.to_string(), vec![]);
    }

    let mut module = Module::from_circuit(mapped_circuit);

    let cycles = input_stimuli_blasted.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        // poke inputs
        for key in input_stimuli_blasted.keys() {
            let val = input_stimuli_blasted[key].get(cycle);
            match val {
                Some(b) => {
                    module.poke(key.to_string(), *b as Bit)?;
                }
                None => {}
            }
        }

        // run a cycle
        module.run_cycle();
        println!("outputs: {:?}", module.get_outputs());

        for opb in output_ports_blasted.iter() {
            let output = module.peek(opb).unwrap();
            output_blasted.get_mut(opb).unwrap().push(output as u64);
        }
    }
    let output_values = aggregate_bitblasted_values(&ports, &mut output_blasted);
    println!("{:?}", output_values);


    return Ok(());
}
