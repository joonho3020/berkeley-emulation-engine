pub mod dut;
pub mod dut_if;
pub mod sim;
pub mod simif;
pub mod driver;
use bee::{
    common::{
        config::Args,
        hwgraph::NodeMapInfo, instruction::*,
        mapping::SRAMMapping,
        network::Coordinate,
        primitive::{Bit, Primitive}
    },
    fsim::board::Board,
    rtlsim::rtlsim_utils::get_input_stimuli_blasted,
    testing::try_new_circuit
};
use indexmap::IndexMap;
use std::{
    collections::VecDeque,
    cmp::max
};
use indicatif::ProgressBar;
use bitvec::{order::Lsb0, vec::BitVec};
use dut::*;
use dut_if::*;
use sim::*;
use simif::{
    simif::*,
    mmioif::*,
    dmaif::*
};
use driver::{
    axi::*,
    driver::*
};

#[derive(Debug)]
pub enum RTLSimError {
    IOError(Box<dyn std::error::Error>),
    SimError(String)
}

impl From<std::io::Error> for RTLSimError {
    fn from(err: std::io::Error) -> RTLSimError {
        RTLSimError::IOError(Box::new(err))
    }
}

impl From<Box<dyn std::error::Error>> for RTLSimError {
    fn from(err: Box<dyn std::error::Error>) -> RTLSimError {
        RTLSimError::IOError(err)
    }
}

impl From<String> for RTLSimError {
    fn from(err: String) -> RTLSimError {
        RTLSimError::SimError(err)
    }
}

