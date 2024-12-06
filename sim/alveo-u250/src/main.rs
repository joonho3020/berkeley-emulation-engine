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
        config::{Args, PlatformConfig},
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
use simif::{
    simif::*,
    mmioif::*,
    dmaif::*
};
use driver::{
    driver::*,
    axi::*
};

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

    #[arg(long, default_value_t = 64)]
    pub axi_addr_bits: u32,

    #[arg(long, default_value_t = 4)]
    pub axi_id_bits: u32,

    #[arg(long, default_value_t = 512)]
    pub axi_data_bits: u32,

    #[arg(long, default_value_t = 64)]
    pub axil_addr_bits: u32,

    #[arg(long, default_value_t = 4)]
    pub axil_id_bits: u32,

    #[arg(long, default_value_t = 32)]
    pub axil_data_bits: u32,

    #[arg(short, long, default_value_t = false)]
    pub functional_cosim: bool,

    #[clap(flatten)]
    pub bee_args: Args
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
            addr_bits: args.axi_addr_bits,
            id_bits: args.axi_id_bits,
            data_bits: args.axi_data_bits
        },
        axil: AXI4Config {
            addr_bits: args.axil_addr_bits,
            id_bits: args.axil_id_bits,
            data_bits: args.axil_data_bits
        },
        emul: circuit.platform_cfg.clone()
    };

    let mut driver = Driver::try_from_simif(Box::new(simif));

    pll_lock_and_fpga_top_reset(&mut driver)?;
    board_reset(&mut driver, &fpga_top_cfg)?;
    test_dma_bridge(&mut driver, 1000, &fpga_top_cfg)?;
    set_target_config_regs(&mut driver, &sram_cfgs, circuit.emul.host_steps)?;
    push_instructions(&mut driver, module_insts, circuit.emul.host_steps, &fpga_top_cfg)?;

    while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
        println!("init {}", driver.ctrl_bridge.init_done.read(&mut driver.simif)?);
        sleep(std::time::Duration::from_millis(1));
    }

    println!("Init done!!!");
    println!("Start Simulation");
    let total_procs = circuit.platform_cfg.total_procs();
    let axi4_data_bits = fpga_top_cfg.axi.data_bits;
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

        let written_bytes = driver.io_bridge.push(&mut driver.simif, &ivec)?;
        if written_bytes == 0 {
            println!("Target cycle {} DMA FAILED", tcycle);
            mismatch = true;
            break 'emulation_loop;
        }

        let mut ovec = vec![0u8; ivec.len()];
        'poll_io_out: loop {
            let read_bytes = driver.io_bridge.pull(&mut driver.simif, &mut ovec)?;
            if read_bytes == 0 {
                sleep(std::time::Duration::from_millis(1));
            } else {
                break 'poll_io_out;
            }
        }

        // Run functional simulator
        if args.functional_cosim {
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
        } else {
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
