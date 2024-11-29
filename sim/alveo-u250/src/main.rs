use clap::Parser;
use xdma_driver::*;
use rand::Rng;
use indicatif::ProgressBar;
use indexmap::IndexMap;
use std::{
    cmp::max, collections::VecDeque, thread::sleep
};
use bee::{
    common::{
        circuit::Circuit,
        config::Args,
        hwgraph::NodeMapInfo, instruction::*,
        mapping::{SRAMMapping, SRAMPortType},
        network::Coordinate,
        primitive::{Primitive, Bit}
    },
    fsim::board::Board,
    rtlsim::rtlsim_utils::{
        get_input_stimuli_blasted,
        InputStimuliMap
    },
    testing::try_new_circuit
};
use bitvec::{order::Lsb0, vec::BitVec};
use simif::simif::*;
use simif::mmioif::*;
use simif::dmaif::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct SimArgs {
    #[arg(long, default_value_t = 0x0000)]
    pub domain: u16,

    #[arg(long, default_value_t = 0x17)]
    pub bus: u8,

    #[arg(long, default_value_t = 0x00)]
    pub dev: u8,

    #[arg(long, default_value_t = 0x0)]
    pub func: u8,

    #[arg(long, default_value_t = 0x10ee)]
    pub pci_vendor: u16,

    #[arg(long, default_value_t = 0x903f)]
    pub pci_device: u16,

    #[clap(flatten)]
    pub bee_args: Args
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

fn main() -> Result<(), SimIfErr> {
    let args = SimArgs::parse();
    let simif = XDMAInterface::try_new(
        args.pci_vendor,
        args.pci_device,
        args.domain,
        args.bus,
        args.dev,
        args.func,
    )?;

    let circuit = try_new_circuit(&args.bee_args)?;
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
        get_input_stimuli_blasted(
            &args.bee_args.top_mod,
            &args.bee_args.input_stimuli_path,
            &args.bee_args.sv_file_path)?;

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
        let coord = all_signal_map.get(sig).unwrap().info.coord;
        mapped_input_stimulti_blasted.insert(coord, VecDeque::from(stim.clone()));
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

    let mut driver = Driver::try_from_simif(Box::new(simif));

    println!("Testing MMIO fingerprint");
    let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
    println!("fgr_init: {:x}", fgr_init);

    driver.ctrl_bridge.fingerprint.write(&mut driver.simif, 0xdeadbeaf)?;
    println!("reading from fingerprint addr: {:x}", driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?);

    let total_procs = args.bee_args.num_mods * args.bee_args.num_procs;
    let data_bits = 512;
    let io_stream_bits = ((total_procs + data_bits - 1) / data_bits) * data_bits;
    let dma_bytes = io_stream_bits / 8;
// let dma_bytes = 64;

    println!("total_procs: {}, axi data bits: {}, io_stream_bits: {}, dma_bytes: {}",
        total_procs, data_bits, io_stream_bits, dma_bytes);

    let mut rng = rand::thread_rng();

    println!("Testing Debug DMA Bridge");
    let iterations = 1000;
    let bar = ProgressBar::new(iterations);
    for _i in 0..iterations {
        bar.inc(1);

        let mut wbuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
        wbuf.extend((0..dma_bytes).map(|_| rng.gen_range(10..16)));
        let written_bytes = driver.dbg_bridge.push(&mut driver.simif, &wbuf)?;
        assert!(written_bytes == dma_bytes,
            "DMA write didn't write expected amount. Wrote: {} out of {} byte, iter {}",
            written_bytes, dma_bytes, _i);

// sleep(std::time::Duration::from_millis(10));

        let mut rbuf = vec![0u8; dma_bytes as usize];
        let read_bytes = driver.dbg_bridge.pull(&mut driver.simif, &mut rbuf)?;
        assert!(read_bytes == dma_bytes, "Read {} bytes, expected read {}, iter {}",
            read_bytes, dma_bytes, _i);

        assert!(wbuf == rbuf, "wbuf: {:X?}\nrbuf: {:X?}\ndiverge at index: {:?}, num diff: {:?}, iter{}",
            wbuf,
            rbuf,
            wbuf.iter()
                .zip(rbuf.iter())
                .enumerate()
                .find(|(_, (a, b))| a != b)
                .map(|(index, _)| index),
            wbuf.iter()
                .zip(rbuf.iter())
                .map(|(a, b)| (a != b) as u32)
                .reduce(|a, b| a + b),
            _i);
    }
    bar.finish();

    println!("Start configuration register setup");


    println!("Setting SRAM config registers");
    for (m, sram_cfg) in sram_cfgs.iter() {
        let single_port_sram = match sram_cfg.port_type {
            SRAMPortType::SinglePortSRAM     => { true }
            SRAMPortType::OneRdOneWrPortSRAM => { false }
        };
        let sram_mmios: &SRAMConfig = driver.ctrl_bridge.sram.get(*m as usize).unwrap();


        sram_mmios.ptype.write(&mut driver.simif, single_port_sram    as u32)?;
        sram_mmios.mask .write(&mut driver.simif, sram_cfg.wmask_bits as u32)?;
        sram_mmios.width.write(&mut driver.simif, sram_cfg.width_bits as u32)?;
    }

    println!("Setting host_steps");
    driver.ctrl_bridge.host_steps.write(&mut driver.simif, circuit.emul.host_steps)?;
    assert!(driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0, "Init set before pushing instructions");

    while driver.ctrl_bridge.host_steps.read(&mut driver.simif)? == 0 {
    }

    println!("host_steps {}", driver.ctrl_bridge.host_steps.read(&mut driver.simif)?);

    sleep(std::time::Duration::from_millis(10));

    println!("Start pushing instructions");
    let inst_bar = ProgressBar::new(module_insts.len() as u64);
    for (_m, insts) in module_insts.iter() {
        inst_bar.inc(1);

        let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
        let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
        assert!(dbg_init_cntr == 0, "There should be no processors that are initialized");

        for (inst_idx, inst) in insts.iter().enumerate() {
            let _p = inst_idx as u32 / circuit.emul.host_steps;

            let mut bitbuf = inst.to_bits(&circuit.platform_cfg);

            assert!(bitbuf.len() < 8 * 8, "Instruction bits {} > 64", bitbuf.len());
            assert!(driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0,
                "Init set while pushing instructions, module {} inst {}", _m, inst_idx);

            for x in 0..circuit.platform_cfg.num_proc_bits() {
                let sl = circuit.platform_cfg.num_proc_bits() - x - 1;
                bitbuf.push((_p >> sl) & 1 == 1);
            }
            for x in 0..circuit.platform_cfg.num_mod_bits() {
                let sl = circuit.platform_cfg.num_mod_bits() - x - 1;
                bitbuf.push((_m >> sl) & 1 == 1);
            }
            bitbuf.reverse();

            assert!(bitbuf.len() < 512);

            let mut bytebuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
            bytebuf.extend(bitbuf
                .clone()
                .into_vec()
                .iter()
                .flat_map(|&x| x.to_le_bytes())
                .rev());
            bytebuf.reverse();
            bytebuf.resize(64 as usize, 0);

            sleep(std::time::Duration::from_millis(2));

            let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
            let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
            if dbg_init_cntr != _p {
                println!("FISHY... Initializing module {} processor {}, initialized count {}",
                    _m, _p, dbg_init_cntr);
            }

            match driver.inst_bridge.push(&mut driver.simif, &bytebuf) {
                Ok(written_bytes) => {
                    if written_bytes == 0 {
                        let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
                        let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
                        println!("init cntrs: {}, module: {} proc: {}", dbg_init_cntr, _m, _p);

                        // Check that the logic for midx validation works
                        let midx_mismatch_cnt = driver.ctrl_bridge.midx_mismatch_cnt.read(&mut driver.simif)?;
                        for _ in 0..midx_mismatch_cnt {
                            println!("midx_mismatch found: received midx {}, expect {}",
                                driver.ctrl_bridge.midx_mismatch_deq.read(&mut driver.simif)?,
                                _m);
                        }

                        // Check that the logic for pidx validation works
                        let pidx_mismatch_cnt = driver.ctrl_bridge.pidx_mismatch_cnt.read(&mut driver.simif)?;
                        for _ in 0..pidx_mismatch_cnt {
                            println!("pidx_mismatch found: received pidx {}, expect {}",
                                driver.ctrl_bridge.pidx_mismatch_deq.read(&mut driver.simif)?,
                                inst_idx);
                        }

                        // Check that the host_steps did not change while pushing the instructions
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

                        // Check if all processor 0 & processor n-1 have been initialized
                        let proc_0_init_vec = driver.ctrl_bridge.dbg_proc_0_init.read(&mut driver.simif)?;
                        let proc_n_init_vec = driver.ctrl_bridge.dbg_proc_n_init.read(&mut driver.simif)?;
                        println!("proc_0_init_vec: {:x} proc_n_init_vec: {:x}", proc_0_init_vec, proc_n_init_vec);

                        assert!(false, "wrote zero bytes");
                        sleep(std::time::Duration::from_millis(1));
                        continue;
                    } else {
                        assert!(written_bytes == 64, "Less than 64 bytes written for instruction");
                    }
                }
                Err(_) => {
                    println!("DMA push panics while pushing instructions");
                    sleep(std::time::Duration::from_millis(1));
                    assert!(driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0,
                        "Init set while pushing instructions");
                }
            }

            // Check that the logic for midx validation works
            let midx_mismatch_cnt = driver.ctrl_bridge.midx_mismatch_cnt.read(&mut driver.simif)?;
            for _ in 0..midx_mismatch_cnt {
                println!("midx_mismatch found: received midx {}, expect {}",
                    driver.ctrl_bridge.midx_mismatch_deq.read(&mut driver.simif)?,
                    _m);
            }

            // Check that the logic for pidx validation works
            let pidx_mismatch_cnt = driver.ctrl_bridge.pidx_mismatch_cnt.read(&mut driver.simif)?;
            for _ in 0..pidx_mismatch_cnt {
                println!("pidx_mismatch found: received pidx {}, expect {}",
                    driver.ctrl_bridge.pidx_mismatch_deq.read(&mut driver.simif)?,
                    inst_idx);
            }
        }
        let tot_insts_pushed = driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?;
        println!("total instructions pushed {} ",
            driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?);
        assert!(tot_insts_pushed == circuit.emul.host_steps * circuit.platform_cfg.num_procs * (_m + 1));

        // Check if all processor 0 & processor n-1 have been initialized
        let proc_0_init_vec = driver.ctrl_bridge.dbg_proc_0_init.read(&mut driver.simif)?;
        let proc_n_init_vec = driver.ctrl_bridge.dbg_proc_n_init.read(&mut driver.simif)?;
        assert!(proc_0_init_vec == proc_n_init_vec,
            "proc 0 {:x} n {:x}",
            proc_0_init_vec, proc_n_init_vec);

        // Check that the number of processors initialized processors match w/
        // what is expected
        let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
        let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
        assert!(dbg_init_cntr == circuit.platform_cfg.num_procs,
            "number of processors initialized for module {}: {} out of {}",
            _m, dbg_init_cntr, circuit.platform_cfg.num_procs);

        for module_idx in 0..circuit.platform_cfg.num_mods {
            let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(module_idx as usize).unwrap();
            let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
            println!("Number of processor initialized for module {}: {}", module_idx, dbg_init_cntr);
        }
    }
    inst_bar.finish();

    println!("total instructions pushed {} ",
        driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?);

    assert!(driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)? ==
            driver.ctrl_bridge.host_steps.read(&mut driver.simif)? * total_procs,
            "Pushed instructions doesn't match expectation w/ host steps {}",
            driver.ctrl_bridge.host_steps.read(&mut driver.simif)?);

    // Check that the host_steps did not change while pushing the instructions
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

    while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
        println!("init {}", driver.ctrl_bridge.init_done.read(&mut driver.simif)?);
        sleep(std::time::Duration::from_millis(1));
    }

    println!("Init done!!!");
    println!("Start Simulation");
    let total_procs = circuit.platform_cfg.total_procs();
    let axi4_data_bits = 512;
    let io_stream_bits = ((total_procs + axi4_data_bits - 1) / axi4_data_bits) * axi4_data_bits;
    let io_stream_bytes = io_stream_bits / 8;

    let mut mismatch = false;

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

        let mut ivec: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
        ivec.extend(bit_vec
            .into_vec()
            .iter()
            .flat_map(|x| x.to_le_bytes()));
            ivec.resize(io_stream_bytes as usize, 0);

            driver.io_bridge.push(&mut driver.simif, &ivec)?;

            let mut ovec = vec![0u8; ivec.len()];
            while true {
                let read_bytes = driver.io_bridge.pull(&mut driver.simif, &mut ovec)?;
                if read_bytes == 0 {
                    sleep(std::time::Duration::from_millis(1));
                } else {
                    break;
                }
            }

            // Run functional simulator
            let input_stimuli_by_step = get_input_stimuli_by_step(
                &circuit,
                &input_stimuli_blasted,
                &all_signal_map,
                tcycle as u32);
            funct_sim.run_cycle(&input_stimuli_by_step);

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

        println!("ovec: {:?}", ovec);
        println!("ovec_ref: {:?}", ovec_ref);

        if ovec != ovec_ref {
            println!("MISMATCH");
            println!("ovec: {:?}", ovec);
            println!("ovec_ref: {:?}", ovec_ref);
            println!("Target cycle {} mismatch got {:?} expect {:?}",
                tcycle, ovec, ovec_ref);
            mismatch = true;
            break 'emulation_loop;
        }
    }
    sim_bar.finish();

    if mismatch {
        println!("Test failed");
    } else {
        println!("Test passed");
    }

    println!("Test Finished");
    return Ok(());
}
