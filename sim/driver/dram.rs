use crate::driver::axi::AXI4B;

use super::{axi::{AXI4Channels, AXI4AW, AXI4R}, harness::AXI4ReadyBits};



pub type Addr = u64;



#[derive(Debug, Default)]
pub struct DRAM {
    pub base_addr: Addr,
    pub word_size: u32,
    pub data: Vec<u8>,
    pub inflight_aw: Option<AXI4AW>,
    pub store_cnt: u32
}

impl DRAM {
    pub fn new(base_addr: Addr, size: Addr, word_size: u32) -> Self {
        Self {
            base_addr: base_addr,
            data: vec![0u8; size as usize],
            word_size: word_size,
            inflight_aw: None,
            store_cnt: 0
        }
    }

    /// Read a chunk of memory
    pub fn read(self: &Self, faddr: Addr) -> Vec<u8> {
// println!("dram read addr 0x{:x}", faddr);
        let addr = (faddr - self.base_addr) as usize;
        assert!(addr < self.data.len());
        return self.data[addr..addr + self.word_size as usize].to_vec();
    }

    /// Write a chunk of memory
    pub fn write(self: &mut Self, faddr: Addr, strb: u64, size: u64, data: &Vec<u8>) {
// println!("dram write addr 0x{:x} strb {:x} size {} data {:X?}", faddr, strb, size, data);

        let addr = (faddr - self.base_addr) as usize;
        assert!(addr < self.data.len());

        let max_strb_bytes = 64;
        assert!(size <= max_strb_bytes);

        let mut strb_ = if size != max_strb_bytes {
            strb & ((1 << size) - 1) << (addr % self.word_size as usize)
        } else {
            strb
        };

        let offset = (addr / self.word_size as usize) * self.word_size as usize;
        for i in 0..self.word_size {
            if strb_ & 1 == 1 {
                self.data[offset as usize + i as usize] = data[i as usize];
            }
            strb_ >>= 1;
        }
    }

    // TODO: add timing model
    pub fn step(self: &mut Self, axi: &mut AXI4Channels, axi_rdy: &mut AXI4ReadyBits) {
        if !axi.aw.is_empty() && axi_rdy.aw {
            self.inflight_aw = axi.aw.pop_front();
            axi_rdy.aw = false;
            axi_rdy.w  = true;
            axi_rdy.ar = false;
        }

        if !axi.w.is_empty() && axi_rdy.w {
            let w = axi.w.pop_front().unwrap();
            let aw = self.inflight_aw.clone().unwrap();
            let store_size = 1 << aw.size;
            self.write((aw.addr + self.store_cnt * store_size).into(), w.strb, store_size.into(), &w.data);
            self.store_cnt += 1;

            if self.store_cnt == aw.len + 1 {
                self.inflight_aw = None;
                self.store_cnt = 0;
                assert!(w.last);
                axi.b.push_back(AXI4B::from_id(aw.id));
                axi_rdy.w  = false;
                axi_rdy.aw = true;
                axi_rdy.ar = true;
            }
        }

        if !axi.ar.is_empty() && axi_rdy.ar {
            let ar = axi.ar.pop_front().unwrap();
            let start_addr = (ar.addr / self.word_size) * self.word_size;
            let req_len = ar.len + 1;
            let _req_size = 1 << ar.size;
            for i in 0..req_len {
                let read_data = self.read((start_addr + i * self.word_size).into());
                axi.r.push_back(AXI4R::from_id_data_last(ar.id, read_data, i == req_len - 1));
            }
        }
    }
}
