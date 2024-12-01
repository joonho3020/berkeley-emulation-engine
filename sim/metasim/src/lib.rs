pub mod dut;
pub mod dut_if;
pub mod sim;
pub mod axi;
pub mod simif;
use bee::{
    common::{
        circuit::Circuit,
        config::Args,
        hwgraph::NodeMapInfo, instruction::*,
        mapping::{SRAMMapping, SRAMPortType},
        network::Coordinate,
        primitive::{Bit, Primitive}
    },
    fsim::board::Board,
    rtlsim::rtlsim_utils::{
        get_input_stimuli_blasted,
        InputStimuliMap
    },
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
use axi::*;
use sim::*;
use simif::simif::*;
use simif::mmioif::*;
use simif::dmaif::*;

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

pub fn get_input_stimuli_by_step<'a>(
    circuit: &'a Circuit,
    input_stimuli_blasted: &'a InputStimuliMap,
    signal_map: &IndexMap<String, NodeMapInfo>,
    cycle: u32
) -> IndexMap<u32, Vec<(&'a str, Bit)>> {
    // Collect input stimuli for the current cycle by name
    let mut input_stimuli_by_name: IndexMap<&str, Bit> = IndexMap::new();
    for key in input_stimuli_blasted.keys() {
        let val = input_stimuli_blasted[key].get(cycle as usize);
        match val {
            Some(b) => input_stimuli_by_name.insert(key, *b as Bit),
            None => None
        };
    }

    // Find the step at which the input has to be poked
    // Save that in the input_stimuli_by_step
    let mut input_stimuli_by_step: IndexMap<u32, Vec<(&str, Bit)>> = IndexMap::new();
    for (sig, bit) in input_stimuli_by_name.iter() {
        match signal_map.get(*sig) {
            Some(nmap) => {
                let pc = circuit.graph.node_weight(nmap.idx).unwrap().info().pc;
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
    return input_stimuli_by_step;
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
// let coord = all_signal_map.get(sig).unwrap().info.coord;
// mapped_input_stimulti_blasted.insert(coord, VecDeque::from(stim.clone()));
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
        for _ in 0..5 {
            sim.step();
        }
        poke_reset(sim.dut, 0);
        for _ in 0..5 {
            sim.step();
        }

        println!("Reset done");

        println!("Poke invalid memory address: this should not hang the simulation");
        mmio_write(&mut sim, 0x1000, 0xdeadcafe);
        mmio_read(&mut sim,  0x1000);

        mmio_write(&mut sim, 0x2000, 0xdeadcafe);
        mmio_read(&mut sim,  0x2000);


        let mut driver = Driver::try_from_simif(Box::new(sim));

        // Custom reset
        println!("Set custom resetn to low");
        driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 0)?;
        for _i in 0..10 {
            driver.simif.step();
        }

        println!("Set custom resetn to high");
        driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 1)?;
        for _i in 0..10 {
            driver.simif.step();
        }

        let pcs_are_zero = driver.ctrl_bridge.pcs_are_zero.read(&mut driver.simif)?;
        assert!(pcs_are_zero == (1 << circuit.platform_cfg.num_mods) - 1,
            "All PC values should be initialized after reset {:x}", pcs_are_zero);

        println!("Testing MMIO fingerprint");
        let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
        assert!(fgr_init == 0xf00dcafe,
            "mmio fingerprint mismatch, expect 0xf00dcafe got {}", fgr_init);

        println!("Write to MMIO fingerprint");
        driver.ctrl_bridge.fingerprint.write(&mut driver.simif, 0xdeadcafe)?;
        let fgr_read = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;

        assert!(fgr_read == 0xdeadcafe,
            "mmio fingerprint mismatch, expect {:x} got {:x}", 0xdeadcafeu32, fgr_read);

        println!("Set custom resetn to low");
        driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 0)?;
        for _i in 0..10 {
            driver.simif.step();
        }

        println!("Set custom resetn to high");
        driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 1)?;
        for _i in 0..10 {
            driver.simif.step();
        }

        println!("Read MMIO fingerprint again after reset");
        let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
        assert!(fgr_init == 0xf00dcafe,
            "mmio fingerprint mismatch, expect 0xf00dcafe got {}", fgr_init);

        println!("Start testing the DMA interface");
        let pattern: Vec<u8> = vec![0xd, 0xe, 0xa, 0xd, 0xc, 0xa, 0xf, 0xe];
        let mut data: Vec<u8> = vec![];
        data.extend(pattern.iter().cycle().take(io_stream_bytes as usize));

        let wbytes = driver.dma_bridge.push(&mut driver.simif, &data)?;
        assert!(wbytes == data.len() as u32, "write failed");

        for _ in 0..20 {
            driver.simif.step();
        }

        let mut rdata: Vec<u8> = vec![0u8; data.len()];
        let rbytes = driver.dma_bridge.pull(&mut driver.simif, &mut rdata)?;

        assert!(rbytes == data.len() as u32, "read failed, rbytes: {}, expected: {}", rbytes, data.len());
        assert!(data == rdata, "DMA read {:X?}\nexpect   {:X?}", rdata, data);


        println!("Start configuration register setup");

        for (m, sram_cfg) in sram_cfgs.iter() {
            let single_port_sram = match sram_cfg.port_type {
                SRAMPortType::SinglePortSRAM     => { true }
                SRAMPortType::OneRdOneWrPortSRAM => { false }
            };
            let sram_mmios: &SRAMConfig = driver.ctrl_bridge.sram.get(*m as usize).unwrap();

            println!("Module {} SRAM config {:?}", m, sram_cfg);

            sram_mmios.ptype.write(&mut driver.simif, single_port_sram as u32)?;
            for _ in 0..5 {
                driver.simif.step();
            }

            sram_mmios.mask.write(&mut driver.simif, sram_cfg.wmask_bits as u32)?;
            for _ in 0..5 {
                driver.simif.step();
            }

            sram_mmios.width.write(&mut driver.simif, sram_cfg.width_bits as u32)?;
            for _ in 0..5 {
                driver.simif.step();
            }
        }

        driver.ctrl_bridge.host_steps.write(&mut driver.simif, host_steps)?;
        driver.simif.step();

        // Check that the host_step change checking logic works
        driver.ctrl_bridge.host_steps.write(&mut driver.simif, host_steps + 1)?;
        driver.simif.step();

        let host_steps_changed =
            driver.ctrl_bridge.host_steps_prv_cnt.read(&mut driver.simif)?;

        println!("host_steps_changed: {}", host_steps_changed);

        if host_steps_changed != 1 {
            let deq_cnt = driver.ctrl_bridge.host_steps_cur_cnt.read(&mut driver.simif)?;
            println!("host_steps_prv_deq {} entries, cur_deq {} entries",
                host_steps_changed, deq_cnt);

            for _ in 0..host_steps_changed {
                println!("prv {} -> cur {}",
                    driver.ctrl_bridge.host_steps_prv_deq.read(&mut driver.simif)?,
                    driver.ctrl_bridge.host_steps_cur_deq.read(&mut driver.simif)?);
            }

            println!("host_steps should only change once, changed {} times", host_steps_changed);
        }

        // Set the host_step back to a correct value
        driver.ctrl_bridge.host_steps.write(&mut driver.simif, host_steps)?;
        driver.simif.step();

        println!("Start pushing instructions");

        let inst_bar = ProgressBar::new(module_insts.len() as u64);
        for (_m, insts) in module_insts.iter() {
            inst_bar.inc(1);

            println!("Total pushed instructions: {}",
                driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?);

            for (i, inst) in insts.iter().enumerate() {
                let _p = i as u32 / host_steps;
                let mut bitbuf = inst.to_bits(&circuit.platform_cfg);
                assert!(bitbuf.len() < 8 * 8, "Instruction bits {} > 64", bitbuf.len());

                for x in 0..circuit.platform_cfg.num_proc_bits() {
                    let sl = circuit.platform_cfg.num_proc_bits() - x - 1;
                    bitbuf.push((_p >> sl) & 1 == 1);
                }
                for x in 0..circuit.platform_cfg.num_mod_bits() {
                    let sl = circuit.platform_cfg.num_mod_bits() - x - 1;
                    bitbuf.push((_m >> sl) & 1 == 1);
                }

                bitbuf.reverse();

                assert!(bitbuf.len() as u32 <= fpga_top_cfg.axi.beat_bytes() * 8,
                    "Instruction + procidx + modidx bits {} > 512", bitbuf.len());

                let mut bytebuf: Vec<u8> = bitbuf
                                            .into_vec()
                                            .iter()
                                            .flat_map(|&x| x.to_le_bytes())
                                            .rev()
                                            .collect();
                bytebuf.reverse();
                bytebuf.resize(fpga_top_cfg.axi.beat_bytes() as usize, 0);
                driver.inst_bridge.push(&mut driver.simif, &bytebuf)?;

                for _ in 0..2 {
                    driver.simif.step();
                }

                // Check that the logic for midx validation works
                let midx_mismatch_cnt = driver.ctrl_bridge.midx_mismatch_cnt.read(&mut driver.simif)?;
                for _ in 0..midx_mismatch_cnt {
                    println!("midx_mismatch found: received midx {}",
                        driver.ctrl_bridge.midx_mismatch_deq.read(&mut driver.simif)?);
                }

                // Check that the logic for pidx validation works
                let pidx_mismatch_cnt = driver.ctrl_bridge.pidx_mismatch_cnt.read(&mut driver.simif)?;
                for _ in 0..pidx_mismatch_cnt {
                    println!("pidx_mismatch found: received pidx {}",
                        driver.ctrl_bridge.pidx_mismatch_deq.read(&mut driver.simif)?);
                }
            }

            // Check if all processor 0 & processor n-1 have been initialized
            let proc_0_init_vec = driver.ctrl_bridge.dbg_proc_0_init.read(&mut driver.simif)?;
            let proc_n_init_vec = driver.ctrl_bridge.dbg_proc_n_init.read(&mut driver.simif)?;
            assert!(proc_0_init_vec == proc_n_init_vec,
                "proc 0 {:x} n {:x}",
                proc_0_init_vec, proc_n_init_vec);

            // Check that the number of processors initialized processors match w/
            // what is expected
            let dbg_init_cntr = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
            assert!(dbg_init_cntr.read(&mut driver.simif)? == circuit.platform_cfg.num_procs);
        }
        inst_bar.finish();


        // Wait until initialization is finished
        while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
            driver.simif.step();
        }

        println!("proc_0_init_vec: {:x}",
            driver.ctrl_bridge.dbg_proc_0_init.read(&mut driver.simif)?);

        println!("proc_n_init_vec: {:x}",
            driver.ctrl_bridge.dbg_proc_n_init.read(&mut driver.simif)?);

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
// println!("rtl_state_vec: {:?}", rtl_state_vec);
// println!("rtl_state_bit_vec: {:?}", rtl_state_bit_vec);
// println!("     rtl_state: {:?}", rtl_state);
// println!("    fsim_state: {:?}", fsim_state);
                }
            }


// funct_sim.run_cycle(&input_stimuli_by_step);

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
