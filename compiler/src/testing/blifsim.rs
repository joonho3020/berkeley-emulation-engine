use indexmap::IndexMap;
use std::fs;
use std::cmp::max;
use indicatif::ProgressBar;
use petgraph::graph::NodeIndex;

use crate::common::primitive::*;
use crate::common::config::*;
use crate::common::utils::save_graph_pdf;
use crate::fsim::board::*;
use crate::rtlsim::rtlsim_utils::*;
use crate::rtlsim::blif_sim::*;
use crate::testing::try_new_circuit;

pub fn compare_blif_sim_to_fsim(args: Args) -> std::io::Result<()> {
    let circuit = try_new_circuit(&args)?;
    let input_stimuli_blasted = get_input_stimuli_blasted(
        &args.top_mod,
        &args.input_stimuli_path,
        &args.sv_file_path)?;

    let mut board = Board::from(&circuit);
    let mut bsim  = BlifSimulator::new(circuit.clone(), input_stimuli_blasted.clone());

    let cycles = input_stimuli_blasted.values().fold(0, |x, y| max(x, y.len()));
    assert!(cycles > 1, "No point in running {}", cycles);

    let bar = ProgressBar::new(cycles as u64);
    let mut printed_compared_cnt = false;
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

        let mut has_reset = false;
        for (s, b) in input_stimuli_by_name.iter() {
            if !is_debug_reset(s) && is_reset_signal(s) && *b > 0 {
                has_reset = true;
                break;
            }
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

        // Run emulator & blif simulator
        board.run_cycle(&input_stimuli_by_step);
        bsim.run_cycle();

        if has_reset {
            continue;
        }

        let mut compared_cnt = 0;
        let mut found_mismatch = false;
        let mut mismatch_nodes: Vec<NodeIndex> = vec![];
        for nidx in bsim.circuit.graph.node_indices() {
            let node = bsim.circuit.graph.node_weight(nidx).unwrap();
            let bsim_val = node.info().debug.val;
            let opt_emul_val = board.peek(node.name());
            match opt_emul_val {
                Some(emul_val) => {
                    compared_cnt += 1;
                    if bsim_val != emul_val {
                        if !found_mismatch {
                            println!("========= cycle: {} ==============", cycle);
                            found_mismatch = true;
                        }

                        println!("node: {:?} blif sim val {} emul sim val {}",
                            circuit.graph.node_weight(nidx).unwrap().name(),
                            bsim_val,
                            emul_val);

                        let out_dir = &circuit.compiler_cfg.output_dir;
                        let signal_name = node.name();
                        save_graph_pdf(
                            &&circuit.debug_graph_2(nidx, &board),
                            &format!("{}/after-cycle-{}-signal-{}.dot",
                                     out_dir, cycle, signal_name),
                            &format!("{}/after-cycle-{}-signal-{}.pdf",
                                     out_dir, cycle, signal_name))?;

                        mismatch_nodes.push(nidx);
                    }
                }
                None => {
                }
            }
        }

        if !printed_compared_cnt {
            println!("Compared {} nodes", compared_cnt);
            printed_compared_cnt = true;
        }

        if found_mismatch {
            let outdir = &bsim.circuit.compiler_cfg.output_dir;
            save_graph_pdf(
                &bsim.circuit.print_given_nodes(&mismatch_nodes),
                &format!("{}/after-cycle-{}.blifsim.dot",
                         outdir, cycle),
                &format!("{}/after-cycle-{}.blifsim.pdf",
                         outdir, cycle))?;

            return Err(std::io::Error::other(format!("Simulation mismatch")));
        }
    }
    bar.finish();

    return Ok(());
}
