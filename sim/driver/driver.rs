use rand::Rng;
use indicatif::ProgressBar;
use indexmap::IndexMap;
use std::{
    collections::VecDeque, thread::sleep
};
use bee::{
    common::{
        config::PlatformConfig,
        circuit::Circuit,
        hwgraph::NodeMapInfo, instruction::*,
        mapping::{SRAMMapping, SRAMPortType},
        primitive::Bit
    },
    rtlsim::rtlsim_utils::InputStimuliMap,
};
use crate::simif::simif::*;
use crate::simif::mmioif::*;
use crate::simif::dmaif::*;
use crate::driver::axi::*;

#[derive(Debug, Default, Clone)]
pub struct FPGATopConfig {
    pub axi:  AXI4Config,
    pub axil: AXI4Config,
    pub emul: PlatformConfig
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

/// Check if the PLL is locked and if it is not, assert & deassert the reset
/// signal going into the PLL.
/// After this, wait until the PLL gets locked, and trigger the FGPATop
/// reset signal.
pub fn pll_lock_and_fpga_top_reset(driver: &mut Driver) -> Result<(), SimIfErr> {
    println!("Clock wizard fingerprint register: {:x}",
        driver.clkwiz_ctrl.fingerprint.read(&mut driver.simif)?);

    driver.clkwiz_ctrl.pll_reset_cycle.write(&mut driver.simif, 500)?;
    driver.clkwiz_ctrl.pll_reset.write(&mut driver.simif, 1)?;

    while driver.clkwiz_ctrl.pll_locked.read(&mut driver.simif)? == 0 {
        println!("pll_locked mmio read is 0");
        for _ in 0..10 {
            driver.simif.step();
        }

        // PLL is locked
        driver.simif.init();
        driver.simif.step();
    }

    println!("PLL locked!");

    println!("FPGATop resetn sequence");
    driver.clkwiz_ctrl.fpga_top_resetn.write(&mut driver.simif, 0)?;
    for _i in 0..10 {
        driver.simif.step();
    }
    driver.clkwiz_ctrl.fpga_top_resetn.write(&mut driver.simif, 1)?;

    return Ok(());
}

/// Assert the board level reset.
/// Use finger print registers to check that the reset has been properly
/// triggered.
pub fn board_reset(driver: &mut Driver, cfg: &FPGATopConfig) -> Result<(), SimIfErr> {
    println!("Set custom resetn to low");
    driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 0)?;
    driver.simif.step();

    println!("Set custom resetn to high");
    driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 1)?;
    driver.simif.step();

    let pcs_are_zero = driver.ctrl_bridge.pcs_are_zero.read(&mut driver.simif)?;
    assert!(pcs_are_zero == (1 << cfg.emul.num_mods) - 1,
        "All PC values should be initialized after reset {:x}", pcs_are_zero);

    println!("Testing MMIO fingerprint");
    let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
    assert!(fgr_init == 0xf00dcafe,
        "mmio fingerprint mismatch, expect 0xf00dcafe got {}", fgr_init);

    println!("Write to MMIO fingerprint");
    driver.ctrl_bridge.fingerprint.write(&mut driver.simif, 0xdeadbeaf)?;
    driver.simif.step();

    let fgr_read = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
    assert!(fgr_read == 0xdeadbeaf,
        "mmio fingerprint mismatch, expect {:x} got {:x}", 0xdeadbeafu32, fgr_read);

    // Custom reset
    println!("Set custom resetn to low");
    driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 0)?;
    driver.simif.step();

    println!("Set custom resetn to high");
    driver.ctrl_bridge.custom_resetn.write(&mut driver.simif, 1)?;
    driver.simif.step();

    println!("Read MMIO fingerprint again after reset");
    let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
    assert!(fgr_init == 0xf00dcafe,
        "mmio fingerprint mismatch, expect 0xf00dcafe got {}", fgr_init);

    println!("Read MMIO pcs_are_zero");
    let pcs_are_zero = driver.ctrl_bridge.pcs_are_zero.read(&mut driver.simif)?;
    assert!(pcs_are_zero == (1 << cfg.emul.num_mods) - 1,
        "All PC values should be initialized after reset {:x}", pcs_are_zero);

    return Ok(());
}

