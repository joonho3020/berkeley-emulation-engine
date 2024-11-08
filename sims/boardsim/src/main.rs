pub mod dut;
use bee::{
    common::{
        circuit::Circuit, config::Args, hwgraph::NodeMapInfo, instruction::*, mapping::{SRAMMapping, SRAMPortType}, network::Coordinate, primitive::{Bit, Primitive}
    }, fsim::board::Board, rtlsim::rtlsim_utils::{get_input_stimuli_blasted, InputStimuliMap}, testing::try_new_circuit
};
use clap::Parser;
use dut::*;
use indexmap::IndexMap;
use std::{collections::VecDeque, cmp::max};

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

unsafe fn step(dut: *mut VBoard, vcd: *mut VerilatedVcdC, cycle: &mut u32) {
    let time = *cycle * 2;
    Board_eval(dut);
    dump_vcd(vcd, time);

    poke_clock(dut, 1);
    Board_eval(dut);
    dump_vcd(vcd, time + 1);

    poke_clock(dut, 0);
    *cycle += 1;
}

unsafe fn poke_io_coord(dut: *mut VBoard, coord: &Coordinate, bit: u64) {
    if coord.module == 0 && coord.proc == 0 {
        poke_io_io_0_i_0(dut, bit);
    } else if coord.module == 0 && coord.proc == 1 {
        poke_io_io_0_i_1(dut, bit);
    } else if coord.module == 0 && coord.proc == 2 {
        poke_io_io_0_i_2(dut, bit);
    } else if coord.module == 0 && coord.proc == 3 {
        poke_io_io_0_i_3(dut, bit);
    } else if coord.module == 0 && coord.proc == 4 {
        poke_io_io_0_i_4(dut, bit);
    } else if coord.module == 0 && coord.proc == 5 {
        poke_io_io_0_i_5(dut, bit);
    } else if coord.module == 0 && coord.proc == 6 {
        poke_io_io_0_i_6(dut, bit);
    } else if coord.module == 0 && coord.proc == 7 {
        poke_io_io_0_i_7(dut, bit);
    } else if coord.module == 1 && coord.proc == 0 {
        poke_io_io_1_i_0(dut, bit);
    } else if coord.module == 1 && coord.proc == 1 {
        poke_io_io_1_i_1(dut, bit);
    } else if coord.module == 1 && coord.proc == 2 {
        poke_io_io_1_i_2(dut, bit);
    } else if coord.module == 1 && coord.proc == 3 {
        poke_io_io_1_i_3(dut, bit);
    } else if coord.module == 1 && coord.proc == 4 {
        poke_io_io_1_i_4(dut, bit);
    } else if coord.module == 1 && coord.proc == 5 {
        poke_io_io_1_i_5(dut, bit);
    } else if coord.module == 1 && coord.proc == 6 {
        poke_io_io_1_i_6(dut, bit);
    } else if coord.module == 1 && coord.proc == 7 {
        poke_io_io_1_i_7(dut, bit);
    } else if coord.module == 2 && coord.proc == 0 {
        poke_io_io_2_i_0(dut, bit);
    } else if coord.module == 2 && coord.proc == 1 {
        poke_io_io_2_i_1(dut, bit);
    } else if coord.module == 2 && coord.proc == 2 {
        poke_io_io_2_i_2(dut, bit);
    } else if coord.module == 2 && coord.proc == 3 {
        poke_io_io_2_i_3(dut, bit);
    } else if coord.module == 2 && coord.proc == 4 {
        poke_io_io_2_i_4(dut, bit);
    } else if coord.module == 2 && coord.proc == 5 {
        poke_io_io_2_i_5(dut, bit);
    } else if coord.module == 2 && coord.proc == 6 {
        poke_io_io_2_i_6(dut, bit);
    } else if coord.module == 2 && coord.proc == 7 {
        poke_io_io_2_i_7(dut, bit);
    } else if coord.module == 3 && coord.proc == 0 {
        poke_io_io_3_i_0(dut, bit);
    } else if coord.module == 3 && coord.proc == 1 {
        poke_io_io_3_i_1(dut, bit);
    } else if coord.module == 3 && coord.proc == 2 {
        poke_io_io_3_i_2(dut, bit);
    } else if coord.module == 3 && coord.proc == 3 {
        poke_io_io_3_i_3(dut, bit);
    } else if coord.module == 3 && coord.proc == 4 {
        poke_io_io_3_i_4(dut, bit);
    } else if coord.module == 3 && coord.proc == 5 {
        poke_io_io_3_i_5(dut, bit);
    } else if coord.module == 3 && coord.proc == 6 {
        poke_io_io_3_i_6(dut, bit);
    } else if coord.module == 3 && coord.proc == 7 {
        poke_io_io_3_i_7(dut, bit);
    } else if coord.module == 4 && coord.proc == 0 {
        poke_io_io_4_i_0(dut, bit);
    } else if coord.module == 4 && coord.proc == 1 {
        poke_io_io_4_i_1(dut, bit);
    } else if coord.module == 4 && coord.proc == 2 {
        poke_io_io_4_i_2(dut, bit);
    } else if coord.module == 4 && coord.proc == 3 {
        poke_io_io_4_i_3(dut, bit);
    } else if coord.module == 4 && coord.proc == 4 {
        poke_io_io_4_i_4(dut, bit);
    } else if coord.module == 4 && coord.proc == 5 {
        poke_io_io_4_i_5(dut, bit);
    } else if coord.module == 4 && coord.proc == 6 {
        poke_io_io_4_i_6(dut, bit);
    } else if coord.module == 4 && coord.proc == 7 {
        poke_io_io_4_i_7(dut, bit);
    } else if coord.module == 5 && coord.proc == 0 {
        poke_io_io_5_i_0(dut, bit);
    } else if coord.module == 5 && coord.proc == 1 {
        poke_io_io_5_i_1(dut, bit);
    } else if coord.module == 5 && coord.proc == 2 {
        poke_io_io_5_i_2(dut, bit);
    } else if coord.module == 5 && coord.proc == 3 {
        poke_io_io_5_i_3(dut, bit);
    } else if coord.module == 5 && coord.proc == 4 {
        poke_io_io_5_i_4(dut, bit);
    } else if coord.module == 5 && coord.proc == 5 {
        poke_io_io_5_i_5(dut, bit);
    } else if coord.module == 5 && coord.proc == 6 {
        poke_io_io_5_i_6(dut, bit);
    } else if coord.module == 5 && coord.proc == 7 {
        poke_io_io_5_i_7(dut, bit);
    } else if coord.module == 6 && coord.proc == 0 {
        poke_io_io_6_i_0(dut, bit);
    } else if coord.module == 6 && coord.proc == 1 {
        poke_io_io_6_i_1(dut, bit);
    } else if coord.module == 6 && coord.proc == 2 {
        poke_io_io_6_i_2(dut, bit);
    } else if coord.module == 6 && coord.proc == 3 {
        poke_io_io_6_i_3(dut, bit);
    } else if coord.module == 6 && coord.proc == 4 {
        poke_io_io_6_i_4(dut, bit);
    } else if coord.module == 6 && coord.proc == 5 {
        poke_io_io_6_i_5(dut, bit);
    } else if coord.module == 6 && coord.proc == 6 {
        poke_io_io_6_i_6(dut, bit);
    } else if coord.module == 6 && coord.proc == 7 {
        poke_io_io_6_i_7(dut, bit);
    } else if coord.module == 7 && coord.proc == 0 {
        poke_io_io_7_i_0(dut, bit);
    } else if coord.module == 7 && coord.proc == 1 {
        poke_io_io_7_i_1(dut, bit);
    } else if coord.module == 7 && coord.proc == 2 {
        poke_io_io_7_i_2(dut, bit);
    } else if coord.module == 7 && coord.proc == 3 {
        poke_io_io_7_i_3(dut, bit);
    } else if coord.module == 7 && coord.proc == 4 {
        poke_io_io_7_i_4(dut, bit);
    } else if coord.module == 7 && coord.proc == 5 {
        poke_io_io_7_i_5(dut, bit);
    } else if coord.module == 7 && coord.proc == 6 {
        poke_io_io_7_i_6(dut, bit);
    } else if coord.module == 7 && coord.proc == 7 {
        poke_io_io_7_i_7(dut, bit);
    } else if coord.module == 8 && coord.proc == 0 {
        poke_io_io_8_i_0(dut, bit);
    } else if coord.module == 8 && coord.proc == 1 {
        poke_io_io_8_i_1(dut, bit);
    } else if coord.module == 8 && coord.proc == 2 {
        poke_io_io_8_i_2(dut, bit);
    } else if coord.module == 8 && coord.proc == 3 {
        poke_io_io_8_i_3(dut, bit);
    } else if coord.module == 8 && coord.proc == 4 {
        poke_io_io_8_i_4(dut, bit);
    } else if coord.module == 8 && coord.proc == 5 {
        poke_io_io_8_i_5(dut, bit);
    } else if coord.module == 8 && coord.proc == 6 {
        poke_io_io_8_i_6(dut, bit);
    } else if coord.module == 8 && coord.proc == 7 {
        poke_io_io_8_i_7(dut, bit);
    }
}

