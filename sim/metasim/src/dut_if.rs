use crate::dut::*;
use crate::axi::*;
use crate::sim::*;
use crate::simif::simif::*;

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
    return u32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]]);
}

pub unsafe fn mmio_write(
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
    sim.step();

    // Wait until we get the response
    while peek_io_mmio_axi4_master_b_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_mmio_axi4_master_b_ready(sim.dut, false.into());
    let _b = peek_io_mmio_axi4_master_b(sim.dut);
    sim.step();
}


pub unsafe fn poke_io_clkwiz_ctrl_ctrl_axil_aw(dut: *mut VFPGATop, aw: &AXI4AW) {
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_addr (dut, aw.addr.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_id   (dut, aw.id.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_len  (dut, aw.len.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_size (dut, aw.size.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_burst(dut, aw.burst.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_lock (dut, aw.lock.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_cache(dut, aw.cache.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_prot (dut, aw.prot.into());
    poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_qos  (dut, aw.qos.into());
}

pub unsafe fn poke_io_clkwiz_ctrl_ctrl_axil_w(dut: *mut VFPGATop, w: &AXI4W) {
    assert!(w.data.len() == 4);
    let arr = [w.data[0], w.data[1], w.data[2], w.data[3]];
    let data = u32::from_le_bytes(arr);

    poke_io_clkwiz_ctrl_ctrl_axil_w_bits_last(dut, w.last.into());
    poke_io_clkwiz_ctrl_ctrl_axil_w_bits_data(dut, data as u64);
    poke_io_clkwiz_ctrl_ctrl_axil_w_bits_strb(dut, w.strb.into());
}

pub unsafe fn peek_io_clkwiz_ctrl_ctrl_axil_b(dut: *mut VFPGATop) -> AXI4B {
    AXI4B {
        id:   peek_io_clkwiz_ctrl_ctrl_axil_b_bits_id  (dut) as u32,
        resp: peek_io_clkwiz_ctrl_ctrl_axil_b_bits_resp(dut) as u32
    }
}

pub unsafe fn poke_io_clkwiz_ctrl_ctrl_axil_ar(dut: *mut VFPGATop, ar: &AXI4AR) {
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_addr (dut, ar.addr.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_id   (dut, ar.id.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_len  (dut, ar.len.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_size (dut, ar.size.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_burst(dut, ar.burst.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_lock (dut, ar.lock.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_cache(dut, ar.cache.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_prot (dut, ar.prot.into());
    poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_qos  (dut, ar.qos.into());
}

pub unsafe fn peek_io_clkwiz_ctrl_ctrl_axil_r(dut: *mut VFPGATop) -> AXI4R {
    AXI4R {
        id:   peek_io_clkwiz_ctrl_ctrl_axil_r_bits_id  (dut) as u32,
        resp: peek_io_clkwiz_ctrl_ctrl_axil_r_bits_resp(dut) as u32,
        last: peek_io_clkwiz_ctrl_ctrl_axil_r_bits_last(dut) != 0,
        data: peek_io_clkwiz_ctrl_ctrl_axil_r_bits_data(dut).to_le_bytes().to_vec()
    }
}

pub unsafe fn clkwiz_ctrl_read(
    sim: &mut Sim,
    addr: u32,
) -> u32 {
    // Wait until the ready signal is high
    while peek_io_clkwiz_ctrl_ctrl_axil_ar_ready(sim.dut) == 0 {
        sim.step();
    }

    // Submit AXI request through AR channel
    let ar = AXI4AR::from_addr_size(addr, 2);

    poke_io_clkwiz_ctrl_ctrl_axil_ar(sim.dut, &ar);
    poke_io_clkwiz_ctrl_ctrl_axil_ar_valid(sim.dut, true.into());

    // Assumes that the response will come at least one cycle after the request
    sim.step();
    poke_io_clkwiz_ctrl_ctrl_axil_ar_valid(sim.dut, false.into());
    poke_io_clkwiz_ctrl_ctrl_axil_r_ready(sim.dut, true.into());

    sim.step();

    // Wait until we get the response
    while peek_io_clkwiz_ctrl_ctrl_axil_r_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_clkwiz_ctrl_ctrl_axil_r_ready(sim.dut, false.into());
    let r = peek_io_clkwiz_ctrl_ctrl_axil_r(sim.dut);
    sim.step();
    return u32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]]);
}

pub unsafe fn clkwiz_ctrl_write(
    sim: &mut Sim,
    addr: u32,
    data: u32
) {
    while peek_io_clkwiz_ctrl_ctrl_axil_aw_ready(sim.dut) == 0 ||
          peek_io_clkwiz_ctrl_ctrl_axil_w_ready (sim.dut) == 0 {
        sim.step();
    }

    let aw = AXI4AW::from_addr_size(addr, 2);
    poke_io_clkwiz_ctrl_ctrl_axil_aw(sim.dut, &aw);
    poke_io_clkwiz_ctrl_ctrl_axil_aw_valid(sim.dut, true.into());

    let w = AXI4W::from_u32(data, sim.cfg.axil.strb());
    poke_io_clkwiz_ctrl_ctrl_axil_w(sim.dut, &w);
    poke_io_clkwiz_ctrl_ctrl_axil_w_valid(sim.dut, true.into());

    // Assumes that the response will come at least one cycle after the request
    sim.step();
    poke_io_clkwiz_ctrl_ctrl_axil_aw_valid(sim.dut, false.into());
    poke_io_clkwiz_ctrl_ctrl_axil_w_valid(sim.dut, false.into());
    poke_io_clkwiz_ctrl_ctrl_axil_b_ready(sim.dut, true.into());
    sim.step();

    // Wait until we get the response
    while peek_io_clkwiz_ctrl_ctrl_axil_b_valid(sim.dut) == 0 {
        sim.step();
    }

    poke_io_clkwiz_ctrl_ctrl_axil_b_ready(sim.dut, false.into());
    let _b = peek_io_clkwiz_ctrl_ctrl_axil_b(sim.dut);
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
    data: &mut Vec<u8>,
    size: u32) {
    let beat_bytes = sim.cfg.axi.beat_bytes();
    let mut len: i32 = ((size - 1) / beat_bytes) as i32;
    let mut addr_ = addr;
    let beat_bytes_log2 = (beat_bytes as f32).log2() as u32;

    while len >= 0 {
        let part_len = len as u32 % (sim.max_len() + 1);
        let mut partial_read: Vec<u8> = vec![];
        dma_read_req(sim, addr_, beat_bytes_log2, part_len, &mut partial_read);

        let start = addr_ - addr;
        let end = start + partial_read.len() as u32;
        for i in start..end {
            data[i as usize] = partial_read[(i - start) as usize];
        }

        len   -= (part_len + 1) as i32;
        addr_ += (part_len + 1) * beat_bytes;
    }
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
    }

    // Wait until we get the response
    while peek_io_dma_axi4_master_b_valid(sim.dut) == 0 {
        sim.step();
    }
    sim.step();
    poke_io_dma_axi4_master_b_ready(sim.dut, false.into());
    let _b = peek_io_dma_axi4_master_b(sim.dut);
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
// println!("len: {} part_len: {} start: {} end: {}",
// len, part_len, start, end);

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