/// Tests the DMA push and pull interface
pub fn test_dma_bridge(driver: &mut Driver, iterations: u32, cfg: &FPGATopConfig) -> Result<(), SimIfErr> {
    let bar = ProgressBar::new(iterations as u64);

    let total_procs = cfg.emul.num_mods * cfg.emul.num_procs;
    let data_bits = cfg.axi.data_bits;
    let io_stream_bits = ((total_procs + data_bits - 1) / data_bits) * data_bits;
    let dma_bytes = io_stream_bits / 8;

    println!("total_procs: {}, axi data bits: {}, io_stream_bits: {}, dma_bytes: {}",
        total_procs, data_bits, io_stream_bits, dma_bytes);

    let mut rng = rand::thread_rng();

    for _i in 0..iterations {
        bar.inc(1);

        let mut wbuf: Vec<u8> = vec![];
        wbuf.extend((0..dma_bytes).map(|_| rng.gen_range(10..16)));

        let written_bytes = driver.dma_bridge.push(&mut driver.simif, &wbuf)?;
        assert!(written_bytes == dma_bytes,
            "DMA write didn't write expected amount. Wrote: {} out of {} byte, iter {}",
            written_bytes, dma_bytes, _i);

        for _ in 0..20 {
            driver.simif.step();
        }

        let mut rbuf = vec![0u8; dma_bytes as usize];
        let read_bytes = driver.dma_bridge.pull(&mut driver.simif, &mut rbuf)?;
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

    return Ok(());
}

pub fn set_target_config_regs(
    driver: &mut Driver,
    sram_cfgs: &IndexMap<u32, SRAMMapping>,
    host_steps: u32
) -> Result<(), SimIfErr> {
    println!("Start configuration register setup");

    println!("Setting SRAM config registers");
    for (m, sram_cfg) in sram_cfgs.iter() {
        let single_port_sram = match sram_cfg.port_type {
            SRAMPortType::SinglePortSRAM     => { true }
            SRAMPortType::OneRdOneWrPortSRAM => { false }
        };

        let sram_mmios: &SRAMConfig = driver.ctrl_bridge.sram.get(*m as usize).unwrap();
        sram_mmios.ptype.write(&mut driver.simif, single_port_sram    as u32)?;
        driver.simif.step();

        sram_mmios.mask .write(&mut driver.simif, sram_cfg.wmask_bits as u32)?;
        driver.simif.step();

        sram_mmios.width.write(&mut driver.simif, sram_cfg.width_bits as u32)?;
        driver.simif.step();
    }

    println!("Setting host_steps");
    driver.ctrl_bridge.host_steps.write(&mut driver.simif, host_steps)?;
    driver.simif.step();
    assert!(driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0,
        "Init set before pushing instructions");

    while driver.ctrl_bridge.host_steps.read(&mut driver.simif)? == 0 {
    }
    println!("host_steps set to {}", driver.ctrl_bridge.host_steps.read(&mut driver.simif)?);

    return Ok(());
}

pub fn push_instructions(
    driver: &mut Driver,
    module_insts: IndexMap<u32, VecDeque<Instruction>>,
    host_steps: u32,
    cfg: &FPGATopConfig,
) -> Result<(), SimIfErr> {
    println!("Start pushing instructions");
    println!("num_proc_bits: {} num_mod_bits: {}", cfg.emul.num_proc_bits(), cfg.emul.num_mod_bits());

    let inst_bar = ProgressBar::new(module_insts.len() as u64);
    for (_m, insts) in module_insts.iter() {
        let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
        let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;

        assert!(dbg_init_cntr == 0, "There should be no processors that are initialized");
        assert!(insts.len() as u32 == host_steps * cfg.emul.num_procs,
            "Number of instructions for this module is weird got {}, expect {}",
            insts.len(),
            host_steps * cfg.emul.num_procs);

        let proc_inst_bar = ProgressBar::new(insts.len() as u64);
        for (inst_idx, inst) in insts.iter().enumerate() {
            let _p = inst_idx as u32 / host_steps;

            let mut bitbuf = inst.to_bits(&cfg.emul);

            assert!(bitbuf.len() < 8 * 8, "Instruction bits {} > 64", bitbuf.len());
            assert!(driver.ctrl_bridge.init_done.read(&mut driver.simif)? == 0,
                "Init set while pushing instructions, module {} inst {}", _m, inst_idx);

            for x in 0..cfg.emul.num_proc_bits() {
                let sl = cfg.emul.num_proc_bits() - x - 1;
                bitbuf.push((_p >> sl) & 1 == 1);
            }
            for x in 0..cfg.emul.num_mod_bits() {
                let sl = cfg.emul.num_mod_bits() - x - 1;
                bitbuf.push((_m >> sl) & 1 == 1);
            }
            bitbuf.reverse();

            assert!(bitbuf.len() < cfg.axi.data_bits as usize);

            let mut bytebuf: Vec<u8> = vec![];
            bytebuf.extend(bitbuf
                .clone()
                .into_vec()
                .iter()
                .flat_map(|&x| x.to_le_bytes())
                .rev());
            bytebuf.reverse();
            bytebuf.resize(64 as usize, 0);

            let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(*_m as usize).unwrap();
            let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
            if dbg_init_cntr != _p {
                println!("FISHY... Initializing module {} processor {}, initialized count {}",
                    _m, _p, dbg_init_cntr);

                // Check if all processor 0 & processor n-1 have been initialized
                let proc_0_init_vec = driver.ctrl_bridge.dbg_proc_0_init.read(&mut driver.simif)?;
                let proc_n_init_vec = driver.ctrl_bridge.dbg_proc_n_init.read(&mut driver.simif)?;
                println!("proc_0_init_vec: {:x} proc_n_init_vec: {:x}", proc_0_init_vec, proc_n_init_vec);

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

                        assert!(false, "wrote zero bytes");
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
            driver.simif.step();

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
                    _p);
            }
            proc_inst_bar.inc(1);
        }
        proc_inst_bar.finish();

        let tot_insts_pushed = driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?;
        println!("total instructions pushed {} ",
            driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?);
        assert!(tot_insts_pushed == host_steps * cfg.emul.num_procs * (_m + 1));

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
        assert!(dbg_init_cntr == cfg.emul.num_procs,
            "number of processors initialized for module {}: {} out of {}",
            _m, dbg_init_cntr, cfg.emul.num_procs);

        for module_idx in 0..cfg.emul.num_mods {
            let dbg_init_cntr_mmio = driver.ctrl_bridge.dbg_init_cntrs.get(module_idx as usize).unwrap();
            let dbg_init_cntr = dbg_init_cntr_mmio.read(&mut driver.simif)?;
            println!("Number of processor initialized for module {}: {}", module_idx, dbg_init_cntr);
        }
        inst_bar.inc(1);
    }
    inst_bar.finish();

    println!("total instructions pushed {} ",
        driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)?);

    assert!(driver.ctrl_bridge.tot_insts_pushed.read(&mut driver.simif)? ==
            driver.ctrl_bridge.host_steps.read(&mut driver.simif)? * cfg.emul.total_procs(),
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
    return Ok(());
}
