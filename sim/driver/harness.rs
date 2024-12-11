use crate::driver::axi::*;
use crate::driver::dram::*;
use crate::driver::tsi::*;
use crate::simif::simif::Driver;
use crate::simif::dmaif::*;
use crate::SimIfErr;
use std::collections::VecDeque;
use indexmap::IndexMap;
use bee::common::{
        network::Coordinate,
        config::PlatformConfig
};
use bitvec::{order::Lsb0, vec::BitVec};
use fesvr::Htif;
use derivative::Derivative;
use super::driver::FPGATopConfig;

/// Helper function to split `name[idx]` into a tuple `(name, idx)`.
/// For instance,: `data[0]` will returns a tuple `(data, 0)`.
fn split_indexed_field(input: &str) -> Result<(&str, u32), String> {
    if let Some(open_bracket_pos) = input.find('[') {
        if let Some(close_bracket_pos) = input.find(']') {
            let name = &input[..open_bracket_pos];
            let number_str = &input[open_bracket_pos + 1..close_bracket_pos];
            match number_str.parse::<u32>() {
                Ok(number) => {
                    return Ok((name, number));
                }
                Err(e) => {
                    return Err(format!("Failed to parse: {}", e));
                }
            }
        } else {
            return Err(format!("Closing bracket ']' not found."));
        }
    } else {
        return Ok((input, 0));
    }
}

/// Maps signals comming out from the target to a emulation platform `Coordinate`
/// - Example signals: `mem_axi4_0_aw_valid`, `mem_axi4_0_aw_bits_id[0]`
#[derive(Debug, Default)]
pub struct AXI4TargetOutIdx {
    pub aw_valid: usize,
    pub aw_addr: Vec<usize>,
    pub aw_id:   Vec<usize>,
    pub aw_len:  Vec<usize>,
    pub aw_size: Vec<usize>,

    pub w_valid: usize,
    pub w_strb: Vec<usize>,
    pub w_data: Vec<usize>,
    pub w_last: usize,

    pub b_ready: usize,

    pub ar_valid: usize,
    pub ar_addr: Vec<usize>,
    pub ar_id:   Vec<usize>,
    pub ar_len:  Vec<usize>,
    pub ar_size: Vec<usize>,

    pub r_ready: usize,
}

impl AXI4TargetOutIdx {
    fn new(pfx: &str, output_signals: &IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = AXI4TargetOutIdx::default();

        let mut aw_addr_idx: IndexMap<u32, u32> = IndexMap::new();
        let mut aw_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let mut aw_len_idx:  IndexMap<u32, u32> = IndexMap::new();
        let mut aw_size_idx: IndexMap<u32, u32> = IndexMap::new();

        let mut w_strb_idx: IndexMap<u32, u32> = IndexMap::new();
        let mut w_data_idx: IndexMap<u32, u32> = IndexMap::new();

        let mut ar_addr_idx: IndexMap<u32, u32> = IndexMap::new();
        let mut ar_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let mut ar_len_idx:  IndexMap<u32, u32> = IndexMap::new();
        let mut ar_size_idx: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in output_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let mut split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown AXI channel: {}, split: {:?}", name, split);
                }

                let channel = split.pop_front().unwrap().to_lowercase();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase();