unsafe fn peek_io_coord(dut: *mut VBoard, coord: &Coordinate) -> u64 {
    if coord.module == 0 && coord.proc == 0 {
        return peek_io_io_0_o_0(dut);
    } else if coord.module == 0 && coord.proc == 1 {
        return peek_io_io_0_o_1(dut);
    } else if coord.module == 0 && coord.proc == 2 {
        return peek_io_io_0_o_2(dut);
    } else if coord.module == 0 && coord.proc == 3 {
        return peek_io_io_0_o_3(dut);
    } else if coord.module == 0 && coord.proc == 4 {
        return peek_io_io_0_o_4(dut);
    } else if coord.module == 0 && coord.proc == 5 {
        return peek_io_io_0_o_5(dut);
    } else if coord.module == 0 && coord.proc == 6 {
        return peek_io_io_0_o_6(dut);
    } else if coord.module == 0 && coord.proc == 7 {
        return peek_io_io_0_o_7(dut);
    } else if coord.module == 1 && coord.proc == 0 {
        return peek_io_io_1_o_0(dut);
    } else if coord.module == 1 && coord.proc == 1 {
        return peek_io_io_1_o_1(dut);
    } else if coord.module == 1 && coord.proc == 2 {
        return peek_io_io_1_o_2(dut);
    } else if coord.module == 1 && coord.proc == 3 {
        return peek_io_io_1_o_3(dut);
    } else if coord.module == 1 && coord.proc == 4 {
        return peek_io_io_1_o_4(dut);
    } else if coord.module == 1 && coord.proc == 5 {
        return peek_io_io_1_o_5(dut);
    } else if coord.module == 1 && coord.proc == 6 {
        return peek_io_io_1_o_6(dut);
    } else if coord.module == 1 && coord.proc == 7 {
        return peek_io_io_1_o_7(dut);
    } else if coord.module == 2 && coord.proc == 0 {
        return peek_io_io_2_o_0(dut);
    } else if coord.module == 2 && coord.proc == 1 {
        return peek_io_io_2_o_1(dut);
    } else if coord.module == 2 && coord.proc == 2 {
        return peek_io_io_2_o_2(dut);
    } else if coord.module == 2 && coord.proc == 3 {
        return peek_io_io_2_o_3(dut);
    } else if coord.module == 2 && coord.proc == 4 {
        return peek_io_io_2_o_4(dut);
    } else if coord.module == 2 && coord.proc == 5 {
        return peek_io_io_2_o_5(dut);
    } else if coord.module == 2 && coord.proc == 6 {
        return peek_io_io_2_o_6(dut);
    } else if coord.module == 2 && coord.proc == 7 {
        return peek_io_io_2_o_7(dut);
    } else if coord.module == 3 && coord.proc == 0 {
        return peek_io_io_3_o_0(dut);
    } else if coord.module == 3 && coord.proc == 1 {
        return peek_io_io_3_o_1(dut);
    } else if coord.module == 3 && coord.proc == 2 {
        return peek_io_io_3_o_2(dut);
    } else if coord.module == 3 && coord.proc == 3 {
        return peek_io_io_3_o_3(dut);
    } else if coord.module == 3 && coord.proc == 4 {
        return peek_io_io_3_o_4(dut);
    } else if coord.module == 3 && coord.proc == 5 {
        return peek_io_io_3_o_5(dut);
    } else if coord.module == 3 && coord.proc == 6 {
        return peek_io_io_3_o_6(dut);
    } else if coord.module == 3 && coord.proc == 7 {
        return peek_io_io_3_o_7(dut);
    } else if coord.module == 4 && coord.proc == 0 {
        return peek_io_io_4_o_0(dut);
    } else if coord.module == 4 && coord.proc == 1 {
        return peek_io_io_4_o_1(dut);
    } else if coord.module == 4 && coord.proc == 2 {
        return peek_io_io_4_o_2(dut);
    } else if coord.module == 4 && coord.proc == 3 {
        return peek_io_io_4_o_3(dut);
    } else if coord.module == 4 && coord.proc == 4 {
        return peek_io_io_4_o_4(dut);
    } else if coord.module == 4 && coord.proc == 5 {
        return peek_io_io_4_o_5(dut);
    } else if coord.module == 4 && coord.proc == 6 {
        return peek_io_io_4_o_6(dut);
    } else if coord.module == 4 && coord.proc == 7 {
        return peek_io_io_4_o_7(dut);
    } else if coord.module == 5 && coord.proc == 0 {
        return peek_io_io_5_o_0(dut);
    } else if coord.module == 5 && coord.proc == 1 {
        return peek_io_io_5_o_1(dut);
    } else if coord.module == 5 && coord.proc == 2 {
        return peek_io_io_5_o_2(dut);
    } else if coord.module == 5 && coord.proc == 3 {
        return peek_io_io_5_o_3(dut);
    } else if coord.module == 5 && coord.proc == 4 {
        return peek_io_io_5_o_4(dut);
    } else if coord.module == 5 && coord.proc == 5 {
        return peek_io_io_5_o_5(dut);
    } else if coord.module == 5 && coord.proc == 6 {
        return peek_io_io_5_o_6(dut);
    } else if coord.module == 5 && coord.proc == 7 {
        return peek_io_io_5_o_7(dut);
    } else if coord.module == 6 && coord.proc == 0 {
        return peek_io_io_6_o_0(dut);
    } else if coord.module == 6 && coord.proc == 1 {
        return peek_io_io_6_o_1(dut);
    } else if coord.module == 6 && coord.proc == 2 {
        return peek_io_io_6_o_2(dut);
    } else if coord.module == 6 && coord.proc == 3 {
        return peek_io_io_6_o_3(dut);
    } else if coord.module == 6 && coord.proc == 4 {
        return peek_io_io_6_o_4(dut);
    } else if coord.module == 6 && coord.proc == 5 {
        return peek_io_io_6_o_5(dut);
    } else if coord.module == 6 && coord.proc == 6 {
        return peek_io_io_6_o_6(dut);
    } else if coord.module == 6 && coord.proc == 7 {
        return peek_io_io_6_o_7(dut);
    } else if coord.module == 7 && coord.proc == 0 {
        return peek_io_io_7_o_0(dut);
    } else if coord.module == 7 && coord.proc == 1 {
        return peek_io_io_7_o_1(dut);
    } else if coord.module == 7 && coord.proc == 2 {
        return peek_io_io_7_o_2(dut);
    } else if coord.module == 7 && coord.proc == 3 {
        return peek_io_io_7_o_3(dut);
    } else if coord.module == 7 && coord.proc == 4 {
        return peek_io_io_7_o_4(dut);
    } else if coord.module == 7 && coord.proc == 5 {
        return peek_io_io_7_o_5(dut);
    } else if coord.module == 7 && coord.proc == 6 {
        return peek_io_io_7_o_6(dut);
    } else if coord.module == 7 && coord.proc == 7 {
        return peek_io_io_7_o_7(dut);
    } else if coord.module == 8 && coord.proc == 0 {
        return peek_io_io_8_o_0(dut);
    } else if coord.module == 8 && coord.proc == 1 {
        return peek_io_io_8_o_1(dut);
    } else if coord.module == 8 && coord.proc == 2 {
        return peek_io_io_8_o_2(dut);
    } else if coord.module == 8 && coord.proc == 3 {
        return peek_io_io_8_o_3(dut);
    } else if coord.module == 8 && coord.proc == 4 {
        return peek_io_io_8_o_4(dut);
    } else if coord.module == 8 && coord.proc == 5 {
        return peek_io_io_8_o_5(dut);
    } else if coord.module == 8 && coord.proc == 6 {
        return peek_io_io_8_o_6(dut);
    } else if coord.module == 8 && coord.proc == 7 {
        return peek_io_io_8_o_7(dut);
    } else {
        return 0;
    }
}

