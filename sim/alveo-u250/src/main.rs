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
    let mut simif = XDMAInterface::try_new(
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

    let dma_bytes = 64;
    let mut rng = rand::thread_rng();

    println!("Testing Debug DMA Bridge");
    let iterations = 10000;
    let bar = ProgressBar::new(iterations);
    for i in 0..iterations {
        bar.inc(1);

        let mut wbuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
        wbuf.extend((0..dma_bytes).map(|_| rng.gen_range(10..16)));

        let written_bytes = driver.dbg_bridge.push(&mut driver.simif, &wbuf)?;
// println!("written_bytes: {}", written_bytes);

        let mut rbuf = vec![0u8; dma_bytes as usize];
        let read_bytes = driver.dbg_bridge.pull(&mut driver.simif, &mut rbuf)?;

        assert!(read_bytes == dma_bytes, "Read {} bytes, expected read {}", read_bytes, dma_bytes);
        assert!(wbuf == rbuf, "wbuf: {:X?}\nrbuf: {:X?}", wbuf, rbuf);
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

    println!("Start pushing instructions");
    let inst_bar = ProgressBar::new(module_insts.len() as u64);
    for (_m, insts) in module_insts.iter() {
        inst_bar.inc(1);
        for inst in insts {
            let mut bitbuf = inst.to_bits(&circuit.platform_cfg);
            bitbuf.reverse();
            assert!(bitbuf.len() < 8 * 8, "Instruction bits {} > 64", bitbuf.len());
            let mut bytebuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
            bytebuf.extend(bitbuf
                .clone()
                .into_vec()
                .iter()
                .flat_map(|&x| x.to_le_bytes())
                .rev());
            bytebuf.reverse();
            bytebuf.resize(64 as usize, 0);

            let mut bytebuf_ref: Vec<u8> = bitbuf
                .into_vec()
                .iter()
                .flat_map(|&x| x.to_le_bytes())
                .rev()
                .collect();
            bytebuf_ref.reverse();
            bytebuf_ref.resize(64 as usize, 0);
            assert!(bytebuf == bytebuf_ref, "inst buf mismatch\ngot: {:X?}\nref: {:X?}", bytebuf, bytebuf_ref);

            driver.inst_bridge.push(&mut driver.simif, &bytebuf)?;
        }
    }
    inst_bar.finish();

    while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
        sleep(std::time::Duration::from_millis(1));
    }

    println!("Init done!!!");
    println!("Start Simulation");
    let io_stream_bytes = 64;

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