                match rdy_val_bits.as_str() {
                    "valid" => {
                        match channel.as_str() {
                            "aw" => { ret.aw_valid = coord.id(pcfg) as usize; }
                            "w"  => { ret.w_valid  = coord.id(pcfg) as usize; }
                            "ar" => { ret.ar_valid = coord.id(pcfg) as usize; }
                            _    => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "ready" => {
                        match channel.as_str() {
                            "b" => { ret.b_ready = coord.id(pcfg) as usize; }
                            "r" => { ret.r_ready = coord.id(pcfg) as usize; }
                            _   => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((name, idx)) => {
                                match (channel.as_str(), name) {
                                    ("aw", "addr") => { aw_addr_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "id")   => {   aw_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "len")  => {  aw_len_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "size") => { aw_size_idx.insert(idx, coord.id(pcfg)); }

                                    ("w",  "strb") => {  w_strb_idx.insert(idx, coord.id(pcfg)); }
                                    ("w",  "data") => {  w_data_idx.insert(idx, coord.id(pcfg)); }
                                    ("w",  "last") => {  ret.w_last = coord.id(pcfg) as usize; }

                                    ("ar", "addr") => { ar_addr_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "id")   => {   ar_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "len")  => {  ar_len_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "size") => { ar_size_idx.insert(idx, coord.id(pcfg)); }

                                    _ => {
                                        println!("Unrecognized AXI signal {} {}", channel, name);
                                    }
                                }
                            }
                            Err(e) => { assert!(false, "{}", e); }
                        }
                    }
                    _ => {
                        assert!(false, "Could not parse rdy_val_bits {}", rdy_val_bits);
                    }
                }
            }
        }
        aw_addr_idx.sort_keys();
        aw_id_idx.sort_keys();
        aw_len_idx.sort_keys();
        aw_size_idx.sort_keys();

        w_strb_idx.sort_keys();
        w_data_idx.sort_keys();

        ar_addr_idx.sort_keys();
        ar_id_idx.sort_keys();
        ar_len_idx.sort_keys();
        ar_size_idx.sort_keys();

        ret.aw_addr = aw_addr_idx.values().map(|&v| v as usize).collect();
        ret.aw_id   = aw_id_idx.values().map(|&v| v as usize).collect();
        ret.aw_len  = aw_len_idx.values().map(|&v| v as usize).collect();
        ret.aw_size = aw_size_idx.values().map(|&v| v as usize).collect();

        ret.w_strb = w_strb_idx.values().map(|&v| v as usize).collect();
        ret.w_data = w_data_idx.values().map(|&v| v as usize).collect();

        ret.ar_addr = ar_addr_idx.values().map(|&v| v as usize).collect();
        ret.ar_id   = ar_id_idx.values().map(|&v| v as usize).collect();
        ret.ar_len  = ar_len_idx.values().map(|&v| v as usize).collect();
        ret.ar_size = ar_size_idx.values().map(|&v| v as usize).collect();

        return ret;
    }
}

/// Maps signals going in to the target to a emulation platform `Coordinate`
/// - Example signals: `mem_axi4_0_aw_ready`, `mem_axi4_0_b_bits_id[0]`
#[derive(Debug, Default)]
pub struct AXI4TargetInIdx {
    pub aw_ready: usize,

    pub w_ready: usize,

    pub b_valid: usize,
    pub b_id:   Vec<usize>,
    pub b_resp: Vec<usize>,

    pub ar_ready: usize,

    pub r_valid: usize,
    pub r_id: Vec<usize>,
    pub r_resp: Vec<usize>,
    pub r_data: Vec<usize>,
    pub r_last: usize,
}

impl AXI4TargetInIdx {
    fn new(pfx: &str, input_signals: &IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = AXI4TargetInIdx::default();

        let mut b_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let mut b_resp_idx: IndexMap<u32, u32> = IndexMap::new();

        let mut r_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let mut r_resp_idx: IndexMap<u32, u32> = IndexMap::new();
        let mut r_data_idx: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in input_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let mut split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown AXI channel: {}, split: {:?}", name, split);
                }

                let channel = split.pop_front().unwrap().to_lowercase();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase();

