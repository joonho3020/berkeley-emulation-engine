pub mod dut;
pub mod dut_if;
use bee::{
    common::{
        circuit::Circuit,
        config::{Args, PlatformConfig},
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
use clap::Parser;
use bit_vec::BitVec;
use dut::*;
use dut_if::*;

#[derive(Debug)]
pub enum RTLSimError {
    IOError(std::io::Error),
    SimError(String)
}

impl From<std::io::Error> for RTLSimError {
    fn from(err: std::io::Error) -> RTLSimError {
        RTLSimError::IOError(err)
    }
}

impl From<String> for RTLSimError {
    fn from(err: String) -> RTLSimError {
        RTLSimError::SimError(err)
    }
}

fn main() -> Result<(), RTLSimError> {
    let args = Args::parse();
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

        let num_mods = fpga_top_cfg.emul.num_mods;

        for m in 0..fpga_top_cfg.emul.num_mods {
            let used_procs = circuit.platform_cfg.num_procs;
            mmio_write(&mut sim, m * 4, used_procs);

            // NOTE: seems like the MCR file is designed under the
            // assumption that MMIO AXI requests arrive with more than
            // 1 cycle in between
            for _ in 0..5 {
                sim.step();
            }
        }

        for (m, sram_cfg) in sram_cfgs.iter() {
            let single_port_sram = match sram_cfg.port_type {
                SRAMPortType::SinglePortSRAM     => { true }
                SRAMPortType::OneRdOneWrPortSRAM => { false }
            };
            mmio_write(&mut sim, (m + 1 * num_mods) * 4, single_port_sram as u32);
            for _ in 0..5 {
                sim.step();
            }
            mmio_write(&mut sim, (m + 2 * num_mods) * 4, sram_cfg.wmask_bits);
            for _ in 0..5 {
                sim.step();
            }
            mmio_write(&mut sim, (m + 3 * num_mods) * 4, sram_cfg.width_bits);
            for _ in 0..5 {
                sim.step();
            }
        }

        mmio_write(&mut sim, (4 * num_mods) * 4, host_steps);

        sim.step();

        println!("configuration registers set finished");

        for (_m, insts) in module_insts.iter() {
            for inst in insts {
                let mut bitbuf = inst.to_bits(&circuit.platform_cfg);
                bitbuf.reverse();
                assert!(bitbuf.len() < 8 * 8, "Instruction bits {} > 64", bitbuf.len());

                println!("bitbuf: {:?}", bitbuf);

                let mut bytebuf: Vec<u8> = bitbuf
                                            .into_vec()
                                            .iter()
                                            .flat_map(|&x| x.to_le_bytes())
                                            .rev()
                                            .collect();
                println!("bytebuf: {:?}", bytebuf);

                bytebuf.reverse();

                println!("bytebuf reverse: {:?}", bytebuf);

                bytebuf.resize(fpga_top_cfg.axi.beat_bytes() as usize, 0);
                println!("bytebuf resized: {:?}", bytebuf);
// bytebuf.reverse();
// println!("bytebuf resized reverse: {:?}", bytebuf);
                dma_write(&mut sim, 4096, bytebuf.len() as u32, &bytebuf);
            }
        }

        println!("Start loading instructions");

        // Wait until initialization is finished
        while mmio_read(&mut sim, (4 * num_mods + 1) * 4)  == 0 {
            sim.step();
        }

        println!("Instructions loaded");

        for tc in 0..target_cycles {
            let tot_procs = circuit.platform_cfg.total_procs();
            let mut bit_vec = BitVec::new();
            for _ in 0..tot_procs {
                bit_vec.push(false);
            }
            for (coord, stim) in mapped_input_stimulti_blasted.iter_mut() {
                let bit = stim.pop_front().unwrap();
                let id = coord.id(&circuit.platform_cfg);
                bit_vec.set(id as usize, bit != 0);
            }

            let ivec: Vec<u8> = bit_vec.to_bytes();
            println!("ivec: {:?}", ivec);
            dma_write(&mut sim, 0, ivec.len() as u32, &ivec);

            // FIXME: properly compute the number of buffer entries
            while mmio_read(&mut sim, (4 * num_mods + 1) * 4) < 1 {
                sim.step();
            }

            let ovec: Vec<u8> = dma_read(&mut sim, 0, 64);
            println!("ovec: {:?}", ovec);
        }

        sim.finish();
    }
    println!("Hello, world!");
    return Ok(());
}