unsafe  fn poke_sram_cfg(dut: *mut VBoard, module: &u32, cfg: &SRAMMapping) {
    let single_port_sram = match cfg.port_type {
        SRAMPortType::SinglePortSRAM => { true }
        SRAMPortType::OneRdOneWrPortSRAM => { false }
    };

    match module {
        0 => {
            poke_io_cfg_in_0_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_0_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_0_sram_single_port_ram(dut, single_port_sram as u64);
        }
        1 => {
            poke_io_cfg_in_1_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_1_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_1_sram_single_port_ram(dut, single_port_sram as u64);
        }
        2 => {
            poke_io_cfg_in_2_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_2_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_2_sram_single_port_ram(dut, single_port_sram as u64);
        }
        3 => {
            poke_io_cfg_in_3_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_3_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_3_sram_single_port_ram(dut, single_port_sram as u64);
        }
        4 => {
            poke_io_cfg_in_4_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_4_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_4_sram_single_port_ram(dut, single_port_sram as u64);
        }
        5 => {
            poke_io_cfg_in_5_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_5_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_5_sram_single_port_ram(dut, single_port_sram as u64);
        }
        6 => {
            poke_io_cfg_in_6_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_6_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_6_sram_single_port_ram(dut, single_port_sram as u64);
        }
        7 => {
            poke_io_cfg_in_7_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_7_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_7_sram_single_port_ram(dut, single_port_sram as u64);
        }
        8 => {
            poke_io_cfg_in_8_sram_width_bits(dut, cfg.width_bits as u64);
            poke_io_cfg_in_8_sram_wmask_bits(dut, cfg.wmask_bits as u64);
            poke_io_cfg_in_8_sram_single_port_ram(dut, single_port_sram as u64);
        }
        _ => {
        }
    }
}