                match rdy_val_bits.as_str() {
                    "valid" => {
                        match channel.as_str() {
                            "b" => { ret.b_valid = coord.id(pcfg) as usize; }
                            "r" => { ret.r_valid  = coord.id(pcfg) as usize; }
                            _    => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "ready" => {
                        match channel.as_str() {
                            "aw" => { ret.aw_ready = coord.id(pcfg) as usize; }
                            "w"  => { ret.w_ready  = coord.id(pcfg) as usize; }
                            "ar" => { ret.ar_ready = coord.id(pcfg) as usize; }
                            _   => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((name, idx)) => {
                                match (channel.as_str(), name) {
                                    ("b", "id")   => { b_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("b", "resp") => { b_resp_idx.insert(idx, coord.id(pcfg)); }

                                    ("r", "id")   => {   r_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "resp") => { r_resp_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "data") => { r_data_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "last") => { ret.r_last = coord.id(pcfg) as usize; }

                                    _ => {
                                        println!("Unrecognized AXI signal {} {}", channel, name);
                                    }
                                }
                            }
                            Err(e) => { assert!(false, "{}", e); }
                        }
                    }
                    _ => {
                        assert!(false, "Could not parse rdy_val_bits {}", rdy_val_bits);
                    }
                }
            }
        }

        b_id_idx.sort_keys();
        b_resp_idx.sort_keys();

        r_id_idx.sort_keys();
        r_resp_idx.sort_keys();
        r_data_idx.sort_keys();

        ret.b_id   = b_id_idx.values().map(|&v| v as usize).collect();
        ret.b_resp = b_resp_idx.values().map(|&v| v as usize).collect();

        ret.r_id   = r_id_idx.values().map(|&v| v as usize).collect();
        ret.r_resp = r_resp_idx.values().map(|&v| v as usize).collect();
        ret.r_data = r_data_idx.values().map(|&v| v as usize).collect();

        return ret;
    }
}

#[derive(Debug)]
pub struct AXI4ReadyBits {
    pub aw: bool,
    pub w: bool,
    pub b: bool,
    pub ar: bool,
    pub r: bool,
}

impl Default for AXI4ReadyBits {
    fn default() -> Self {
        Self {
            aw: true,
            w: false,
            b: false,
            ar: true,
            r: false
        }
    }
}

/// Maps signals comming out from the target to a emulation platform `Coordinate`
/// - Example signals: `serial_tl_0_out_valid`, `serial_tl_0_out_bits_phit[0]`
#[derive(Debug, Default)]
pub struct TSITargetOutIdx {
    pub out_valid: usize,
    pub out_bits: Vec<usize>,

    pub in_ready: usize
}

impl TSITargetOutIdx {
    fn new(pfx: &str, output_signals: &IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = TSITargetOutIdx::default();

        let mut bits: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in output_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let mut split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown TSI channel: {}, split: {:?}", name, split);
                }

                let _in_out = split.pop_front().unwrap().to_lowercase();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase();

                println!("_in_out: {} rdy_val_bits: {}", _in_out, rdy_val_bits);

                match (_in_out.as_str(), rdy_val_bits.as_str()) {
                    ("out", "valid") => {
                        ret.out_valid = coord.id(pcfg) as usize;
                    }
                    ("in", "ready") => {
                        ret.in_ready = coord.id(pcfg) as usize;
                    }
                    ("out", _) => {
// let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(rdy_val_bits.as_str()) {
                            Ok((_, idx)) => {
                                bits.insert(idx, coord.id(pcfg));
                            }
                            Err(e) => { assert!(false, "{}", e); }
                        }
                    }
                    _ => {
                        panic!("name: {}, sfx: {} _in_out: {} rdy_val_bits: {}", name, sfx, _in_out, rdy_val_bits);
                    }
                }
            }
        }
        bits.sort_keys();
        ret.out_bits = bits.values().map(|&v| v as usize).collect();

        return ret;
    }
}

/// Maps signals going in to the target to a emulation platform `Coordinate`
/// - Example signals: `serial_tl_0_in_valid`, `serial_tl_0_in_bits_phit[0]`
#[derive(Debug, Default)]
pub struct TSITargetInIdx {
    pub out_ready: usize,
    pub in_valid: usize,
    pub in_bits: Vec<usize>
}

impl TSITargetInIdx {
    fn new(pfx: &str, input_signals: &IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = TSITargetInIdx::default();

        let mut bits: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in input_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let mut split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown TSI channel: {}, split: {:?}", name, split);
                }

                let _in_out = split.pop_front().unwrap().to_lowercase();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase();
                println!("name: {}, sfx {} _in_out {} rdy_val_bits {}", name, sfx, _in_out, rdy_val_bits);

