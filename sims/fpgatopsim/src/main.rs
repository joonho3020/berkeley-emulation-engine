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

#[derive(Debug)]
struct Sim {
    pub dut: *mut VFPGATop,
    vcd: *mut VerilatedVcdC,
    cycle: u32
}

impl Sim {
    unsafe fn try_new() -> Self {
        let dut = FPGATop_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);
        Self {
            dut: dut,
            vcd: vcd,
            cycle: 0
        }
    }

    unsafe fn step(self: &mut Self) {
        let time = self.cycle * 2;
        FPGATop_eval(self.dut);
        dump_vcd(self.vcd, time);

        poke_clock(self.dut, 1);
        FPGATop_eval(self.dut);
        dump_vcd(self.vcd, time + 1);

        poke_clock(self.dut, 0);
        self.cycle += 1;
    }

    unsafe fn finish(self: &mut Self) {
        close_trace(self.vcd);
        FPGATop_delete(self.dut);
    }
}

#[derive(Default, Debug)]
pub struct AXI4AW {
    addr:  u32,
    id:    u32,
    len:   u32,
    size:  u32,
    burst: u32,
    lock:  bool,
    cache: bool,
    prot:  u32,
    qos:   u32
}

impl AXI4AW {
    fn from_addr_size(addr: u32, size: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct AXI4W {
    last: bool,
    data: Vec<u64>,
    strb: u32,
}

impl AXI4W {
    fn from_u32(data: u32, strb: u32) -> Self {
        Self {
            last: true,
            data: vec![data as u64],
            strb: strb
        }
    }
}

#[derive(Default, Debug)]
pub struct AXI4B {
    id:   u32,
    resp: u32
}

#[derive(Default, Debug)]
pub struct AXI4AR {
    addr:  u32,
    id:    u32,
    len:   u32,
    size:  u32,
    burst: u32,
    lock:  u32,
    cache: u32,
    prot:  u32,
    qos:   u32
}

impl AXI4AR {
    fn from_addr_size(addr: u32, size: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct AXI4R {
    id: u32,
    resp: u32,
    last: bool,
    data: Vec<u64>
}

unsafe fn poke_io_dma_axi4_master_aw(dut: *mut VFPGATop, aw: &AXI4AW) {
    poke_io_dma_axi4_master_aw_bits_id   (dut, aw.id.into());
    poke_io_dma_axi4_master_aw_bits_len  (dut, aw.len.into());
    poke_io_dma_axi4_master_aw_bits_size (dut, aw.size.into());
    poke_io_dma_axi4_master_aw_bits_burst(dut, aw.burst.into());
    poke_io_dma_axi4_master_aw_bits_lock (dut, aw.lock.into());
    poke_io_dma_axi4_master_aw_bits_cache(dut, aw.cache.into());
    poke_io_dma_axi4_master_aw_bits_prot (dut, aw.prot.into());
    poke_io_dma_axi4_master_aw_bits_qos  (dut, aw.qos.into());
}

unsafe fn poke_io_dma_axi4_master_w(dut: *mut VFPGATop, w: &AXI4W) {
    poke_io_dma_axi4_master_w_bits_last(dut, w.last.into());
    poke_io_dma_axi4_master_w_bits_data(dut, w.data.as_ptr());
    poke_io_dma_axi4_master_w_bits_strb(dut, w.strb.into());
}

unsafe fn peek_io_dma_axi4_master_b(dut: *mut VFPGATop) -> AXI4B {
    AXI4B {
        id:   peek_io_dma_axi4_master_b_bits_id  (dut) as u32,
        resp: peek_io_dma_axi4_master_b_bits_resp(dut) as u32
    }
}

unsafe fn poke_io_dma_axi4_master_ar(dut: *mut VFPGATop, ar: &AXI4AR) {
    poke_io_dma_axi4_master_ar_bits_id   (dut, ar.id.into());
    poke_io_dma_axi4_master_ar_bits_len  (dut, ar.len.into());
    poke_io_dma_axi4_master_ar_bits_size (dut, ar.size.into());
    poke_io_dma_axi4_master_ar_bits_burst(dut, ar.burst.into());
    poke_io_dma_axi4_master_ar_bits_lock (dut, ar.lock.into());
    poke_io_dma_axi4_master_ar_bits_cache(dut, ar.cache.into());
    poke_io_dma_axi4_master_ar_bits_prot (dut, ar.prot.into());
    poke_io_dma_axi4_master_ar_bits_qos  (dut, ar.qos.into());
}

unsafe fn peek_io_dma_axi4_master_r(dut: *mut VFPGATop) -> AXI4R {
    // This is fine as we already know the length of the AXI transaction
    // We can make this a bit more general later
    let mut rbuf = vec![0u64; 4];
    peek_io_dma_axi4_master_r_bits_data(dut, rbuf.as_mut_ptr());
    AXI4R {
        id:   peek_io_dma_axi4_master_r_bits_id  (dut) as u32,
        resp: peek_io_dma_axi4_master_r_bits_resp(dut) as u32,
        last: peek_io_dma_axi4_master_r_bits_last(dut) != 0,
        data: rbuf
    }
}

unsafe fn poke_io_mmio_axi4_master_aw(dut: *mut VFPGATop, aw: &AXI4AW) {
    poke_io_mmio_axi4_master_aw_bits_id   (dut, aw.id.into());
    poke_io_mmio_axi4_master_aw_bits_len  (dut, aw.len.into());
    poke_io_mmio_axi4_master_aw_bits_size (dut, aw.size.into());
    poke_io_mmio_axi4_master_aw_bits_burst(dut, aw.burst.into());
    poke_io_mmio_axi4_master_aw_bits_lock (dut, aw.lock.into());
    poke_io_mmio_axi4_master_aw_bits_cache(dut, aw.cache.into());
    poke_io_mmio_axi4_master_aw_bits_prot (dut, aw.prot.into());
    poke_io_mmio_axi4_master_aw_bits_qos  (dut, aw.qos.into());
}

unsafe fn poke_io_mmio_axi4_master_w(dut: *mut VFPGATop, w: &AXI4W) {
    poke_io_mmio_axi4_master_w_bits_last(dut, w.last.into());
    poke_io_mmio_axi4_master_w_bits_data(dut, *w.data.first().unwrap() as u64);
    poke_io_mmio_axi4_master_w_bits_strb(dut, w.strb.into());
}

unsafe fn peek_io_mmio_axi4_master_b(dut: *mut VFPGATop) -> AXI4B {
    AXI4B {
        id:   peek_io_mmio_axi4_master_b_bits_id  (dut) as u32,
        resp: peek_io_mmio_axi4_master_b_bits_resp(dut) as u32
    }
}

unsafe fn poke_io_mmio_axi4_master_ar(dut: *mut VFPGATop, ar: &AXI4AR) {
    poke_io_mmio_axi4_master_ar_bits_id   (dut, ar.id.into());
    poke_io_mmio_axi4_master_ar_bits_len  (dut, ar.len.into());
    poke_io_mmio_axi4_master_ar_bits_size (dut, ar.size.into());
    poke_io_mmio_axi4_master_ar_bits_burst(dut, ar.burst.into());
    poke_io_mmio_axi4_master_ar_bits_lock (dut, ar.lock.into());
    poke_io_mmio_axi4_master_ar_bits_cache(dut, ar.cache.into());
    poke_io_mmio_axi4_master_ar_bits_prot (dut, ar.prot.into());
    poke_io_mmio_axi4_master_ar_bits_qos  (dut, ar.qos.into());
}

unsafe fn peek_io_mmio_axi4_master_r(dut: *mut VFPGATop) -> AXI4R {
    AXI4R {
        id:   peek_io_mmio_axi4_master_r_bits_id  (dut) as u32,
        resp: peek_io_mmio_axi4_master_r_bits_resp(dut) as u32,
        last: peek_io_mmio_axi4_master_r_bits_last(dut) != 0,
        data: vec![peek_io_mmio_axi4_master_r_bits_data(dut)]
    }
}

unsafe fn mmio_read(
    sim: &mut Sim,
    addr: u32,
    size: u32
) -> u32 {
    // Wait until the ready signal is high
    while peek_io_mmio_axi4_master_ar_ready(sim.dut) == 0 {
        sim.step();
    }

    // Submit AXI request through AR channel
    let ar = AXI4AR::from_addr_size(addr, size);
    poke_io_mmio_axi4_master_ar(sim.dut, &ar);
    poke_io_mmio_axi4_master_ar_valid(sim.dut, true.into());

    // Assumes that the response will come at least one cycle after the request
    sim.step();
    poke_io_mmio_axi4_master_ar_valid(sim.dut, false.into());
    poke_io_mmio_axi4_master_r_ready(sim.dut, true.into());

    // Wait until we get the response
    while peek_io_mmio_axi4_master_r_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_mmio_axi4_master_r_ready(sim.dut, false.into());
    let r = peek_io_mmio_axi4_master_r(sim.dut);
    return *r.data.first().unwrap() as u32;
}

unsafe fn mmio_write(
    sim: &mut Sim,
    addr: u32,
    size: u32,
    data: u32,
    strb: u32
) {
    while peek_io_mmio_axi4_master_aw_ready(sim.dut) == 0 ||
          peek_io_mmio_axi4_master_w_ready (sim.dut) == 0 {
        sim.step();
    }

    let aw = AXI4AW::from_addr_size(addr, size);
    poke_io_mmio_axi4_master_aw(sim.dut, &aw);
    poke_io_mmio_axi4_master_aw_valid(sim.dut, true.into());

    let w = AXI4W::from_u32(data, strb);
    poke_io_mmio_axi4_master_w(sim.dut, &w);
    poke_io_mmio_axi4_master_w_valid(sim.dut, true.into());

    // Assumes that the response will come at least one cycle after the request
    sim.step();
    poke_io_mmio_axi4_master_aw_valid(sim.dut, false.into());
    poke_io_mmio_axi4_master_w_valid(sim.dut, false.into());
    poke_io_mmio_axi4_master_b_ready(sim.dut, true.into());

    // Wait until we get the response
    while peek_io_mmio_axi4_master_b_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_mmio_axi4_master_b_ready(sim.dut, false.into());
    let _b = peek_io_mmio_axi4_master_b(sim.dut);
}

fn main() {
    unsafe {
        let mut sim = Sim::try_new();
        let dut = FPGATop_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        poke_reset(sim.dut, 1);
        for _ in 0..100 {
            sim.step();
        }
        sim.finish();
    }
    println!("Hello, world!");
}