unsafe fn poke_inst_module(dut: *mut VBoard, module: &u32, inst: &Instruction) {
    match module {
        0 => {
            poke_io_insts_0_valid(dut, 1);
            poke_io_insts_0_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_0_bits_lut(dut, inst.lut);

            poke_io_insts_0_bits_ops_2_rs(dut, 0);
            poke_io_insts_0_bits_ops_2_local(dut, 0);
            poke_io_insts_0_bits_ops_1_rs(dut, 0);
            poke_io_insts_0_bits_ops_1_local(dut, 0);
            poke_io_insts_0_bits_ops_0_rs(dut, 0);
            poke_io_insts_0_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_0_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_0_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_0_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_0_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_0_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_0_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_0_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_0_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_0_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_0_bits_mem(dut, inst.mem as u64);
        }
        1 => {
            poke_io_insts_1_valid(dut, 1);
            poke_io_insts_1_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_1_bits_lut(dut, inst.lut);

            poke_io_insts_1_bits_ops_2_rs(dut, 0);
            poke_io_insts_1_bits_ops_2_local(dut, 0);
            poke_io_insts_1_bits_ops_1_rs(dut, 0);
            poke_io_insts_1_bits_ops_1_local(dut, 0);
            poke_io_insts_1_bits_ops_0_rs(dut, 0);
            poke_io_insts_1_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_1_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_1_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_1_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_1_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_1_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_1_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_1_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_1_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_1_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_1_bits_mem(dut, inst.mem as u64);
        }
        2 => {
            poke_io_insts_2_valid(dut, 1);
            poke_io_insts_2_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_2_bits_lut(dut, inst.lut);

            poke_io_insts_2_bits_ops_2_rs(dut, 0);
            poke_io_insts_2_bits_ops_2_local(dut, 0);
            poke_io_insts_2_bits_ops_1_rs(dut, 0);
            poke_io_insts_2_bits_ops_1_local(dut, 0);
            poke_io_insts_2_bits_ops_0_rs(dut, 0);
            poke_io_insts_2_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_2_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_2_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_2_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_2_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_2_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_2_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_2_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_2_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_2_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_2_bits_mem(dut, inst.mem as u64);
        }
        3 => {
            poke_io_insts_3_valid(dut, 1);
            poke_io_insts_3_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_3_bits_lut(dut, inst.lut);

            poke_io_insts_3_bits_ops_2_rs(dut, 0);
            poke_io_insts_3_bits_ops_2_local(dut, 0);
            poke_io_insts_3_bits_ops_1_rs(dut, 0);
            poke_io_insts_3_bits_ops_1_local(dut, 0);
            poke_io_insts_3_bits_ops_0_rs(dut, 0);
            poke_io_insts_3_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_3_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_3_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_3_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_3_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_3_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_3_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_3_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_3_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_3_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_3_bits_mem(dut, inst.mem as u64);
        }
        4 => {
            poke_io_insts_4_valid(dut, 1);
            poke_io_insts_4_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_4_bits_lut(dut, inst.lut);

            poke_io_insts_4_bits_ops_2_rs(dut, 0);
            poke_io_insts_4_bits_ops_2_local(dut, 0);
            poke_io_insts_4_bits_ops_1_rs(dut, 0);
            poke_io_insts_4_bits_ops_1_local(dut, 0);
            poke_io_insts_4_bits_ops_0_rs(dut, 0);
            poke_io_insts_4_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_4_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_4_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_4_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_4_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_4_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_4_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_4_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_4_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_4_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_4_bits_mem(dut, inst.mem as u64);
        }
        5 => {
            poke_io_insts_5_valid(dut, 1);
            poke_io_insts_5_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_5_bits_lut(dut, inst.lut);

            poke_io_insts_5_bits_ops_2_rs(dut, 0);
            poke_io_insts_5_bits_ops_2_local(dut, 0);
            poke_io_insts_5_bits_ops_1_rs(dut, 0);
            poke_io_insts_5_bits_ops_1_local(dut, 0);
            poke_io_insts_5_bits_ops_0_rs(dut, 0);
            poke_io_insts_5_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_5_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_5_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_5_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_5_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_5_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_5_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_5_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_5_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_5_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_5_bits_mem(dut, inst.mem as u64);
        }
        6 => {
            poke_io_insts_6_valid(dut, 1);
            poke_io_insts_6_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_6_bits_lut(dut, inst.lut);

            poke_io_insts_6_bits_ops_2_rs(dut, 0);
            poke_io_insts_6_bits_ops_2_local(dut, 0);
            poke_io_insts_6_bits_ops_1_rs(dut, 0);
            poke_io_insts_6_bits_ops_1_local(dut, 0);
            poke_io_insts_6_bits_ops_0_rs(dut, 0);
            poke_io_insts_6_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_6_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_6_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_6_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_6_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_6_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_6_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_6_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_6_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_6_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_6_bits_mem(dut, inst.mem as u64);
        }
        7 => {
            poke_io_insts_7_valid(dut, 1);
            poke_io_insts_7_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_7_bits_lut(dut, inst.lut);

            poke_io_insts_7_bits_ops_2_rs(dut, 0);
            poke_io_insts_7_bits_ops_2_local(dut, 0);
            poke_io_insts_7_bits_ops_1_rs(dut, 0);
            poke_io_insts_7_bits_ops_1_local(dut, 0);
            poke_io_insts_7_bits_ops_0_rs(dut, 0);
            poke_io_insts_7_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_7_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_7_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_7_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_7_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_7_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_7_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_7_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_7_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_7_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_7_bits_mem(dut, inst.mem as u64);
        }
        8 => {
            poke_io_insts_8_valid(dut, 1);
            poke_io_insts_8_bits_opcode(dut, inst.opcode as u64);
            poke_io_insts_8_bits_lut(dut, inst.lut);

            poke_io_insts_8_bits_ops_2_rs(dut, 0);
            poke_io_insts_8_bits_ops_2_local(dut, 0);
            poke_io_insts_8_bits_ops_1_rs(dut, 0);
            poke_io_insts_8_bits_ops_1_local(dut, 0);
            poke_io_insts_8_bits_ops_0_rs(dut, 0);
            poke_io_insts_8_bits_ops_0_local(dut, 0);
            if inst.operands.len() > 2 {
                poke_io_insts_8_bits_ops_2_rs(dut, inst.operands[2].rs as u64);
                poke_io_insts_8_bits_ops_2_local(dut, inst.operands[2].local as u64);
            }
            if inst.operands.len() > 1 {
                poke_io_insts_8_bits_ops_1_rs(dut, inst.operands[1].rs as u64);
                poke_io_insts_8_bits_ops_1_local(dut, inst.operands[1].local as u64);
            }
            if inst.operands.len() > 0 {
                poke_io_insts_8_bits_ops_0_rs(dut, inst.operands[0].rs as u64);
                poke_io_insts_8_bits_ops_0_local(dut, inst.operands[0].local as u64);
            }

            poke_io_insts_8_bits_sinfo_idx(dut, inst.sinfo.idx as u64);
            poke_io_insts_8_bits_sinfo_local(dut, inst.sinfo.local as u64);
            poke_io_insts_8_bits_sinfo_fwd(dut, inst.sinfo.fwd as u64);
            poke_io_insts_8_bits_mem(dut, inst.mem as u64);
        }
        _ => {}
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

pub fn test_rtl(args: &Args) -> Result<(), RTLSimError> {
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

    let mut mismatch_string: Option<String> = None;
    let mut cycle = 0;
    unsafe {
        let dut = Board_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);

        poke_reset(dut, 1);
        for _ in 0..5 {
            step(dut, vcd, &mut cycle);
        }

        poke_reset(dut, 0);
        for _ in 0..5 {
            step(dut, vcd, &mut cycle);
        }

        // Testbench logic here

        // Set emulator configuration
        let host_steps = circuit.emul.host_steps;
        poke_io_cfg_in_0_host_steps(dut, host_steps.into());
        poke_io_cfg_in_1_host_steps(dut, host_steps.into());
        poke_io_cfg_in_2_host_steps(dut, host_steps.into());
        poke_io_cfg_in_3_host_steps(dut, host_steps.into());
        poke_io_cfg_in_4_host_steps(dut, host_steps.into());
        poke_io_cfg_in_5_host_steps(dut, host_steps.into());
        poke_io_cfg_in_6_host_steps(dut, host_steps.into());
        poke_io_cfg_in_7_host_steps(dut, host_steps.into());
        poke_io_cfg_in_8_host_steps(dut, host_steps.into());

        let used_procs = circuit.platform_cfg.num_procs;
        poke_io_cfg_in_0_used_procs(dut, used_procs.into());
        poke_io_cfg_in_1_used_procs(dut, used_procs.into());
        poke_io_cfg_in_2_used_procs(dut, used_procs.into());
        poke_io_cfg_in_3_used_procs(dut, used_procs.into());
        poke_io_cfg_in_4_used_procs(dut, used_procs.into());
        poke_io_cfg_in_5_used_procs(dut, used_procs.into());
        poke_io_cfg_in_6_used_procs(dut, used_procs.into());
        poke_io_cfg_in_7_used_procs(dut, used_procs.into());
        poke_io_cfg_in_8_used_procs(dut, used_procs.into());

        // Set SRAM configuration
        for (m, sram_cfg) in sram_cfgs.iter() {
            poke_sram_cfg(dut, m, sram_cfg);
        }

        // Do nothing for N steps
        for _ in 0..5 {
            step(dut, vcd, &mut cycle);
        }

        // Insert instructions
        while true {
            for (m, insts) in module_insts.iter_mut() {
                match m {
                    0 => {
                        if peek_io_insts_0_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    1 => {
                        if peek_io_insts_1_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    2 => {
                        if peek_io_insts_2_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    3 => {
                        if peek_io_insts_3_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    4 => {
                        if peek_io_insts_4_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    5 => {
                        if peek_io_insts_5_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    6 => {
                        if peek_io_insts_6_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    7 => {
                        if peek_io_insts_7_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    8 => {
                        if peek_io_insts_8_ready(dut) != 0 && !insts.is_empty() {
                            let inst = insts.pop_front().unwrap();
                            poke_inst_module(dut, m, &inst);
                        }
                    }
                    _ => {}
                }
            }
            step(dut, vcd, &mut cycle);
            if module_insts.values().all(|x| x.is_empty()) {
                break;
            }
        }

        // Wait until the init signal is high
        while peek_io_init(dut) == 0 {
            step(dut, vcd, &mut cycle);
        }

        'emulation_loop: for tcycle in 0..target_cycles {

            // Run emulator RTL
            for (coord, stim) in mapped_input_stimulti_blasted.iter_mut() {
                let bit = stim.pop_front().unwrap();
                poke_io_coord(dut, coord, bit as u64);
            }

            poke_io_run(dut, 1);
            for _hcycle in 0..host_steps {
                step(dut, vcd, &mut cycle);
            }

            poke_io_run(dut, 0);
            step(dut, vcd, &mut cycle);

            // Run functional simulator
            let input_stimuli_by_step = get_input_stimuli_by_step(
                &circuit,
                &input_stimuli_blasted,
                &all_signal_map,
                tcycle as u32);
            funct_sim.run_cycle(&input_stimuli_by_step);

            for (os, coord) in output_signals.iter() {
                let rtl_val = peek_io_coord(dut, coord);
                match funct_sim.peek(os) {
                    Some(bit) => {
                        if (bit as u64) != rtl_val {
                            mismatch_string = Some(format!(
                                    "Target cycle {} mismatch, got {} expect {}, signal {} coord {:?}",
                                    tcycle, rtl_val, bit, os, coord));
                            break 'emulation_loop;
                        }
                    }
                    None => { }
                }
            }
        }

        close_trace(vcd);
        Board_delete(dut);
    }
    match mismatch_string {
        Some(emsg) => Err(RTLSimError::from(emsg)),
        None       => Ok(())
    }
}

fn main() {
    let args = Args::parse();
    match test_rtl(&args) {
        Ok(_) => { println!("Test Success!"); }
        Err(emsg) => { println!("Test Failed {:?}", emsg); }
    }
}


#[cfg(test)]
pub mod emulator_rtl_test {
    use crate::test_rtl;
    use bee::common::config::Args;
    use test_case::test_case;

    fn test_emulator_rtl(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
    ) -> bool {
        let args = Args {
            verbose:            false,
            sim_dir:            format!("blif-sim-dir-{}", top_mod),
            sv_file_path:       sv_file_path.to_string(),
            top_mod:            top_mod.to_string(),
            input_stimuli_path: input_stimuli_path.to_string(),
            blif_file_path:     blif_file_path.to_string(),
            vcd:                None,
            instance_path:      "testharness.top".to_string(),
            clock_start_low:    false,
            timesteps_per_cycle: 2,
            ref_skip_cycles:    4,
            no_check_cycles:    0,
            check_cycle_period: 1,
            num_mods:           9,
            num_procs:          8,
            max_steps:          128,
            lut_inputs:         3,
            inter_proc_nw_lat:  0,
            inter_mod_nw_lat:   0,
            imem_lat:           1,
            dmem_rd_lat:        0,
            dmem_wr_lat:        1,
            sram_width:         16,
            sram_entries:       16,
            sram_rd_ports:      1,
            sram_wr_ports:      1,
            sram_rd_lat:        1,
            sram_wr_lat:        1,
            dbg_tail_length:    u32::MAX, // don't print debug graph when testing
            dbg_tail_threshold: u32::MAX  // don't print debug graph when testing
        };

        match test_rtl(&args) {
            Ok(_) => {
                println!("Test Success!");
                return true;
            }
            Err(emsg) => {
                println!("Test Failed {:?}", emsg);
                return false;
            }
        }
    }

// #[test_case("Core"; "Core")]
    #[test_case("Adder"; "Adder Test")]
    #[test_case("TestRegInit"; "TestRegInit Test")]
    #[test_case("Const"; "Const Test")]
    #[test_case("GCD"; "GCD Test")]
    #[test_case("ShiftReg"; "ShiftReg Test")]
    #[test_case("Fir"; "Fir Test")]
    #[test_case("MyQueue"; "MyQueue Test")]
    #[test_case("PointerChasing"; "PointerChasing Test")]
    #[test_case("SinglePortSRAM"; "SinglePortSRAM Test")]
    #[test_case("OneReadOneWritePortSRAM"; "OneReadOneWritePortSRAM Test")]
    pub fn test(top: &str) {
        assert_eq!(
            test_emulator_rtl(
                &format!("../examples/{}.sv", top),
                &top,
                &format!("../examples/{}.input", top),
                &format!("../examples/{}.lut.blif", top)),
                true
        );
    }
}