                match (_in_out.as_str(), rdy_val_bits.as_str()) {
                    ("in", "valid") => {
                        ret.in_valid = coord.id(pcfg) as usize;
                    }
                    ("out", "ready") => {
                        ret.out_ready = coord.id(pcfg) as usize;
                    }
                    ("in", _) => {
// let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(rdy_val_bits.as_str()) {
                            Ok((_, idx)) => {
                                bits.insert(idx, coord.id(pcfg));
                            }
                            Err(e) => { assert!(false, "{}", e); }
                        }
                    }
                    _ => {
                        panic!("name: {}, sfx: {} _in_out: {} rdy_val_bits: {}", name, sfx, _in_out, rdy_val_bits);
                    }
                }
            }
        }
        bits.sort_keys();
        ret.in_bits = bits.values().map(|&v| v as usize).collect();

        return ret;
    }
}

#[derive(Debug)]
pub struct TSIReadyBits {
    pub out: bool,
    pub in_: bool
}

impl Default for TSIReadyBits {
    fn default() -> Self {
        Self {
            out: true,
            in_: false
        }
    }
}

#[derive(Debug, Default)]
pub struct ResetTargetInIdx {
    pub uncore_reset: usize,
    pub hart_is_in_reset: usize
}

impl ResetTargetInIdx {
    fn new(input_signals: &IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = Self::default();
        for (name, coord)in input_signals.iter() {
            if name.ends_with("uncore_reset") {
                ret.uncore_reset = coord.id(pcfg) as usize;
            } else if name.contains("hartIsInReset") {
                ret.hart_is_in_reset = coord.id(pcfg) as usize;
            }
        }
        return ret;
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TargetSystem<'a> {
    #[derivative(Debug="ignore")]
    pub dram: DRAM,
    #[derivative(Debug="ignore")]
    pub axi: AXI4Channels,
    #[derivative(Debug="ignore")]
    pub tsi: TSI,
    #[derivative(Debug="ignore")]
    pub driver: Driver,
    pub cfg: &'a FPGATopConfig,
    pub axi_idx_o: AXI4TargetOutIdx,
    pub axi_idx_i: AXI4TargetInIdx,
    pub axi_rdy: AXI4ReadyBits,
    pub tsi_idx_o: TSITargetOutIdx,
    pub tsi_idx_i: TSITargetInIdx,
    pub tsi_rdy: TSIReadyBits,
    pub reset_idx: ResetTargetInIdx,
    pub io_stream_bytes: u32,
    #[derivative(Debug="ignore")]
    pub input_signals:  IndexMap<String, Coordinate>,
    #[derivative(Debug="ignore")]
    pub output_signals: IndexMap<String, Coordinate>,
    pub cycle: u64,
    pub reset_period: u64,
}

impl<'a> TargetSystem<'a> {
    const TSI_BITS: u32 = 32;
    const TSI_BYTES: u32 = Self::TSI_BITS / 8;
    const SAI_ADDR_CHUNKS: u32 = 2;
    const SAI_LEN_CHUNKS: u32 = 2;

    pub fn new(
        dram_base_addr: Addr,
        dram_size_bytes: Addr,
        dram_word_size: u32,
        driver: Driver,
        cfg: &'a FPGATopConfig,
        input_signals:  IndexMap<String, Coordinate>,
        output_signals: IndexMap<String, Coordinate>,
        dram_pfx_str: String,
        tsi_pfx_str: String,
    ) -> Self {

        let total_procs = cfg.emul.total_procs();
        let axi4_data_bits = cfg.axi.data_bits;
        let io_stream_bits = ((total_procs + axi4_data_bits - 1) / axi4_data_bits) * axi4_data_bits;
        let io_stream_bytes = io_stream_bits / 8;

        Self {
            dram: DRAM::new(dram_base_addr, dram_size_bytes, dram_word_size),
            axi: AXI4Channels::default(),
            tsi: TSI::default(),
            driver: driver,
            cfg: cfg,
            axi_idx_o: AXI4TargetOutIdx::new(&dram_pfx_str, &output_signals, &cfg.emul),
            axi_idx_i: AXI4TargetInIdx::new(&dram_pfx_str, &input_signals, &cfg.emul),
            axi_rdy: AXI4ReadyBits::default(),
            tsi_idx_o: TSITargetOutIdx::new(&tsi_pfx_str, &output_signals, &cfg.emul),
            tsi_idx_i: TSITargetInIdx::new(&tsi_pfx_str, &input_signals, &cfg.emul),
            tsi_rdy: TSIReadyBits::default(),
            reset_idx: ResetTargetInIdx::new(&input_signals, &cfg.emul),
            io_stream_bytes: io_stream_bytes,
            input_signals: input_signals,
            output_signals: output_signals,
            cycle: 0,
            reset_period: 25,
        }
    }

