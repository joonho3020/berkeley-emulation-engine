
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

#[derive(Default, Debug)]
pub struct AXI4AW {
    pub addr:  u32,
    pub id:    u32,
    pub len:   u32,
    pub size:  u32,
    pub burst: u32,
    pub lock:  bool,
    pub cache: bool,
    pub prot:  u32,
    pub qos:   u32
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
    pub last: bool,
    pub data: Vec<u8>,
    pub strb: u64,
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
    pub id:   u32,
    pub resp: u32
}

#[derive(Default, Debug)]
pub struct AXI4AR {
    pub addr:  u32,
    pub id:    u32,
    pub len:   u32,
    pub size:  u32,
    pub burst: u32,
    pub lock:  u32,
    pub cache: u32,
    pub prot:  u32,
    pub qos:   u32
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
    pub id: u32,
    pub resp: u32,
    pub last: bool,
    pub data: Vec<u8>
}