pub fn start_test(args: &Args) -> Result<(), RTLSimError> {
    let circuit = try_new_circuit(&args)?;
    let mut funct_sim = Board::from(&circuit);

    // Aggregate per module instructions
    let mut module_insts: IndexMap<u32, VecDeque<Instruction>> = IndexMap::new();
    let mut sram_cfgs: IndexMap<u32, SRAMMapping> = IndexMap::new();
    for (m, mmap) in circuit.emul.module_mappings.iter() {
        let mut insts: VecDeque<Instruction> = VecDeque::new();
        let mut mmap_ = mmap.clone();
        mmap_.proc_mappings.sort_keys();
        for (_, pmap) in mmap_.proc_mappings.iter() {
            insts.extend(pmap.instructions.clone());
        }
        module_insts.insert(*m, insts);
        sram_cfgs.insert(*m, mmap.sram_mapping.clone());
    }

    // Get the input stimuli
    let input_stimuli_blasted =
        get_input_stimuli_blasted(&args.top_mod, &args.input_stimuli_path, &args.sv_file_path)?;

    // Aggregate signal mappings
    let mut all_signal_map: IndexMap<String, NodeMapInfo> = IndexMap::new();
    for (_, mmap) in circuit.emul.module_mappings.iter() {
        for (_, pmap) in mmap.proc_mappings.iter() {
            all_signal_map.extend(pmap.signal_map.clone());
        }
    }

    // Map the input stimuli to a coordinate
    let mut mapped_input_stimulti_blasted: IndexMap<Coordinate, VecDeque<u64>> = IndexMap::new();
    for (sig, stim) in input_stimuli_blasted.iter() {
        match all_signal_map.get(sig) {
            Some(nmi) =>  {
                let coord = nmi.info.coord;
                mapped_input_stimulti_blasted.insert(coord, VecDeque::from(stim.clone()));
            }
            None =>  { println!("Input Signal {} not found", sig); }
        }
    }

    // Total number of target cycles
    let target_cycles = mapped_input_stimulti_blasted.values().fold(0, |x, y| max(x, y.len()));

    let mut output_signals: IndexMap<String, Coordinate> = IndexMap::new();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() == Primitive::Output {
            assert!(all_signal_map.contains_key(node.name()),
                    "Output {} not found in signal map",
                    node.name());
            output_signals.insert(node.name().to_string(), node.info().coord);
        }
    }

    let fpga_top_cfg = FPGATopConfig {
        axi: AXI4Config {
            addr_bits: 64,
            id_bits: 4,
            data_bits: 512
        },
        axil: AXI4Config {
            addr_bits: 64,
            id_bits: 4,
            data_bits: 32
        },
        emul: circuit.platform_cfg.clone()
    };

    let host_steps = circuit.emul.host_steps;
    let mut mismatch_string: Option<String> = None;

    let total_procs = fpga_top_cfg.emul.total_procs();
    let data_bits = fpga_top_cfg.axi.data_bits;
    let io_stream_bits = ((total_procs + data_bits - 1) / data_bits) * data_bits;
    let io_stream_bytes = io_stream_bits / 8;

    println!("total_procs: {}, axi data bits: {}, io_stream_bits: {}",
        total_procs, data_bits, io_stream_bits);

    unsafe {
        let mut sim = Sim::try_new(&fpga_top_cfg);

        poke_reset(sim.dut, 1);
        poke_io_clkwiz_ctrl_axi_aresetn(sim.dut, 0);
        for _ in 0..5 {
            sim.step();
        }

        poke_reset(sim.dut, 0);
        poke_io_clkwiz_ctrl_axi_aresetn(sim.dut, 1);
        for _ in 0..5 {
            sim.step();
        }

        println!("Reset done");

        println!("Poke invalid memory address: this should not hang the simulation");
        mmio_write(&mut sim, 0x1000, 0xdeadcafe);
        mmio_read(&mut sim,  0x1000);

        mmio_write(&mut sim, 0x2000, 0xdeadcafe);
        mmio_read(&mut sim,  0x2000);

        mmio_write(&mut sim, 0x20000, 0xdeadcafe);
        mmio_read(&mut sim,  0x20000);

        // Assume lock is low when starting
        poke_io_clkwiz_ctrl_ctrl_clk_wiz_locked(sim.dut, 0);

        let mut driver = Driver::try_from_simif(Box::new(sim));

        for _ in 0..10 {
            driver.simif.step();
        }


        pll_lock_and_fpga_top_reset(&mut driver)?;
        board_reset(&mut driver, &fpga_top_cfg)?;
        test_dma_bridge(&mut driver, 20, &fpga_top_cfg)?;
        set_target_config_regs(&mut driver, &sram_cfgs, host_steps)?;
        push_instructions(&mut driver, module_insts, host_steps, &fpga_top_cfg)?;

        // Wait until initialization is finished
        while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
            driver.simif.step();
        }

        println!("Start simulation");

        let sim_bar = ProgressBar::new(target_cycles as u64);
        'emulation_loop: for tcycle in 0..target_cycles {
            sim_bar.inc(1);
            let tot_procs = circuit.platform_cfg.total_procs();
            let mut bit_vec: BitVec<usize, Lsb0> = BitVec::new();
            for _ in 0..tot_procs {
                bit_vec.push(false);
            }

            for (coord, stim) in mapped_input_stimulti_blasted.iter_mut() {
                let bit = stim.pop_front().unwrap();
                let id = coord.id(&circuit.platform_cfg);
                bit_vec.set(id as usize, bit != 0);
            }

            let mut ivec: Vec<u8> = bit_vec
                .into_vec()
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect();
            ivec.resize(io_stream_bytes as usize, 0);

            driver.io_bridge.push(&mut driver.simif, &ivec)?;

            let mut ovec = vec![0u8; ivec.len()];
            while true {
                let read_bytes = driver.io_bridge.pull(&mut driver.simif, &mut ovec)?;
                if read_bytes == 0 {
                    driver.simif.step();
                } else {
                    break;
                }
            }

            // functional simulator input setup
            let input_stimuli_by_step = get_input_stimuli_by_step(
                &circuit,
                &input_stimuli_blasted,
                &all_signal_map,
                tcycle as u32);

            let dbg_stream_bits = ((2 * total_procs + data_bits - 1) / data_bits) * data_bits;
            let dbg_stream_bytes = dbg_stream_bits / 8;

            for step in 0..host_steps {
                let mut rtl_state_vec = vec![0u8; dbg_stream_bytes as usize];
                'spin_until_read: while true {
                    let read_bytes = driver.dbg_bridge.pull(&mut driver.simif, &mut rtl_state_vec)?;
                    if read_bytes == 0 {
                        driver.simif.step();
                    } else {
                        break 'spin_until_read;
                    }
                }

                let rtl_state_bit_vec: Vec<bool> = rtl_state_vec
                                                .iter()
                                                .flat_map(|&byte| (0..8).map(move |i| (byte & (1 << i)) != 0))
                                                .collect();

                let mut rtl_state: Vec<Vec<(Bit, Bit)>> = vec![];
                for m in 0..fpga_top_cfg.emul.num_mods {
                    let mut mod_state: Vec<(Bit, Bit)> = vec![];
                    for p in 0..fpga_top_cfg.emul.num_procs {
                        let ldm_idx = (m * fpga_top_cfg.emul.num_procs + p) * 2;
                        let sdm_idx = ldm_idx + 1;
                        mod_state.push((
                            *rtl_state_bit_vec.get(ldm_idx as usize).unwrap() as Bit,
                            *rtl_state_bit_vec.get(sdm_idx as usize).unwrap() as Bit));
                    }
                    rtl_state.push(mod_state);
                }
                let fsim_state = funct_sim.step_with_input(step, &input_stimuli_by_step);
                if rtl_state != fsim_state {
                    println!("    MISMATCH LDM/SDM write bits at step: {}", step);
                    for m in 0..fpga_top_cfg.emul.num_mods {
                        for p in 0..fpga_top_cfg.emul.num_procs {
                            let rtl = rtl_state
                                .get(m as usize).unwrap().get(p as usize).unwrap();
                            let fsim = fsim_state
                                .get(m as usize).unwrap().get(p as usize).unwrap();
                            if rtl != fsim {
                                println!("        Mismatch at module {} proc {} rtl {:?} fsim {:?}",
                                    m, p, rtl, fsim);
                            }

                        }
                    }
                    assert!(false);
                }
            }

            // Collect functional simulation outputs
            let mut obit_ref: BitVec<usize, Lsb0> = BitVec::new();
            for _ in 0..tot_procs {
                obit_ref.push(false);
            }

            for (os, coord) in output_signals.iter() {
                let fsim_bit = funct_sim.peek(os).unwrap_or(0);
                let id = coord.id(&circuit.platform_cfg);
                obit_ref.set(id as usize, fsim_bit != 0);
            }
            let mut ovec_ref: Vec<u8> = obit_ref
                .into_vec()
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect();
            ovec_ref.resize(io_stream_bytes as usize, 0);

            println!("Target cycle finished: {}", tcycle);
            println!("ovec:     {:X?}", ovec);
            println!("ovec_ref: {:X?}", ovec_ref);

            if ovec != ovec_ref {
                println!("MISMATCH");
                println!("ovec:     {:X?}", ovec);
                println!("ovec_ref: {:X?}", ovec_ref);
                mismatch_string = Some(format!(
                        "Target cycle {} mismatch got {:?} expect {:?}",
                        tcycle, ovec, ovec_ref));
                break 'emulation_loop;
            }
        }
        driver.simif.finish();
        sim_bar.finish();
    }
    match mismatch_string {
        Some(emsg) => Err(RTLSimError::from(emsg)),
        None       => Ok(())
    }
}
