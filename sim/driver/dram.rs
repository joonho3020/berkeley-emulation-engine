

pub type Addr = u64;

pub struct DRAM {
    pub base_addr: Addr,
    pub word_size: u32,
    pub data: Vec<u8>
}

impl DRAM {
    pub fn new(base_addr: Addr, size: Addr, word_size: u32) -> Self {
        Self {
            base_addr: base_addr,
            data: vec![0u8, size as usize],
            word_size: word_size
        }
    }

    pub fn read(self: &Self, faddr: Addr) -> Vec<u8> {
        let addr = faddr - self.base_addr;
        assert!(addr as usize < self.data.len());
        return self.data[addr..addr + self.word_size];
    }

    pub fn write(self: &mut Self, faddr: Addr, strb: u64, size: u64, data: &Vec<u8>) {
        let addr = faddr - self.base_addr;
        assert!(addr as usize < self.data.len());

        let max_strb_bytes = 64;
        assert!(size <= max_strb_bytes);

        let mut strb_ = if size != max_strb_bytes {
            strb & ((1 << size) - 1) << (addr % self.word_size)
        } else {
            strb
        };

        let offset = (addr / self.word_size) * self.word_size;
        for i in 0..self.word_size {
            if strb_ & 1 {
                self.data[offset + i] = data[i];
            }
            strb_ >>= 1;
        }
    }
}
