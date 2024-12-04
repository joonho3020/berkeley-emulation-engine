use indexmap::IndexMap;
use std::cmp::max;
use indicatif::ProgressBar;

use crate::common::primitive::*;
use crate::common::config::*;
use crate::common::circuit::*;
use crate::common::utils::save_graph_pdf;
use crate::fsim::board::*;
use crate::rtlsim::rtlsim_utils::*;
use crate::rtlsim::vcdparser::*;
use crate::rtlsim::ref_rtlsim_testharness::*;

use super::try_new_circuit;

#[derive(Debug, PartialEq)]
pub enum ReturnCode {
    TestSuccess,
    TestFailed,
}

fn compare_signals(
    circuit: &mut Circuit,
    waveform_db: &mut WaveformDB,
    board: &mut Board,
    board_lag: &mut Board,
    input_stimuli_by_step: &IndexMap<u32, Vec<(&str, Bit)>>,
    args: &Args,
    cycle: usize,
) -> std::io::Result<ReturnCode> {
    let cwd = circuit.compiler_cfg.output_dir.clone();

    // Compare the emulated signals with the reference RTL simulation
    let mut at_least_one_compare = false;
    let mut found_mismatch = false;

    let offset = if args.clock_start_low { 1 } else { 0 };
    let waveform_time = (args.timesteps_per_cycle as usize) * (cycle + args.ref_skip_cycles as usize) + offset;
    let ref_signals = waveform_db.signal_values_at_cycle_rebase_top(waveform_time as u32, args.instance_path.clone());

    for (signal_name, four_state_bit) in ref_signals.iter() {
        if is_clock_signal(&signal_name) || is_clock_tap(&signal_name) {
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
                                &circuit.debug_graph(nodeidx, &board, &ref_signals),
                                &format!("{}/after-cycle-{}-signal-{}.dot",
                                         cwd, cycle, signal_name),
                                &format!("{}/after-cycle-{}-signal-{}.pdf",
                                         cwd, cycle, signal_name))?;
                            save_graph_pdf(
                                &circuit.debug_graph(nodeidx, &board_lag, &ref_signals),
                                &format!("{}/before-cycle-{}-signal-{}.dot",
                                         cwd, cycle, signal_name),
                                &format!("{}/before-cycle-{}-signal-{}.pdf",
                                         cwd, cycle, signal_name))?;
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

    if !at_least_one_compare {
        println!("WARNING, no signals compared at cycle {}", cycle);
    }

    if found_mismatch {
        board_lag.run_cycle_verbose(&input_stimuli_by_step, &(cycle as u32));
        return Ok(ReturnCode::TestFailed);
    }

    return Ok(ReturnCode::TestSuccess);
}

fn run_test_cycle(
    circuit: &mut Circuit,
    board: &mut Board,
    board_lag: &mut Board,
    waveform_db: &mut WaveformDB,
    args: &Args,
    has_reset: &bool,
    input_stimuli_by_step: &IndexMap<u32, Vec<(&str, Bit)>>,
    cycle: usize,
) -> std::io::Result<ReturnCode> {
    // run a cycle
    if args.verbose {
        board.run_cycle_verbose(&input_stimuli_by_step, &(cycle as u32));
    } else {
        board.run_cycle(&input_stimuli_by_step);
    }

    let check_period = (cycle as u32) % args.check_cycle_period == 0;

    if (cycle as u32) < args.no_check_cycles || *has_reset || !check_period {
        board_lag.run_cycle(&input_stimuli_by_step);
    } else {
        let rc = compare_signals(circuit, waveform_db, board, board_lag, input_stimuli_by_step, args, cycle);
        match rc {
            Ok(ReturnCode::TestSuccess) => {
                board_lag.run_cycle(&input_stimuli_by_step);
            }
            Ok(ReturnCode::TestFailed) => {
                println!("input: {:#?}", input_stimuli_by_step);
                board_lag.run_cycle_verbose(&input_stimuli_by_step, &(cycle as u32));
                println!("Test failed");
            }
            Err(..) => { }
        }
        return rc;
    }
    return Ok(ReturnCode::TestSuccess);
}

fn run_test(
    circuit: &mut Circuit,
    board: &mut Board,
    board_lag: &mut Board,
    input_stimuli_blasted: &InputStimuliMap,
    waveform_db: &mut WaveformDB,
    args: &Args
) -> std::io::Result<ReturnCode> {
    let cycles = input_stimuli_blasted.values().fold(0, |x, y| max(x, y.len()));
    assert!(cycles > 1, "No point in running {}", cycles);

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
                    let step = pc + circuit.platform_cfg.fetch_decode_lat();
                    if input_stimuli_by_step.get(&step) == None {
                        input_stimuli_by_step.insert(step, vec![]);
                    }
                    input_stimuli_by_step.get_mut(&step).unwrap().push((sig, *bit));
                }
                None => {
                }
            }
        }

        let mut has_reset = false;
        for (s, b) in input_stimuli_by_name.iter() {
            if !is_debug_reset(s) && is_reset_signal(s) && *b > 0 {
                has_reset = true;
                break;
            }
        }

        // Run test cycle
        let rc = run_test_cycle(
            circuit,
            board,
            board_lag,
            waveform_db,
            args,
            &has_reset,
            &input_stimuli_by_step,
            cycle)?;
        match rc {
            ReturnCode::TestFailed => {
                return Ok(rc);
            }
            _ => { }
        }
    }
    bar.finish();
    return Ok(ReturnCode::TestSuccess);
}

pub fn test_emulator(
    args: Args
) -> std::io::Result<ReturnCode> {
    let mut circuit = try_new_circuit(&args)?;

    let out_dir = &circuit.compiler_cfg.output_dir;
    save_graph_pdf(
        &format!("{:?}", circuit.platform_cfg.topology),
        &format!("{}/{}.topology.dot", out_dir, args.top_mod),
        &format!("{}/{}.topology.pdf", out_dir, args.top_mod))?;

// circuit.save_graph("final")?;

    let input_stimuli_blasted = get_input_stimuli_blasted(
        &args.top_mod,
        &args.input_stimuli_path,
        &args.sv_file_path)?;

    let waveform_path = match &args.vcd {
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
                &out_dir,
                &sim_output_file,
            )?;
            println!("Reference RTL simulation finished");

            let mut waveform_path = out_dir.clone();
            waveform_path.push_str("/build/sim.vcd");
            &waveform_path.clone()
        }
    };
    let mut waveform_db = WaveformDB::new(waveform_path);

    let mut board     = Board::from(&circuit);
    let mut board_lag = Board::from(&circuit);

    return run_test(&mut circuit,
        &mut board,
        &mut board_lag,
        &input_stimuli_blasted,
        &mut waveform_db,
        &args);
}
