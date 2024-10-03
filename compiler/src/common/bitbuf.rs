use std::cmp::min;

#[derive(Debug, Default, Clone)]
pub struct BitBuf {
    pub bytes: Vec<u8>,
    pub offset: u32,
    pub size: u32,
}

impl BitBuf {
    pub fn push_bits(self: &mut Self, input: u64, nbits: u32) {
        let mut left = nbits;
        while left > 0 {
            if self.offset == 0 {
                self.bytes.push(0);
            }
            let cur_input = (input >> (nbits - left)) as u8;
            let free_bits = 8 - self.offset;
            let consume_bits = min(free_bits, left);

            let last = self.bytes.last_mut().unwrap();
            *last |= (cur_input << self.offset) as u8;

            self.offset = (self.offset + consume_bits) % 8;
            left -= consume_bits;
        }
        self.size += nbits;
    }
}

