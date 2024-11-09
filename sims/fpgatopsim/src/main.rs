pub mod dut;
use bee::{
    common::{
        circuit::Circuit, config::{Args, PlatformConfig}, hwgraph::NodeMapInfo, instruction::*, mapping::{SRAMMapping, SRAMPortType}, network::Coordinate, primitive::{Bit, Primitive}
    }, fsim::board::Board, rtlsim::rtlsim_utils::{get_input_stimuli_blasted, InputStimuliMap}, testing::try_new_circuit
};
use clap::Parser;
use dut::*;

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

#[derive(Debug, Default, Clone)]
struct AXI4Config {
    id_bits:   u32,
    addr_bits: u32,
    data_bits: u32,
}

impl AXI4Config {
    fn strb_bits(self: &Self) -> u32 {
        self.data_bits / 8
    }

    fn beat_bytes(self: &Self) -> u32 {
        self.strb_bits()
    }

    fn size(self: &Self) -> u32 {
        (self.strb_bits() as f32).log2().ceil() as u32
    }

    fn strb(self: &Self) -> u64 {
        ((1u64 << self.strb_bits()) - 1) as u64
    }
}

#[derive(Debug, Default, Clone)]
struct FPGATopConfig {
    axi:  AXI4Config,
    axil: AXI4Config,
    emul: PlatformConfig
}

#[derive(Debug)]
struct Sim {
    cfg: FPGATopConfig,
    dut: *mut VFPGATop,
    vcd: *mut VerilatedVcdC,
    cycle: u32
}

impl Sim {
    pub const MAX_LEN: u32 = 255;

