use clap::Parser;
use xdma_driver::*;
use rand::Rng;
use indicatif::ProgressBar;
use indexmap::IndexMap;
use std::{
    cmp::max, collections::VecDeque, thread::sleep, path::Path
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
    axi::*, dram::*, driver::*, harness::TargetSystem
};
use fesvr::*;

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

    #[arg(long, default_value_t = 1000)]
    pub dma_test_iterations: u32,

    #[arg(short, long, default_value_t = false)]
    pub trace_mode: bool,

    #[arg(short, long)]
    pub elf_file_path: String,

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
    let mut mapped_input_stimuli_blasted: IndexMap<Coordinate, VecDeque<u64>> = IndexMap::new();
    for (sig, stim) in input_stimuli_blasted.iter() {
        match all_signal_map.get(sig) {
            Some(nmi) =>  {
                let coord = nmi.info.coord;
                mapped_input_stimuli_blasted.insert(coord, VecDeque::from(stim.clone()));
            }
            None =>  { println!("Input Signal {} not found", sig); }
        }
    }

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

    let mut input_signals: IndexMap<String, Coordinate> = IndexMap::new();
    for nidx in circuit.graph.node_indices() {
        let node = circuit.graph.node_weight(nidx).unwrap();
        if node.is() == Primitive::Input {
            assert!(all_signal_map.contains_key(node.name()),
                    "input {} not found in signal map",
                    node.name());
            input_signals.insert(node.name().to_string(), node.info().coord);
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
    test_dma_bridge(&mut driver, args.dma_test_iterations, &fpga_top_cfg)?;
    set_target_config_regs(&mut driver, &sram_cfgs, circuit.emul.host_steps)?;
    push_instructions(&mut driver, module_insts, circuit.emul.host_steps, &fpga_top_cfg)?;

    while driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0 {
        println!("init {}", driver.ctrl_bridge.init_done.read(&mut driver.simif)?);
        sleep(std::time::Duration::from_millis(1));
    }

    println!("Simulation initialization finished");
    println!("Start simulation");

    if args.trace_mode {
        run_from_trace(&mut driver,
            &circuit,
            &input_stimuli_blasted,
            &all_signal_map,
            &output_signals,
            &mapped_input_stimuli_blasted,
            &fpga_top_cfg);
    } else {
        let mut target = TargetSystem::new(
            0,
            1000 * 1000 * 1000,
            8,
            driver,
            &fpga_top_cfg,
            input_signals,
            output_signals,
            "mem_axi4_0".to_string(),
            "serial_tl_0".to_string());

        let mut frontend = frontend::Frontend::try_new(
                Path::new(args.elf_file_path.as_str())).unwrap();
        frontend.write_elf(&mut target);
        frontend.reset(&mut target);

        let mut i = 1;
        loop {
            target.step();
            if i % 50 == 0 {
                if frontend.process(&mut target).expect("htif") {
                    break;
                }
            }
            i += 1;
        }
    }

    println!("Simulation ended");
    return Ok(());
}
