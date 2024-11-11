use crate::dut::*;
use bee::common::config::PlatformConfig;

#[derive(Debug, Default, Clone)]
pub struct AXI4Config {
    pub id_bits:   u32,
    pub addr_bits: u32,
    pub data_bits: u32,
}

impl AXI4Config {
    pub fn strb_bits(self: &Self) -> u32 {
        self.data_bits / 8
    }

    pub fn beat_bytes(self: &Self) -> u32 {
        self.strb_bits()
    }

    pub fn size(self: &Self) -> u32 {
        (self.strb_bits() as f32).log2().ceil() as u32
    }

    pub fn strb(self: &Self) -> u64 {
        ((1u64 << self.strb_bits()) - 1) as u64
    }
}

#[derive(Debug, Default, Clone)]
pub struct FPGATopConfig {
    pub axi:  AXI4Config,
    pub axil: AXI4Config,
    pub emul: PlatformConfig
}

#[derive(Debug)]
pub struct Sim {
    pub cfg: FPGATopConfig,
    pub dut: *mut VFPGATop,
    vcd: *mut VerilatedVcdC,
    cycle: u32
}

impl Sim {
    pub const MAX_LEN: u32 = 255;

    pub unsafe fn try_new(cfg: &FPGATopConfig) -> Self {
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

    pub unsafe fn step(self: &mut Self) {
        let time = self.cycle * 2;
        FPGATop_eval(self.dut);
        dump_vcd(self.vcd, time);

        poke_clock(self.dut, 1);
        FPGATop_eval(self.dut);
        dump_vcd(self.vcd, time + 1);

        poke_clock(self.dut, 0);
        self.cycle += 1;
    }

    pub unsafe fn finish(self: &mut Self) {
        close_trace(self.vcd);
        FPGATop_delete(self.dut);
    }

    pub fn max_len(self: &Self) -> u32 {
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
    pub fn from_addr_size(addr: u32, size: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            ..Self::default()
        }
    }

    pub fn from_addr_size_len(addr: u32, size: u32, len: u32) -> Self {
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
    pub fn from_u32(data: u32, strb: u64) -> Self {
        Self {
            last: true,
            data: data.to_le_bytes().to_vec(),
            strb: strb
        }
    }

    pub fn from_data_strb_last(data: &Vec<u8>, strb: u64, last: bool) -> Self {
        Self {
            last: last,
            data: data.clone(),
            strb: strb
        }
    }

    pub fn data_vec_u32(self: &Self) -> Vec<u32> {
        let vec_u32: Vec<u32> = self.data
            .chunks(4)
            .map(|chunk| {
                let bytes = <[u8; 4]>::try_from(chunk).expect("Chunk must be 4 bytes");
                u32::from_le_bytes(bytes)
            })
            .collect();
        return vec_u32;
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
    pub fn from_addr_size(addr: u32, size: u32) -> Self {
        Self {
            addr: addr,
            size: size,
            ..Self::default()
        }
    }

    pub fn from_addr_size_len(addr: u32, size: u32, len: u32) -> Self {
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

pub unsafe fn poke_io_dma_axi4_master_aw(dut: *mut VFPGATop, aw: &AXI4AW) {
    poke_io_dma_axi4_master_aw_bits_addr (dut, aw.addr.into());
    poke_io_dma_axi4_master_aw_bits_id   (dut, aw.id.into());
    poke_io_dma_axi4_master_aw_bits_len  (dut, aw.len.into());
    poke_io_dma_axi4_master_aw_bits_size (dut, aw.size.into());
    poke_io_dma_axi4_master_aw_bits_burst(dut, aw.burst.into());
    poke_io_dma_axi4_master_aw_bits_lock (dut, aw.lock.into());
    poke_io_dma_axi4_master_aw_bits_cache(dut, aw.cache.into());
    poke_io_dma_axi4_master_aw_bits_prot (dut, aw.prot.into());
    poke_io_dma_axi4_master_aw_bits_qos  (dut, aw.qos.into());
}

pub unsafe fn poke_io_dma_axi4_master_w(dut: *mut VFPGATop, w: &AXI4W) {
    poke_io_dma_axi4_master_w_bits_last(dut, w.last.into());
    poke_io_dma_axi4_master_w_bits_data(dut, w.data_vec_u32().as_ptr());
    poke_io_dma_axi4_master_w_bits_strb(dut, w.strb.into());
}

pub unsafe fn peek_io_dma_axi4_master_b(dut: *mut VFPGATop) -> AXI4B {
    AXI4B {
        id:   peek_io_dma_axi4_master_b_bits_id  (dut) as u32,
        resp: peek_io_dma_axi4_master_b_bits_resp(dut) as u32
    }
}

pub unsafe fn poke_io_dma_axi4_master_ar(dut: *mut VFPGATop, ar: &AXI4AR) {
    poke_io_dma_axi4_master_ar_bits_addr (dut, ar.addr.into());
    poke_io_dma_axi4_master_ar_bits_id   (dut, ar.id.into());
    poke_io_dma_axi4_master_ar_bits_len  (dut, ar.len.into());
    poke_io_dma_axi4_master_ar_bits_size (dut, ar.size.into());
    poke_io_dma_axi4_master_ar_bits_burst(dut, ar.burst.into());
    poke_io_dma_axi4_master_ar_bits_lock (dut, ar.lock.into());
    poke_io_dma_axi4_master_ar_bits_cache(dut, ar.cache.into());
    poke_io_dma_axi4_master_ar_bits_prot (dut, ar.prot.into());
    poke_io_dma_axi4_master_ar_bits_qos  (dut, ar.qos.into());
}

pub unsafe fn peek_io_dma_axi4_master_r(dut: *mut VFPGATop) -> AXI4R {
    // This is fine as we already know the length of the AXI transaction
    // We can make this a bit more general later
    let mut rbuf = vec![0u32; 16];
    peek_io_dma_axi4_master_r_bits_data(dut, rbuf.as_mut_ptr());
    let rbuf_u8 = rbuf.iter()
        .flat_map(|&num| num.to_le_bytes())
        .collect();

    AXI4R {
        id:   peek_io_dma_axi4_master_r_bits_id  (dut) as u32,
        resp: peek_io_dma_axi4_master_r_bits_resp(dut) as u32,
        last: peek_io_dma_axi4_master_r_bits_last(dut) != 0,
        data: rbuf_u8
    }
}

pub unsafe fn poke_io_mmio_axi4_master_aw(dut: *mut VFPGATop, aw: &AXI4AW) {
    poke_io_mmio_axi4_master_aw_bits_addr (dut, aw.addr.into());
    poke_io_mmio_axi4_master_aw_bits_id   (dut, aw.id.into());
    poke_io_mmio_axi4_master_aw_bits_len  (dut, aw.len.into());
    poke_io_mmio_axi4_master_aw_bits_size (dut, aw.size.into());
    poke_io_mmio_axi4_master_aw_bits_burst(dut, aw.burst.into());
    poke_io_mmio_axi4_master_aw_bits_lock (dut, aw.lock.into());
    poke_io_mmio_axi4_master_aw_bits_cache(dut, aw.cache.into());
    poke_io_mmio_axi4_master_aw_bits_prot (dut, aw.prot.into());
    poke_io_mmio_axi4_master_aw_bits_qos  (dut, aw.qos.into());
}

pub unsafe fn poke_io_mmio_axi4_master_w(dut: *mut VFPGATop, w: &AXI4W) {
    assert!(w.data.len() == 4);
    let arr = [w.data[0], w.data[1], w.data[2], w.data[3]];
    let data = u32::from_le_bytes(arr);

    poke_io_mmio_axi4_master_w_bits_last(dut, w.last.into());
    poke_io_mmio_axi4_master_w_bits_data(dut, data as u64);
    poke_io_mmio_axi4_master_w_bits_strb(dut, w.strb.into());
}

pub unsafe fn peek_io_mmio_axi4_master_b(dut: *mut VFPGATop) -> AXI4B {
    AXI4B {
        id:   peek_io_mmio_axi4_master_b_bits_id  (dut) as u32,
        resp: peek_io_mmio_axi4_master_b_bits_resp(dut) as u32
    }
}

pub unsafe fn poke_io_mmio_axi4_master_ar(dut: *mut VFPGATop, ar: &AXI4AR) {
    poke_io_mmio_axi4_master_ar_bits_addr (dut, ar.addr.into());
    poke_io_mmio_axi4_master_ar_bits_id   (dut, ar.id.into());
    poke_io_mmio_axi4_master_ar_bits_len  (dut, ar.len.into());
    poke_io_mmio_axi4_master_ar_bits_size (dut, ar.size.into());
    poke_io_mmio_axi4_master_ar_bits_burst(dut, ar.burst.into());
    poke_io_mmio_axi4_master_ar_bits_lock (dut, ar.lock.into());
    poke_io_mmio_axi4_master_ar_bits_cache(dut, ar.cache.into());
    poke_io_mmio_axi4_master_ar_bits_prot (dut, ar.prot.into());
    poke_io_mmio_axi4_master_ar_bits_qos  (dut, ar.qos.into());
}

pub unsafe fn peek_io_mmio_axi4_master_r(dut: *mut VFPGATop) -> AXI4R {
    AXI4R {
        id:   peek_io_mmio_axi4_master_r_bits_id  (dut) as u32,
        resp: peek_io_mmio_axi4_master_r_bits_resp(dut) as u32,
        last: peek_io_mmio_axi4_master_r_bits_last(dut) != 0,
        data: peek_io_mmio_axi4_master_r_bits_data(dut).to_le_bytes().to_vec()
    }
}

pub unsafe fn mmio_read(
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

    sim.step();

    // Wait until we get the response
    while peek_io_mmio_axi4_master_r_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_mmio_axi4_master_r_ready(sim.dut, false.into());
    let r = peek_io_mmio_axi4_master_r(sim.dut);
    sim.step();
    return *r.data.first().unwrap() as u32;
}

pub unsafe fn mmio_write(
    sim: &mut Sim,
    addr: u32,
    data: u32
) {
    println!("mmio write addr: {} data {}", addr, data);
    println!("mmio write waiting for aw/w ready");
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
    sim.step();

    println!("mmio write waiting for b valid");
    // Wait until we get the response
    while peek_io_mmio_axi4_master_b_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_mmio_axi4_master_b_ready(sim.dut, false.into());
    let _b = peek_io_mmio_axi4_master_b(sim.dut);
    println!("mmio write done");
    sim.step();
}

pub unsafe fn dma_read_req(
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

pub unsafe fn dma_read(
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

pub unsafe fn dma_write_req(
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
    poke_io_dma_axi4_master_aw(sim.dut, &aw);
    poke_io_dma_axi4_master_aw_valid(sim.dut, true.into());
    sim.step();

    poke_io_dma_axi4_master_aw_valid(sim.dut, false.into());
    sim.step();

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
        sim.step();

        // Wait until we get the response
        while peek_io_dma_axi4_master_b_valid(sim.dut) == 0 {
            sim.step();
        }
        sim.step();
        poke_io_dma_axi4_master_b_ready(sim.dut, false.into());
        let b = peek_io_dma_axi4_master_b(sim.dut);
        sim.step();
    }
}

pub unsafe fn dma_write(
    sim: &mut Sim,
    addr: u32,
    size: u32,
    data: &Vec<u8>) {

    let beat_bytes = sim.cfg.axi.beat_bytes();
    let beat_bytes_log2 = (beat_bytes as f32).log2() as u32;
    let mut len: i32 = ((size - 1) / beat_bytes) as i32;
    let remaining = size - (len as u32) * beat_bytes;

    let mut strb: Vec<u64> = vec![];
    for _ in 0..len {
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