    fn construct_reset_input(self: &mut Self, ivec: &mut BitVec<usize, Lsb0>) {
        if self.cycle < self.reset_period {
            ivec.set(self.reset_idx.uncore_reset, true);
        }
        if self.cycle < self.reset_period {
            ivec.set(self.reset_idx.hart_is_in_reset, true);
        }
    }

    fn construct_axi_input(self: &mut Self, ivec: &mut BitVec<usize, Lsb0>) {
        ivec.set(self.axi_idx_i.aw_ready, self.axi_rdy.aw);

        ivec.set(self.axi_idx_i.w_ready,  self.axi_rdy.w);

        if !self.axi.b.is_empty() {
            let b = self.axi.b.pop_front().unwrap();

            ivec.set(self.axi_idx_i.b_valid, true);
            for (i, id_idx) in self.axi_idx_i.b_id.iter().enumerate() {
                ivec.set(*id_idx, (b.id >> i) & 1 == 1);
            }
            for (i, resp_idx) in self.axi_idx_i.b_resp.iter().enumerate() {
                ivec.set(*resp_idx, (b.resp >> i) & 1 == 1);
            }
        }

        ivec.set(self.axi_idx_i.ar_ready, self.axi_rdy.ar);


        // NOTE: In CY, the TLToAXI4 combinationally ties the axi4 r_ready & r_valid signals
        if !self.axi.r.is_empty() {
            let r = self.axi.r.pop_front().unwrap();

            ivec.set(self.axi_idx_i.r_valid, true);
            for (i, id_idx) in self.axi_idx_i.r_id.iter().enumerate() {
                ivec.set(*id_idx, (r.id >> i) & 1 == 1);
            }
            for (i, resp_idx) in self.axi_idx_i.r_resp.iter().enumerate() {
                ivec.set(*resp_idx, (r.resp >> i) & 1 == 1);
            }
            for (i, data_idx) in self.axi_idx_i.r_data.iter().enumerate() {
                let ii = i / 8;
                let jj = i % 8;
                ivec.set(*data_idx, (r.data[ii] >> jj) & 1 == 1);
            }
            ivec.set(self.axi_idx_i.r_last, r.last);
        }
    }

    fn construct_tsi_input(self: &mut Self, ivec: &mut BitVec<usize, Lsb0>) {
        ivec.set(self.tsi_idx_i.out_ready, self.tsi_rdy.out);
        if !self.tsi.i.is_empty() && self.tsi_rdy.in_ && self.cycle > self.reset_period + 30 {
            let tsi_req = self.tsi.i.pop_front().unwrap();

// println!("TSI input data: 0x{:x}", tsi_req);

            ivec.set(self.tsi_idx_i.in_valid, true);
            for (i, idx) in self.tsi_idx_i.in_bits.iter().enumerate() {
                ivec.set(*idx, (tsi_req >> i) & 1 == 1);
            }
        }
    }

    fn construct_ivec(self: &mut Self) -> Vec<u8> {
        let mut bit_vec: BitVec<usize, Lsb0> = BitVec::new();
        for _ in 0..self.cfg.emul.total_procs() {
            bit_vec.push(false);
        }

        self.construct_reset_input(&mut bit_vec);
        self.construct_axi_input(&mut bit_vec);
        self.construct_tsi_input(&mut bit_vec);

        let mut ivec: Vec<u8> = vec![];
        ivec.extend(bit_vec
            .into_vec()
            .iter()
            .flat_map(|x| x.to_le_bytes()));
            ivec.resize(self.io_stream_bytes as usize, 0);
        return ivec;
    }