    unsafe fn try_new(cfg: &FPGATopConfig) -> Self {
        let dut = FPGATop_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);
        Self {
            cfg: cfg.clone(),
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

    fn max_len(self: &Self) -> u32 {
        Self::MAX_LEN
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

    fn from_addr_size_len(addr: u32, size: u32, len: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            len: len,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct AXI4W {
    last: bool,
    data: Vec<u8>,
    strb: u64,
}

impl AXI4W {
    fn from_u32(data: u32, strb: u64) -> Self {
        Self {
            last: true,
            data: data.to_le_bytes().to_vec(),
            strb: strb
        }
    }

    fn from_data_strb_last(data: &Vec<u8>, strb: u64, last: bool) -> Self {
        Self {
            last: last,
            data: data.clone(),
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

    fn from_addr_size_len(addr: u32, size: u32, len: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            len: len,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct AXI4R {
    id: u32,
    resp: u32,
    last: bool,
    data: Vec<u8>
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
    let mut rbuf = vec![0u8; 64];
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
    assert!(w.data.len() == 4);
    let arr = [w.data[0], w.data[1], w.data[2], w.data[3]];
    let data = u32::from_le_bytes(arr);

    poke_io_mmio_axi4_master_w_bits_last(dut, w.last.into());
    poke_io_mmio_axi4_master_w_bits_data(dut, data as u64);
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
        data: peek_io_mmio_axi4_master_r_bits_data(dut).to_le_bytes().to_vec()
    }
}

unsafe fn mmio_read(
    sim: &mut Sim,
    addr: u32,
) -> u32 {
    // Wait until the ready signal is high
    while peek_io_mmio_axi4_master_ar_ready(sim.dut) == 0 {
        sim.step();
    }

    // Submit AXI request through AR channel
    let ar = AXI4AR::from_addr_size(addr, 2);
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
    data: u32
) {
    while peek_io_mmio_axi4_master_aw_ready(sim.dut) == 0 ||
          peek_io_mmio_axi4_master_w_ready (sim.dut) == 0 {
        sim.step();
    }

    let aw = AXI4AW::from_addr_size(addr, 2);
    poke_io_mmio_axi4_master_aw(sim.dut, &aw);
    poke_io_mmio_axi4_master_aw_valid(sim.dut, true.into());

    let w = AXI4W::from_u32(data, sim.cfg.axil.strb());
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
    sim.step();
}

unsafe fn dma_read_req(
    sim: &mut Sim,
    addr: u32,
    size: u32,
    len: u32,
    data: &mut Vec<u8>) {

    while peek_io_dma_axi4_master_ar_ready(sim.dut) == 0 {
        sim.step();
    }

    // Send request
    let ar = AXI4AR::from_addr_size_len(addr, size, len);
    poke_io_dma_axi4_master_ar(sim.dut, &ar);
    poke_io_dma_axi4_master_ar_valid(sim.dut, true.into());

    sim.step();
    poke_io_dma_axi4_master_ar_valid(sim.dut, false.into());
    poke_io_dma_axi4_master_r_ready(sim.dut, true.into());

    // Receive response
    let word_size = 1 << ar.size;
    for i in 0..(ar.len + 1) {
        // Wait until we get the response
        while peek_io_dma_axi4_master_r_valid(sim.dut) == 0 {
            sim.step();
        }
        let r = peek_io_dma_axi4_master_r(sim.dut);
        assert!(i < ar.len || r.last);
        assert!(r.data.len() >= word_size);
        for x in 0..word_size {
            data.push(r.data[x]);
        }
        sim.step();
    }
    poke_io_dma_axi4_master_r_ready(sim.dut, false.into());
}

unsafe fn dma_read(
    sim: &mut Sim,
    addr: u32,
    size: u32) -> Vec<u8> {
    let beat_bytes = sim.cfg.axi.beat_bytes();
    let mut len: i32 = ((size - 1) / beat_bytes) as i32;
    let mut addr_ = addr;
    let beat_bytes_log2 = (beat_bytes as f32).log2() as u32;

    let mut ret = vec![];
    while len >= 0 {
        let part_len = len as u32 % (sim.max_len() + 1);
        let mut data: Vec<u8> = vec![];
        dma_read_req(sim, addr_, beat_bytes_log2, part_len, &mut data);

        len   -= (part_len + 1) as i32;
        addr_ += (part_len + 1) * beat_bytes;
        ret.extend(data);
    }
    return ret;
}

unsafe fn dma_write_req(
    sim: &mut Sim,
    addr: u32,
    size: u32,
    len: u32,
    data: &Vec<u8>,
    strb: &Vec<u64>) {
    while peek_io_dma_axi4_master_aw_ready(sim.dut) == 0 {
        sim.step();
    }

    let aw = AXI4AW::from_addr_size_len(addr, size, len);
    poke_io_mmio_axi4_master_aw(sim.dut, &aw);
    poke_io_mmio_axi4_master_aw_valid(sim.dut, true.into());

    sim.step();

    poke_io_mmio_axi4_master_aw_valid(sim.dut, false.into());

    let nbytes = 1 << size;
    for i in 0..(len + 1) {
        while peek_io_dma_axi4_master_w_ready(sim.dut) == 0 {
            sim.step();
        }

        let start = (i * nbytes) as usize;
        let end = start + nbytes as usize;
        let w = AXI4W::from_data_strb_last(&data[start..end].to_vec(), strb[i as usize], i == len);
        poke_io_dma_axi4_master_w(sim.dut, &w);
        poke_io_dma_axi4_master_w_valid(sim.dut, true.into());

        sim.step();
        poke_io_dma_axi4_master_w_valid(sim.dut, false.into());
        poke_io_dma_axi4_master_b_ready(sim.dut, true.into());

        // Wait until we get the response
        while peek_io_dma_axi4_master_b_valid(sim.dut) == 0 {
            sim.step();
        }
        poke_io_dma_axi4_master_b_ready(sim.dut, false.into());
        let _b = peek_io_dma_axi4_master_b(sim.dut);
        sim.step();
    }
}

unsafe fn dma_write(
    sim: &mut Sim,
    addr: u32,
    size: u32,
    data: &Vec<u8>) {
    let beat_bytes = sim.cfg.axi.beat_bytes();
    let beat_bytes_log2 = (beat_bytes as f32).log2() as u32;
    let mut len: i32 = ((size - 1) / beat_bytes) as i32;
    let mut remaining = size - (len as u32) * beat_bytes;

    let mut strb: Vec<u64> = vec![];
    for i in 0..len {
        let x = if beat_bytes > 63 { u64::MAX } else { (1u64 << beat_bytes) - 1 };
        strb.push(x);
    }

    if remaining == beat_bytes && len > 0 {
        strb.push(*strb.first().unwrap());
    } else if remaining == beat_bytes {
        strb.push(u64::MAX);
    } else {
        strb.push((1 << remaining) - 1);
    }

    let mut addr_ = addr;
    let mut idx: usize = 0;
    while len >= 0 {
        let part_len = len as u32 % (sim.max_len() + 1);
        let start = idx * beat_bytes as usize;
        let end = start + ((part_len + 1) * beat_bytes) as usize;

        dma_write_req(sim,
            addr_,
            beat_bytes_log2,
            part_len,
            &data[start..end].to_vec(),
            &strb[idx..idx + part_len as usize + 1].to_vec());

        idx += part_len as usize + 1;
        addr_ += (part_len + 1) * beat_bytes;
        len -= (part_len + 1) as i32;
    }
}


fn main() {
    // TODO: Set proper configurations
    let fpga_top_cfg = FPGATopConfig::default();

    let host_steps = 128;

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

        let num_mods = fpga_top_cfg.emul.num_mods;

        for m in 0..fpga_top_cfg.emul.num_mods {
            let used_procs = 8;
            mmio_write(&mut sim, m * 4,              used_procs);

            let single_port_ram = 0;
            mmio_write(&mut sim, (m + num_mods) * 4, single_port_ram);

            let wmask_bits = 0;
            mmio_write(&mut sim, (m + 2 * num_mods) * 4, wmask_bits);

            let width_bits = 0;
            mmio_write(&mut sim, (m + 3 * num_mods) * 4, width_bits);
        }

        mmio_write(&mut sim, (4 * num_mods) * 4, host_steps);

        // TODO: FOR ALL INSTS
        dma_write(&mut sim, 4096, 8, &vec![0u8; 8]);

        while mmio_read(&mut sim, (4 * num_mods + 1) * 4)  == 0 {
            sim.step();
        }


        // for target cycles {
        //    dma_write()
        //    while mmio_read(&mut sim, (4 * num_mods + 2) * 4) == 0 {
        //        sim.step();
        //    }
        //
        //    dma_read()
        // }


        sim.finish();
    }
    println!("Hello, world!");
}
