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

unsafe fn step(dut: *mut VFPGATop, vcd: *mut VerilatedVcdC, cycle: &mut u32) {
    let time = *cycle * 2;
    FPGATop_eval(dut);
    dump_vcd(vcd, time);

    poke_clock(dut, 1);
    FPGATop_eval(dut);
    dump_vcd(vcd, time + 1);

    poke_clock(dut, 0);
    *cycle += 1;
}

fn main() {
    let mut cycle = 0;
    unsafe {
        let dut = FPGATop_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);
        poke_reset(dut, 1);
        for _ in 0..5 {
            step(dut, vcd, &mut cycle);
        }
    }
    println!("Hello, world!");
}
