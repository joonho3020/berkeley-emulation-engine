use bee::common::{config::*, utils::*, primitive::Bit};
use bee::fsim::board::*;
use bee::passes::runner;
use bee::passes::blif_to_circuit::blif_to_circuit;
use bee::rtlsim::ref_rtlsim_testharness::*;
use bee::rtlsim::rtlsim_utils::*;
use bee::rtlsim::vcdparser::*;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use std::cmp::max;
use std::{env, fs};
use std::process::Command;
use clap::Parser;
use indicatif::ProgressBar;

#[derive(Debug, PartialEq)]
enum ReturnCode {
    TestSuccess,
    TestFailed,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let _ = test_emulator(args);
    Ok(())
}

fn test_emulator(
    args: Args
) -> std::io::Result<ReturnCode> {
    let sim_dir = format!("sim-dir-{}", args.top_mod);
    let mut cwd = env::current_dir()?;
    cwd.push(sim_dir.clone());
    Command::new("mkdir").arg(&cwd).status()?;

    println!("Parsing blif file");
    let res = blif_to_circuit(&args.blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    circuit.set_cfg(
        PlatformConfig {
            num_mods:          args.num_mods,
            num_procs:         args.num_procs,
            max_steps:         args.max_steps,
            lut_inputs:        args.lut_inputs,
            inter_proc_nw_lat: args.inter_proc_nw_lat,
            inter_mod_nw_lat:  args.inter_mod_nw_lat,
            imem_lat:          args.imem_lat,
            dmem_rd_lat:       args.dmem_rd_lat,
            dmem_wr_lat:       args.dmem_wr_lat,
            sram_width:        args.sram_width,
            sram_entries:      args.sram_entries,
            sram_rd_ports:     args.sram_rd_ports,
            sram_wr_ports:     args.sram_wr_ports,
            sram_rd_lat:       args.sram_rd_lat,
            sram_wr_lat:       args.sram_wr_lat,
            topology: GlobalNetworkTopology::new(args.num_mods, args.num_procs)
        },
        CompilerConfig {
            top_module: args.top_mod.clone(),
            output_dir: cwd.to_str().unwrap().to_string(),
            dbg_tail_length: args.dbg_tail_length,
            dbg_tail_threshold: args.dbg_tail_threshold,
        }
    );

    println!("Running compiler passes with config: {:#?}", &circuit.platform_cfg);
    runner::run_compiler_passes(&mut circuit);
    println!("Compiler pass finished");

    save_graph_pdf(
        &format!("{:?}", circuit.platform_cfg.topology),
        &format!("{}/{}.topology.dot", cwd.to_str().unwrap(), args.top_mod),
        &format!("{}/{}.topology.pdf", cwd.to_str().unwrap(), args.top_mod))?;

    circuit.save_emulator_instructions()?;
    circuit.save_emulator_sigmap()?;
// circuit.save_graph("final")?;

    let verilog_str = match fs::read_to_string(&args.sv_file_path) {
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

    let waveform_path = match args.vcd {
        Some(vcd) => {
            vcd
        }
        None => {
            // No reference waveform provided: run reference RTL simulation
            let sim_output_file = format!("{}-simulation.out", args.top_mod);
            run_rtl_simulation(
                &args.sv_file_path,
                &args.top_mod,
                &args.input_stimuli_path,
                &sim_dir,
                &sim_output_file,
            )?;
            println!("Reference RTL simulation finished");

            let mut waveform_path = sim_dir.clone();
            waveform_path.push_str("/build/sim.vcd");
            waveform_path
        }
    };
    let mut waveform_db = WaveformDB::new(waveform_path);

    let mut board     = Board::from(&circuit);
    let mut board_lag = Board::from(&circuit);

    let cycles = input_stimuli_blasted.values().fold(0, |x, y| max(x, y.len()));
    assert!(cycles > 1, "No point in running {}", cycles);

    let instance_depth = args.instance_path.split(".").collect_vec().len();

    let check_signals: IndexSet<String> = IndexSet::from_iter(vec![
        "auto_master_out_a_valid".to_string(),
        "io_resp_valid".to_string()
    ]);

    let cond_check_signals: IndexMap<String, String> = IndexMap::from_iter(vec![
        ("auto_master_out_a_bits_address[0]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[1]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[2]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[3]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[4]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[5]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[6]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[7]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[8]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[9]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[10]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[11]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[12]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[13]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[15]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[16]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[17]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[18]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[19]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[20]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[21]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[22]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[23]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[24]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[25]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[26]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[27]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[28]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[29]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[30]".to_string(), "auto_master_out_a_valid".to_string()),
        ("auto_master_out_a_bits_address[31]".to_string(), "auto_master_out_a_valid".to_string()),
        ("io_resp_bits_data[0]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[1]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[2]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[3]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[4]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[5]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[6]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[8]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[9]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[10]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[11]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[12]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[13]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[14]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[15]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[16]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[17]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[18]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[19]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[20]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[21]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[22]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[23]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[24]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[25]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[26]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[27]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[28]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[29]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[30]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_data[31]".to_string(),              "io_resp_valid".to_string()),
        ("io_resp_bits_ae".to_string(),                "io_resp_valid".to_string())
    ]);

    let bar = ProgressBar::new(cycles as u64);
    for cycle in 0..(cycles-1) {
        bar.inc(1);

        // Collect input stimuli for the current cycle by name
        let mut input_stimuli_by_name: IndexMap<String, Bit> = IndexMap::new();
        for key in input_stimuli_blasted.keys() {
            let val = input_stimuli_blasted[key].get(cycle);
            match val {
                Some(b) => input_stimuli_by_name.insert(key.to_string(), *b as Bit),
                None => None
            };
        }

        // Find the step at which the input has to be poked
        // Save that in the input_stimuli_by_step
        let mut input_stimuli_by_step: IndexMap<u32, Vec<(&str, Bit)>> = IndexMap::new();
        for (sig, bit) in input_stimuli_by_name.iter() {
            match board.nodeindex(sig) {
                Some(nidx) => {
                    let pc = circuit.graph.node_weight(nidx).unwrap().info().pc;
                    let step = pc + circuit.platform_cfg.pc_ldm_offset();
                    if input_stimuli_by_step.get(&step) == None {
                        input_stimuli_by_step.insert(step, vec![]);
                    }
                    input_stimuli_by_step.get_mut(&step).unwrap().push((sig, *bit));
                }
                None => {
                }
            }
        }

        // run a cycle
        if args.verbose {
            board.run_cycle_verbose(&input_stimuli_by_step, &(cycle as u32));
        } else {
            board.run_cycle(&input_stimuli_by_step);
        }

        // Compare the emulated signals with the reference RTL simulation
        let mut at_least_one_compare = false;
        let mut found_mismatch = false;

        let offset = if args.clock_start_low { 1 } else { 0 };
        let waveform_time = (args.timesteps_per_cycle as usize) * (cycle + args.ref_skip_cycles as usize) + offset;
        let ref_signals = waveform_db.signal_values_at_cycle(waveform_time as u32);

        // if this cycle has a reset, don't perform comparisons
        let mut has_reset = false;
        for (s, b) in input_stimuli_by_name.iter() {
            if !is_debug_reset(s) && is_reset_signal(s) && *b > 0 {
                has_reset = true;
                break;
            }
        }
        if has_reset {
            continue;
        }

        let mut      check_signal_vals: IndexMap<String, FourStateBit> = IndexMap::new();
        let mut cond_check_signal_vals: IndexMap<String, FourStateBit> = IndexMap::new();
        for (signal_path, four_state_bit) in ref_signals.iter() {
            let name = signal_path.name();
            let mut signal_path = signal_path.hier();

            // Find the signals under interest
            if signal_path.len() >= instance_depth {
                let signal_path_depth = &signal_path[..instance_depth];
                let signal_path_str = signal_path_depth.join(".");
                if signal_path_str == args.instance_path {
                    signal_path.drain(0..instance_depth);
                    signal_path.push(name.clone());
                } else {
                    continue;
                }
            }

            // Ignore certain signals
            let signal_name = signal_path.join(".");
            if is_clock_signal(&signal_name) || is_clock_tap(&signal_name) {
                continue;
            }

            if check_signals.contains(&signal_name) {
                check_signal_vals.insert(signal_name, *four_state_bit);
            } else if cond_check_signals.contains_key(&signal_name) {
                cond_check_signal_vals.insert(signal_name, *four_state_bit);
            }
        }

        for (signal_name, four_state_bit) in check_signal_vals.iter() {
            let peek = board.peek(&signal_name);
            match (peek, four_state_bit.to_bit()) {
                (Some(bit), Some(ref_bit)) => {
                    println!("performing signal check: {} emu: {} ref: {}", signal_name, bit, ref_bit);

                    at_least_one_compare = true;
                    if bit != ref_bit {
                        found_mismatch = true;
                        println!(
                            "cycle {} signal {} expected {} get {}",
                            cycle, signal_name, ref_bit, bit
                        );

                        match board.nodeindex(&signal_name) {
                            Some(nodeidx) => {
                                save_graph_pdf(
                                    &circuit.debug_graph(nodeidx, &board),
                                    &format!("{}/after-cycle-{}-signal-{}.dot",
                                             cwd.to_str().unwrap(), cycle, signal_name),
                                    &format!("{}/after-cycle-{}-signal-{}.pdf",
                                             cwd.to_str().unwrap(), cycle, signal_name))?;
                                save_graph_pdf(
                                    &circuit.debug_graph(nodeidx, &board_lag),
                                    &format!("{}/before-cycle-{}-signal-{}.dot",
                                             cwd.to_str().unwrap(), cycle, signal_name),
                                    &format!("{}/before-cycle-{}-signal-{}.pdf",
                                             cwd.to_str().unwrap(), cycle, signal_name))?;
                            }
                            None => {}
                        }
                        if args.verbose {
                            println!("============= Sig Map ================");
                            board.print_sigmap();
                        }
                    }
                }
                _ => {}
            }
        }

        for (signal_name, four_state_bit) in cond_check_signal_vals.iter() {
            let cond_signal = cond_check_signals.get(signal_name).unwrap();
            let cond = match check_signal_vals.get(cond_signal).unwrap().to_bit() {
                Some(bit) => {
                    bit == 1
                }
                None => {
                    false
                }
            };

            if !cond {
                continue;
            }

            let peek = board.peek(&signal_name);
            match (peek, four_state_bit.to_bit()) {
                (Some(bit), Some(ref_bit)) => {
                    at_least_one_compare = true;
                    if bit != ref_bit {
                        found_mismatch = true;
                        println!(
                            "cycle {} signal {} expected {} get {}",
                            cycle, signal_name, ref_bit, bit
                        );

                        match board.nodeindex(&signal_name) {
                            Some(nodeidx) => {
                                save_graph_pdf(
                                    &circuit.debug_graph(nodeidx, &board),
                                    &format!("{}/after-cycle-{}-signal-{}.dot",
                                        cwd.to_str().unwrap(), cycle, signal_name),
                                        &format!("{}/after-cycle-{}-signal-{}.pdf",
                                            cwd.to_str().unwrap(), cycle, signal_name))?;
                                save_graph_pdf(
                                    &circuit.debug_graph(nodeidx, &board_lag),
                                    &format!("{}/before-cycle-{}-signal-{}.dot",
                                        cwd.to_str().unwrap(), cycle, signal_name),
                                        &format!("{}/before-cycle-{}-signal-{}.pdf",
                                            cwd.to_str().unwrap(), cycle, signal_name))?;
                            }
                            None => {}
                        }
                        if args.verbose {
                            println!("============= Sig Map ================");
                            board.print_sigmap();
                        }
                    }
                }
                _ => {}
            }
        }



        if !has_reset && !at_least_one_compare {
            println!("WARNING, no signals are getting compared");
        }

        if found_mismatch {
            println!("input: {:#?}", input_stimuli_by_step);
            board_lag.run_cycle_verbose(&input_stimuli_by_step, &(cycle as u32));
            println!("Test failed");
            return Ok(ReturnCode::TestFailed);
        }

        board_lag.run_cycle(&input_stimuli_by_step);

        for opb in output_ports_blasted.iter() {
            let output = board.peek(opb).unwrap();
            output_blasted.get_mut(opb).unwrap().push(output as u64);
        }
    }
    bar.finish();

    write_string_to_file(
        output_value_fmt(&aggregate_bitblasted_values(&ports, &mut output_blasted)),
        &format!("{}/{}-emulation.out", cwd.to_str().unwrap(), args.top_mod),
    )?;
    println!("Test success!");

    return Ok(ReturnCode::TestSuccess);
}

#[cfg(test)]
pub mod emulation_tester {
    use test_case::test_case;
    use super::*;

    fn perform_test(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
        num_mods: u32,
        num_procs: u32,
        inter_proc_nw_lat: u32,
        inter_mod_nw_lat: u32,
        imem_lat: u32,
        dmem_rd_lat: u32,
        dmem_wr_lat: u32,
    ) -> bool {
        let ret = test_emulator(Args {
            verbose:            false,
            sv_file_path:       sv_file_path.to_string(),
            top_mod:            top_mod.to_string(),
            input_stimuli_path: input_stimuli_path.to_string(),
            blif_file_path:     blif_file_path.to_string(),
            vcd:                None,
            instance_path:      "testharness.top".to_string(),
            clock_start_low:    false,
            timesteps_per_cycle: 2,
            ref_skip_cycles:    4,
            num_mods:           num_mods,
            num_procs:          num_procs,
            max_steps:          65536,
            lut_inputs:         3,
            inter_proc_nw_lat:  inter_proc_nw_lat,
            inter_mod_nw_lat:   inter_mod_nw_lat,
            imem_lat:           imem_lat,
            dmem_rd_lat:        dmem_rd_lat,
            dmem_wr_lat:        dmem_wr_lat,
            sram_width:         128,
            sram_entries:       1024,
            sram_rd_ports:      1,
            sram_wr_ports:      1,
            sram_rd_lat:        1,
            sram_wr_lat:        1,
            dbg_tail_length:    u32::MAX, // don't print debug graph when testing
            dbg_tail_threshold: u32::MAX  // don't print debug graph when testing
        });
        match ret {
            Ok(rc) => return rc == ReturnCode::TestSuccess,
            Err(_) => return false
        }
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_adder(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Adder.sv",
                "Adder",
                "../examples/Adder.input",
                "../examples/Adder.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_testreginit(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/TestRegInit.sv",
                "TestRegInit",
                "../examples/TestRegInit.input",
                "../examples/TestRegInit.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 2, 0, 0, 1, 0; "mod 1 procs 2 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_const(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Const.sv",
                "Const",
                "../examples/Const.input",
                "../examples/Const.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 2, 0, 0, 1, 0; "mod 1 procs 2 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_counter(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Counter.sv",
                "Counter",
                "../examples/Counter.input",
                "../examples/Counter.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_gcd(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/GCD.sv",
                "GCD",
                "../examples/GCD.input",
                "../examples/GCD.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_fir(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Fir.sv",
                "Fir",
                "../examples/Fir.input",
                "../examples/Fir.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_myqueue(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/MyQueue.sv",
                "MyQueue",
                "../examples/MyQueue.input",
                "../examples/MyQueue.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 4, 0, 0, 1, 0; "mod 1 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_core(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Core.sv",
                "Core",
                "../examples/Core.input",
                "../examples/Core.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 2, 0, 0, 1, 0; "mod 1 procs 2 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_1r1w_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/OneReadOneWritePortSRAM.sv",
                "OneReadOneWritePortSRAM",
                "../examples/OneReadOneWritePortSRAM.input",
                "../examples/OneReadOneWritePortSRAM.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 2, 0, 0, 1, 0; "mod 1 procs 2 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_1rw_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/SinglePortSRAM.sv",
                "SinglePortSRAM",
                "../examples/SinglePortSRAM.input",
                "../examples/SinglePortSRAM.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(1, 2, 0, 0, 1, 0; "mod 1 procs 2 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_pointer_chasing(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/PointerChasing.sv",
                "PointerChasing",
                "../examples/PointerChasing.input",
                "../examples/PointerChasing.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_cache(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Cache.sv",
                "Cache",
                "../examples/Cache.input",
                "../examples/Cache.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }
}