    fn parse_axi_output(self: &mut Self, ovec: &BitVec<u8, Lsb0>) {
        self.axi_rdy.b = *ovec.get(self.axi_idx_o.b_ready).unwrap();
        self.axi_rdy.r = *ovec.get(self.axi_idx_o.r_ready).unwrap();

        if *ovec.get(self.axi_idx_o.aw_valid).unwrap() {
            println!("aw valid high, aw_ready: {}", self.axi_rdy.aw);
        }

        if self.axi_rdy.aw && *ovec.get(self.axi_idx_o.aw_valid).unwrap() {
            let mut addr = 0;
            for (i, idx) in self.axi_idx_o.aw_addr.iter().enumerate() {
                addr |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let mut size = 0;
            for (i, idx) in self.axi_idx_o.aw_size.iter().enumerate() {
                size |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let mut len = 0;
            for (i, idx) in self.axi_idx_o.aw_len.iter().enumerate() {
                len |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let aw = AXI4AW::from_addr_size_len(addr, size, len);
            println!("pull aw {:?}", aw);

            self.axi.aw.push_back(aw);
        }
        if *ovec.get(self.axi_idx_o.w_valid).unwrap() {
            println!("w valid high, w_ready: {}", self.axi_rdy.w);
        }
        if self.axi_rdy.w && *ovec.get(self.axi_idx_o.w_valid).unwrap() {
            let mut strb = 0;
            for (i, idx) in self.axi_idx_o.w_strb.iter().enumerate() {
                strb |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let mut data = 0;
            for (i, idx) in self.axi_idx_o.w_data.iter().enumerate() {
                data |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let last = ovec.get(self.axi_idx_o.w_last).unwrap() == true;

            let w = AXI4W::from_data_strb_last(&data.to_le_bytes().to_vec(), strb.into(), last);
            println!("pull w {:?}", w);

            self.axi.w.push_back(w);
        }


        if self.axi_rdy.ar && *ovec.get(self.axi_idx_o.ar_valid).unwrap() {
            let mut addr = 0;
            for (i, idx) in self.axi_idx_o.ar_addr.iter().enumerate() {
                addr |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let mut size = 0;
            for (i, idx) in self.axi_idx_o.ar_size.iter().enumerate() {
                size |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let mut len = 0;
            for (i, idx) in self.axi_idx_o.ar_len.iter().enumerate() {
                len |= (*ovec.get(*idx).unwrap() as u32) << i;
            }
            let ar = AXI4AR::from_addr_size_len(addr, size, len);
            println!("pull ar {:?}", ar);
            self.axi.ar.push_back(ar);
        }
    }

    fn parse_tsi_output(self: &mut Self, ovec: &BitVec<u8, Lsb0>) {
        self.tsi_rdy.in_ = *ovec.get(self.tsi_idx_o.in_ready).unwrap();
        if self.tsi_rdy.out && *ovec.get(self.tsi_idx_o.out_valid).unwrap() {
            let mut bits = 0;
            for (i, idx) in self.tsi_idx_o.out_bits.iter().enumerate() {
                bits |= (*ovec.get(*idx).unwrap() as u32) << i;
            }

            println!("TSI output data: 0x{:x}", bits);

            self.tsi.o.push_back(bits);
        }
    }

    fn parse_ovec(self: &mut Self, ovec: Vec<u8>) {
        let ovec_bit: BitVec<u8, Lsb0> = BitVec::from_vec(ovec);
        self.parse_axi_output(&ovec_bit);
        self.parse_tsi_output(&ovec_bit);
    }

    pub fn step(self: &mut Self) -> Result<(), SimIfErr> {
// println!("target step: {}", self.cycle);

        // push aw_ready
        // push  w_ready
        // push ar_ready
        // push b_valid if b_ready
        // push r_valid if r_ready
        let ivec = self.construct_ivec();
        self.driver.io_bridge.push(&mut self.driver.simif, &ivec)?;

        let mut ovec = vec![0u8; ivec.len()];
        'poll_io_out: loop {
            let read_bytes = self.driver.io_bridge.pull(&mut self.driver.simif, &mut ovec)?;
            if read_bytes == 0 {
                continue 'poll_io_out;
            } else {
                break 'poll_io_out;
            }
        }

        // pull aw_valid & if aw_ready -> push aw to channel
        // pull  w_valid & if  w_ready -> push  w to channel
        // pull ar_valid & if ar_ready -> push ar to channel
        // pull b_ready
        // pull r_ready
        self.parse_ovec(ovec);

// if !self.tsi.i.is_empty() {
// if self.tsi_rdy.in_ {
// let tsi_req = self.tsi.i.pop_front().unwrap();
// println!("push TSI input data: 0x{:x}", tsi_req);
// } else {
// let tsi_req = self.tsi.i.front().unwrap();
// println!("pending TSI input data: 0x{:x}", tsi_req);
// }
// }

// if !self.axi.r.is_empty() {
// if self.axi_rdy.r {
// let r = self.axi.r.pop_front().unwrap();
// println!("push AXI r: {:X?}", r);
// } else {
// println!("pending AXI r: {:X?}", self.axi.r.front().unwrap());
// }
// }

// if !self.axi.b.is_empty() {
// if self.axi_rdy.b {
// let b = self.axi.b.pop_front().unwrap();
// println!("push AXI b: {:X?}", b);
// } else {
// println!("pending AXI b: {:X?}", self.axi.b.front().unwrap());
// }
// }

        // if aw_valid && aw_ready -> do stuff in dram & update aw_ready
        // if  w_valid &&  w_ready -> do stuff in dram & update  w_ready
        // if ar_valid && ar_ready -> do stuff in dram & update ar_ready
        // push b_resp to channel
        // push r_resp to channel
        self.dram.step(&mut self.axi, &mut self.axi_rdy);

        self.cycle += 1;

        return Ok(());
    }

}

enum SAICommands {
    SaiCmdRead = 0,
    SaiCmdWrite,
}

impl<'a> TargetSystem<'a> {
    fn push_addr(&mut self, ptr: u64) {
        let mut addr = ptr;
        for _i in 0..TargetSystem::SAI_ADDR_CHUNKS {
            self.tsi.i.push_back((addr & 0xffffffff) as u32);
            addr >>= TargetSystem::TSI_BITS;
        }
    }

    fn push_len(&mut self, length: u64) {
        let mut len = length;
        for _i in 0..TargetSystem::SAI_LEN_CHUNKS {
            self.tsi.i.push_back((len & 0xffffffff) as u32);
            len >>= 32;
        }
    }

    fn to_u32(slice: &[u8]) -> u32 {
        let mut buffer = [0u8; 4];
        let len = slice.len().min(4);
        buffer[..len].copy_from_slice(&slice[..len]);
        u32::from_le_bytes(buffer)
    }
}

impl<'a> Htif for TargetSystem<'a> {
    fn read(&mut self, ptr: u64, buf: &mut [u8]) -> Result<(), fesvr::Error> {
        let chunks = buf.chunks(TargetSystem::TSI_BYTES as usize).len();

        self.tsi.i.push_back(SAICommands::SaiCmdRead as u32);
        self.push_addr(ptr);
        self.push_len((chunks - 1) as u64);

        for chunk in buf.chunks_mut(TargetSystem::TSI_BYTES as usize) {
            while self.tsi.o.is_empty() {
                let _ = self.step();
            }
            let buf_u32 = self.tsi.o.pop_front().unwrap();
            let buf_u8 = buf_u32.to_le_bytes();
            for (i, b) in chunk.iter_mut().enumerate() {
                *b = buf_u8[i];
            }
        }

        println!("Htif read done buf: {:?}", buf);

        return Ok(());
    }

    fn write(&mut self, ptr: u64, buf: &[u8]) -> Result<(), fesvr::Error> {
        let chunks = buf.chunks(TargetSystem::TSI_BYTES as usize);

        println!("Htif write to addr: 0x{:x} len: {} chunks.len: {}", ptr, buf.len(), chunks.len());

        self.tsi.i.push_back(SAICommands::SaiCmdWrite as u32);
        self.push_addr(ptr);
        self.push_len((chunks.len() - 1) as u64);

        for chunk in chunks {
            self.tsi.i.push_back(Self::to_u32(chunk));
        }

        println!("Htif write done buf: {:?}", buf);

        return Ok(());
    }
}
