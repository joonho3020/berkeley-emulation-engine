use crate::driver::axi::*;
use crate::driver::dram::*;
use crate::driver::tsi::*;
use crate::simif::simif::Driver;
use crate::simif::mmioif::*;
use crate::simif::dmaif::*;
use std::collections::VecDeque;
use bee::common::config::PlatformConfig;
use indexmap::IndexMap;
use bee::common::{
        network::Coordinate,
        config::PlatformConfig
};
use bitvec::{order::Lsb0, vec::BitVec};
use super::driver::FPGATopConfig;


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

// .inputs auto_chipyard_prcictrl_domain_reset_setter_clock_in_member_allClocks_uncore_clock auto_chipyard_prcictrl_domain_reset_setter_clock_in_member_allClocks_uncore_reset resetctrl_hartIsInReset_0 debug_clock debug_reset debug_systemjtag_jtag_TCK debug_systemjtag_jtag_TMS debug_systemjtag_jtag_TDI debug_systemjtag_reset debug_dmactiveAck mem_axi4_0_aw_ready mem_axi4_0_w_ready mem_axi4_0_b_valid mem_axi4_0_b_bits_id[0] mem_axi4_0_b_bits_id[1] mem_axi4_0_b_bits_id[2] mem_axi4_0_b_bits_id[3] mem_axi4_0_b_bits_resp[0] mem_axi4_0_b_bits_resp[1] mem_axi4_0_ar_ready mem_axi4_0_r_valid mem_axi4_0_r_bits_id[0] mem_axi4_0_r_bits_id[1] mem_axi4_0_r_bits_id[2] mem_axi4_0_r_bits_id[3] mem_axi4_0_r_bits_data[0] mem_axi4_0_r_bits_data[1] mem_axi4_0_r_bits_data[2] mem_axi4_0_r_bits_data[3] mem_axi4_0_r_bits_data[4] mem_axi4_0_r_bits_data[5] mem_axi4_0_r_bits_data[6] mem_axi4_0_r_bits_data[7] mem_axi4_0_r_bits_data[8] mem_axi4_0_r_bits_data[9] mem_axi4_0_r_bits_data[10] mem_axi4_0_r_bits_data[11] mem_axi4_0_r_bits_data[12] mem_axi4_0_r_bits_data[13] mem_axi4_0_r_bits_data[14] mem_axi4_0_r_bits_data[15] mem_axi4_0_r_bits_data[16] mem_axi4_0_r_bits_data[17] mem_axi4_0_r_bits_data[18] mem_axi4_0_r_bits_data[19] mem_axi4_0_r_bits_data[20] mem_axi4_0_r_bits_data[21] mem_axi4_0_r_bits_data[22] mem_axi4_0_r_bits_data[23] mem_axi4_0_r_bits_data[24] mem_axi4_0_r_bits_data[25] mem_axi4_0_r_bits_data[26] mem_axi4_0_r_bits_data[27] mem_axi4_0_r_bits_data[28] mem_axi4_0_r_bits_data[29] mem_axi4_0_r_bits_data[30] mem_axi4_0_r_bits_data[31] mem_axi4_0_r_bits_data[32] mem_axi4_0_r_bits_data[33] mem_axi4_0_r_bits_data[34] mem_axi4_0_r_bits_data[35] mem_axi4_0_r_bits_data[36] mem_axi4_0_r_bits_data[37] mem_axi4_0_r_bits_data[38] mem_axi4_0_r_bits_data[39] mem_axi4_0_r_bits_data[40] mem_axi4_0_r_bits_data[41] mem_axi4_0_r_bits_data[42] mem_axi4_0_r_bits_data[43] mem_axi4_0_r_bits_data[44] mem_axi4_0_r_bits_data[45] mem_axi4_0_r_bits_data[46] mem_axi4_0_r_bits_data[47] mem_axi4_0_r_bits_data[48] mem_axi4_0_r_bits_data[49] mem_axi4_0_r_bits_data[50] mem_axi4_0_r_bits_data[51] mem_axi4_0_r_bits_data[52] mem_axi4_0_r_bits_data[53] mem_axi4_0_r_bits_data[54] mem_axi4_0_r_bits_data[55] mem_axi4_0_r_bits_data[56] mem_axi4_0_r_bits_data[57] mem_axi4_0_r_bits_data[58] mem_axi4_0_r_bits_data[59] mem_axi4_0_r_bits_data[60] mem_axi4_0_r_bits_data[61] mem_axi4_0_r_bits_data[62] mem_axi4_0_r_bits_data[63] mem_axi4_0_r_bits_resp[0] mem_axi4_0_r_bits_resp[1] mem_axi4_0_r_bits_last custom_boot serial_tl_0_in_valid serial_tl_0_in_bits_phit[0] serial_tl_0_in_bits_phit[1] serial_tl_0_in_bits_phit[2] serial_tl_0_in_bits_phit[3] serial_tl_0_in_bits_phit[4] serial_tl_0_in_bits_phit[5] serial_tl_0_in_bits_phit[6] serial_tl_0_in_bits_phit[7] serial_tl_0_in_bits_phit[8] serial_tl_0_in_bits_phit[9] serial_tl_0_in_bits_phit[10] serial_tl_0_in_bits_phit[11] serial_tl_0_in_bits_phit[12] serial_tl_0_in_bits_phit[13] serial_tl_0_in_bits_phit[14] serial_tl_0_in_bits_phit[15] serial_tl_0_in_bits_phit[16] serial_tl_0_in_bits_phit[17] serial_tl_0_in_bits_phit[18] serial_tl_0_in_bits_phit[19] serial_tl_0_in_bits_phit[20] serial_tl_0_in_bits_phit[21] serial_tl_0_in_bits_phit[22] serial_tl_0_in_bits_phit[23] serial_tl_0_in_bits_phit[24] serial_tl_0_in_bits_phit[25] serial_tl_0_in_bits_phit[26] serial_tl_0_in_bits_phit[27] serial_tl_0_in_bits_phit[28] serial_tl_0_in_bits_phit[29] serial_tl_0_in_bits_phit[30] serial_tl_0_in_bits_phit[31] serial_tl_0_out_ready serial_tl_0_clock_in uart_0_rxd
// .outputs auto_mbus_fixedClockNode_anon_out_clock auto_cbus_fixedClockNode_anon_out_clock auto_cbus_fixedClockNode_anon_out_reset debug_systemjtag_jtag_TDO_data debug_dmactive mem_axi4_0_aw_valid mem_axi4_0_aw_bits_id[0] mem_axi4_0_aw_bits_id[1] mem_axi4_0_aw_bits_id[2] mem_axi4_0_aw_bits_id[3] mem_axi4_0_aw_bits_addr[0] mem_axi4_0_aw_bits_addr[1] mem_axi4_0_aw_bits_addr[2] mem_axi4_0_aw_bits_addr[3] mem_axi4_0_aw_bits_addr[4] mem_axi4_0_aw_bits_addr[5] mem_axi4_0_aw_bits_addr[6] mem_axi4_0_aw_bits_addr[7] mem_axi4_0_aw_bits_addr[8] mem_axi4_0_aw_bits_addr[9] mem_axi4_0_aw_bits_addr[10] mem_axi4_0_aw_bits_addr[11] mem_axi4_0_aw_bits_addr[12] mem_axi4_0_aw_bits_addr[13] mem_axi4_0_aw_bits_addr[14] mem_axi4_0_aw_bits_addr[15] mem_axi4_0_aw_bits_addr[16] mem_axi4_0_aw_bits_addr[17] mem_axi4_0_aw_bits_addr[18] mem_axi4_0_aw_bits_addr[19] mem_axi4_0_aw_bits_addr[20] mem_axi4_0_aw_bits_addr[21] mem_axi4_0_aw_bits_addr[22] mem_axi4_0_aw_bits_addr[23] mem_axi4_0_aw_bits_addr[24] mem_axi4_0_aw_bits_addr[25] mem_axi4_0_aw_bits_addr[26] mem_axi4_0_aw_bits_addr[27] mem_axi4_0_aw_bits_addr[28] mem_axi4_0_aw_bits_addr[29] mem_axi4_0_aw_bits_addr[30] mem_axi4_0_aw_bits_addr[31] mem_axi4_0_aw_bits_len[0] mem_axi4_0_aw_bits_len[1] mem_axi4_0_aw_bits_len[2] mem_axi4_0_aw_bits_len[3] mem_axi4_0_aw_bits_len[4] mem_axi4_0_aw_bits_len[5] mem_axi4_0_aw_bits_len[6] mem_axi4_0_aw_bits_len[7] mem_axi4_0_aw_bits_size[0] mem_axi4_0_aw_bits_size[1] mem_axi4_0_aw_bits_size[2] mem_axi4_0_aw_bits_burst[0] mem_axi4_0_aw_bits_burst[1] mem_axi4_0_aw_bits_lock mem_axi4_0_aw_bits_cache[0] mem_axi4_0_aw_bits_cache[1] mem_axi4_0_aw_bits_cache[2] mem_axi4_0_aw_bits_cache[3] mem_axi4_0_aw_bits_prot[0] mem_axi4_0_aw_bits_prot[1] mem_axi4_0_aw_bits_prot[2] mem_axi4_0_aw_bits_qos[0] mem_axi4_0_aw_bits_qos[1] mem_axi4_0_aw_bits_qos[2] mem_axi4_0_aw_bits_qos[3] mem_axi4_0_w_valid mem_axi4_0_w_bits_data[0] mem_axi4_0_w_bits_data[1] mem_axi4_0_w_bits_data[2] mem_axi4_0_w_bits_data[3] mem_axi4_0_w_bits_data[4] mem_axi4_0_w_bits_data[5] mem_axi4_0_w_bits_data[6] mem_axi4_0_w_bits_data[7] mem_axi4_0_w_bits_data[8] mem_axi4_0_w_bits_data[9] mem_axi4_0_w_bits_data[10] mem_axi4_0_w_bits_data[11] mem_axi4_0_w_bits_data[12] mem_axi4_0_w_bits_data[13] mem_axi4_0_w_bits_data[14] mem_axi4_0_w_bits_data[15] mem_axi4_0_w_bits_data[16] mem_axi4_0_w_bits_data[17] mem_axi4_0_w_bits_data[18] mem_axi4_0_w_bits_data[19] mem_axi4_0_w_bits_data[20] mem_axi4_0_w_bits_data[21] mem_axi4_0_w_bits_data[22] mem_axi4_0_w_bits_data[23] mem_axi4_0_w_bits_data[24] mem_axi4_0_w_bits_data[25] mem_axi4_0_w_bits_data[26] mem_axi4_0_w_bits_data[27] mem_axi4_0_w_bits_data[28] mem_axi4_0_w_bits_data[29] mem_axi4_0_w_bits_data[30] mem_axi4_0_w_bits_data[31] mem_axi4_0_w_bits_data[32] mem_axi4_0_w_bits_data[33] mem_axi4_0_w_bits_data[34] mem_axi4_0_w_bits_data[35] mem_axi4_0_w_bits_data[36] mem_axi4_0_w_bits_data[37] mem_axi4_0_w_bits_data[38] mem_axi4_0_w_bits_data[39] mem_axi4_0_w_bits_data[40] mem_axi4_0_w_bits_data[41] mem_axi4_0_w_bits_data[42] mem_axi4_0_w_bits_data[43] mem_axi4_0_w_bits_data[44] mem_axi4_0_w_bits_data[45] mem_axi4_0_w_bits_data[46] mem_axi4_0_w_bits_data[47] mem_axi4_0_w_bits_data[48] mem_axi4_0_w_bits_data[49] mem_axi4_0_w_bits_data[50] mem_axi4_0_w_bits_data[51] mem_axi4_0_w_bits_data[52] mem_axi4_0_w_bits_data[53] mem_axi4_0_w_bits_data[54] mem_axi4_0_w_bits_data[55] mem_axi4_0_w_bits_data[56] mem_axi4_0_w_bits_data[57] mem_axi4_0_w_bits_data[58] mem_axi4_0_w_bits_data[59] mem_axi4_0_w_bits_data[60] mem_axi4_0_w_bits_data[61] mem_axi4_0_w_bits_data[62] mem_axi4_0_w_bits_data[63] mem_axi4_0_w_bits_strb[0] mem_axi4_0_w_bits_strb[1] mem_axi4_0_w_bits_strb[2] mem_axi4_0_w_bits_strb[3] mem_axi4_0_w_bits_strb[4] mem_axi4_0_w_bits_strb[5] mem_axi4_0_w_bits_strb[6] mem_axi4_0_w_bits_strb[7] mem_axi4_0_w_bits_last mem_axi4_0_b_ready mem_axi4_0_ar_valid mem_axi4_0_ar_bits_id[0] mem_axi4_0_ar_bits_id[1] mem_axi4_0_ar_bits_id[2] mem_axi4_0_ar_bits_id[3] mem_axi4_0_ar_bits_addr[0] mem_axi4_0_ar_bits_addr[1] mem_axi4_0_ar_bits_addr[2] mem_axi4_0_ar_bits_addr[3] mem_axi4_0_ar_bits_addr[4] mem_axi4_0_ar_bits_addr[5] mem_axi4_0_ar_bits_addr[6] mem_axi4_0_ar_bits_addr[7] mem_axi4_0_ar_bits_addr[8] mem_axi4_0_ar_bits_addr[9] mem_axi4_0_ar_bits_addr[10] mem_axi4_0_ar_bits_addr[11] mem_axi4_0_ar_bits_addr[12] mem_axi4_0_ar_bits_addr[13] mem_axi4_0_ar_bits_addr[14] mem_axi4_0_ar_bits_addr[15] mem_axi4_0_ar_bits_addr[16] mem_axi4_0_ar_bits_addr[17] mem_axi4_0_ar_bits_addr[18] mem_axi4_0_ar_bits_addr[19] mem_axi4_0_ar_bits_addr[20] mem_axi4_0_ar_bits_addr[21] mem_axi4_0_ar_bits_addr[22] mem_axi4_0_ar_bits_addr[23] mem_axi4_0_ar_bits_addr[24] mem_axi4_0_ar_bits_addr[25] mem_axi4_0_ar_bits_addr[26] mem_axi4_0_ar_bits_addr[27] mem_axi4_0_ar_bits_addr[28] mem_axi4_0_ar_bits_addr[29] mem_axi4_0_ar_bits_addr[30] mem_axi4_0_ar_bits_addr[31] mem_axi4_0_ar_bits_len[0] mem_axi4_0_ar_bits_len[1] mem_axi4_0_ar_bits_len[2] mem_axi4_0_ar_bits_len[3] mem_axi4_0_ar_bits_len[4] mem_axi4_0_ar_bits_len[5] mem_axi4_0_ar_bits_len[6] mem_axi4_0_ar_bits_len[7] mem_axi4_0_ar_bits_size[0] mem_axi4_0_ar_bits_size[1] mem_axi4_0_ar_bits_size[2] mem_axi4_0_ar_bits_burst[0] mem_axi4_0_ar_bits_burst[1] mem_axi4_0_ar_bits_lock mem_axi4_0_ar_bits_cache[0] mem_axi4_0_ar_bits_cache[1] mem_axi4_0_ar_bits_cache[2] mem_axi4_0_ar_bits_cache[3] mem_axi4_0_ar_bits_prot[0] mem_axi4_0_ar_bits_prot[1] mem_axi4_0_ar_bits_prot[2] mem_axi4_0_ar_bits_qos[0] mem_axi4_0_ar_bits_qos[1] mem_axi4_0_ar_bits_qos[2] mem_axi4_0_ar_bits_qos[3] mem_axi4_0_r_ready serial_tl_0_in_ready serial_tl_0_out_valid serial_tl_0_out_bits_phit[0] serial_tl_0_out_bits_phit[1] serial_tl_0_out_bits_phit[2] serial_tl_0_out_bits_phit[3] serial_tl_0_out_bits_phit[4] serial_tl_0_out_bits_phit[5] serial_tl_0_out_bits_phit[6] serial_tl_0_out_bits_phit[7] serial_tl_0_out_bits_phit[8] serial_tl_0_out_bits_phit[9] serial_tl_0_out_bits_phit[10] serial_tl_0_out_bits_phit[11] serial_tl_0_out_bits_phit[12] serial_tl_0_out_bits_phit[13] serial_tl_0_out_bits_phit[14] serial_tl_0_out_bits_phit[15] serial_tl_0_out_bits_phit[16] serial_tl_0_out_bits_phit[17] serial_tl_0_out_bits_phit[18] serial_tl_0_out_bits_phit[19] serial_tl_0_out_bits_phit[20] serial_tl_0_out_bits_phit[21] serial_tl_0_out_bits_phit[22] serial_tl_0_out_bits_phit[23] serial_tl_0_out_bits_phit[24] serial_tl_0_out_bits_phit[25] serial_tl_0_out_bits_phit[26] serial_tl_0_out_bits_phit[27] serial_tl_0_out_bits_phit[28] serial_tl_0_out_bits_phit[29] serial_tl_0_out_bits_phit[30] serial_tl_0_out_bits_phit[31] uart_0_txd clock_tap

impl AXI4TargetOutIdx {
// mem_axi4_0_aw_valid
// mem_axi4_0_aw_bits_id[0]
    fn new(pfx: String, output_signals: IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = AXI4TargetOutIdx::default();

        let aw_addr_idx: IndexMap<u32, u32> = IndexMap::new();
        let aw_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let aw_len_idx:  IndexMap<u32, u32> = IndexMap::new();
        let aw_size_idx: IndexMap<u32, u32> = IndexMap::new();

        let w_strb_idx: IndexMap<u32, u32> = IndexMap::new();
        let w_data_idx: IndexMap<u32, u32> = IndexMap::new();

        let ar_addr_idx: IndexMap<u32, u32> = IndexMap::new();
        let ar_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let ar_len_idx:  IndexMap<u32, u32> = IndexMap::new();
        let ar_size_idx: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in output_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown AXI channel: {}, split: {:?}", name, split);
                }

                let channel = split.pop_front().unwrap().to_lowercase().as_str();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase().as_str();

                match rdy_val_bits {
                    "valid" => {
                        match channel {
                            "aw" => { ret.aw_valid = coord.id(pcfg); }
                            "w"  => { ret.w_valid  = coord.id(pcfg); }
                            "ar" => { ret.ar_valid = coord.id(pcfg); }
                            _    => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "ready" => {
                        match channel {
                            "b" => { ret.b_ready = coord.id(pcfg); }
                            "r" => { ret.r_ready = coord.id(pcfg); }
                            _   => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((name, idx)) => {
                                match (channel, name) {
                                    ("aw", "addr") => { aw_addr_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "id")   => {   aw_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "len")  => {  aw_len_idx.insert(idx, coord.id(pcfg)); }
                                    ("aw", "size") => { aw_size_idx.insert(idx, coord.id(pcfg)); }

                                    ("w",  "strb") => {  w_strb_idx.insert(idx, coord.id(pcfg)); }
                                    ("w",  "data") => {  w_data_idx.insert(idx, coord.id(pcfg)); }
                                    ("w",  "last") => {  ret.w_last = coord.id(pcfg); }

                                    ("ar", "addr") => { ar_addr_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "id")   => {   ar_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "len")  => {  ar_len_idx.insert(idx, coord.id(pcfg)); }
                                    ("ar", "size") => { ar_size_idx.insert(idx, coord.id(pcfg)); }

                                    _ => {
                                        println!("Unrecognized AXI signal {} {}", channel, name);
                                    }
                                }
                            }
                            Err(e) => { assert!(false, e); }
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

        ret.aw_addr = aw_addr_idx.values();
        ret.aw_id   = aw_id_idx.values();
        ret.aw_len  = aw_len_idx.values();
        ret.aw_size = aw_size_idx.values();

        ret.w_strb = w_strb_idx.values();
        ret.w_data = w_data_idx.values();

        ret.ar_addr = ar_addr_idx.values();
        ret.ar_id   = ar_id_idx.values();
        ret.ar_len  = ar_len_idx.values();
        ret.ar_size = ar_size_idx.values();

        return ret;
    }
}

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
    fn new(pfx: String, input_signals: IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = AXI4TargetInIdx::default();

        let b_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let b_resp_idx: IndexMap<u32, u32> = IndexMap::new();

        let r_id_idx:   IndexMap<u32, u32> = IndexMap::new();
        let r_resp_idx: IndexMap<u32, u32> = IndexMap::new();
        let r_data_idx: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in input_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown AXI channel: {}, split: {:?}", name, split);
                }

                let channel = split.pop_front().unwrap().to_lowercase().as_str();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase().as_str();

                match rdy_val_bits {
                    "valid" => {
                        match channel {
                            "b" => { ret.b_valid = coord.id(pcfg); }
                            "r" => { ret.r_valid  = coord.id(pcfg); }
                            _    => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "ready" => {
                        match channel {
                            "aw" => { ret.aw_ready = coord.id(pcfg); }
                            "w"  => { ret.w_ready  = coord.id(pcfg); }
                            "ar" => { ret.ar_ready = coord.id(pcfg); }
                            _   => { assert!(false, "Invalid signal {}", name); }
                        }
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((name, idx)) => {
                                match (channel, name) {
                                    ("b", "id")   => { b_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("b", "resp") => { b_resp_idx.insert(idx, coord.id(pcfg)); }

                                    ("r", "id")   => {   r_id_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "resp") => { r_resp_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "data") => { r_data_idx.insert(idx, coord.id(pcfg)); }
                                    ("r", "last") => { ret.r_last = coord.id(pcfg); }

                                    _ => {
                                        println!("Unrecognized AXI signal {} {}", channel, name);
                                    }
                                }
                            }
                            Err(e) => { assert!(false, e); }
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

        ret.b_id   = b_id_idx.values();
        ret.b_resp = b_resp_idx.values();

        ret.r_id   = r_id_idx.values();
        ret.r_resp = r_resp_idx.values();
        ret.r_data = r_data_idx.values();

        return ret;
    }
}

#[derive(Debug, Default)]
pub struct AXI4ReadyBits {
    pub aw: bool,
    pub w: bool,
    pub b: bool,
    pub ar: bool,
    pub r: bool,
}

#[derive(Debug, Default)]
pub struct TSITargetOutIdx {
    pub out_valid: usize,
    pub out_bits: Vec<usize>,

    pub in_ready: usize
}

impl TSITargetOutIdx {
// serial_tl_0_in_valid serial_tl_0_in_bits_phit[0]
    fn new(pfx: String, output_signals: IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = TSITargetOutIdx::default();

        let bits: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in output_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown TSI channel: {}, split: {:?}", name, split);
                }

                let in_out = split.pop_front().unwrap().to_lowercase().as_str();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase().as_str();

                match rdy_val_bits {
                    "valid" => {
                        ret.out_valid = coord.id(pcfg);
                    }
                    "ready" => {
                        ret.in_ready = coord.id(pcfg);
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((_, idx)) => {
                                bits.insert(idx, coord.id(pcfg));
                            }
                            Err(e) => { assert!(false, e); }
                        }
                    }
                    _ => {
                        assert!(false, "Could not parse rdy_val_bits {}", rdy_val_bits);
                    }
                }
            }
        }
        bits.sort_keys();
        ret.out_bits = bits;

        return ret;
    }
}

#[derive(Debug, Default)]
pub struct TSITargetInIdx {
    pub out_ready: usize,
    pub in_valid: usize,
    pub in_bits: Vec<usize>
}

impl TSITargetInIdx {
// serial_tl_0_in_valid serial_tl_0_in_bits_phit[0]
    fn new(pfx: String, input_signals: IndexMap<String, Coordinate>, pcfg: &PlatformConfig) -> Self {
        let mut ret = TSITargetInIdx::default();

        let bits: IndexMap<u32, u32> = IndexMap::new();

        for (name, coord) in input_signals.iter() {
            if let Some(sfx) = name.strip_prefix(pfx) {
                let split: VecDeque<&str> = sfx.split('_').filter(|&s| !s.is_empty()).collect();
                if split.len() < 2 {
                    assert!(false, "Unknown TSI channel: {}, split: {:?}", name, split);
                }

                let in_out = split.pop_front().unwrap().to_lowercase().as_str();
                let rdy_val_bits = split.pop_front().unwrap().to_lowercase().as_str();

                match rdy_val_bits {
                    "valid" => {
                        ret.in_valid = coord.id(pcfg);
                    }
                    "ready" => {
                        ret.out_ready = coord.id(pcfg);
                    }
                    "bits" => {
                        let field_with_bit_index = split.pop_front().unwrap().to_lowercase();
                        match split_indexed_field(field_with_bit_index.as_str()) {
                            Ok((_, idx)) => {
                                bits.insert(idx, coord.id(pcfg));
                            }
                            Err(e) => { assert!(false, e); }
                        }
                    }
                    _ => {
                        assert!(false, "Could not parse rdy_val_bits {}", rdy_val_bits);
                    }
                }
            }
        }
        bits.sort_keys();
        ret.in_bits = bits;

        return ret;
    }
}

#[derive(Debug, Default)]
pub struct TSIReadyBits {
    pub out: bool,
    pub in_: bool
}

#[derive(Debug, Default)]
pub struct TestHarness {
    pub dram: DRAM,
    pub axi: AXI4Channels,
    pub tsi: TSI,
    pub driver: Driver,
    pub cfg: &FPGATopConfig,
    pub axi_idx_o: AXI4TargetOutIdx,
    pub axi_idx_i: AXI4TargetInIdx,
    pub axi_rdy: AXI4ReadyBits,
    pub tsi_idx_o: TSITargetOutIdx,
    pub tsi_idx_i: TSITargetInIdx,
    pub tsi_rdy: TSIReadyBits,
    pub io_stream_bytes: u32,
    pub tot_procs: u32,
    pub input_signals:  IndexMap<String, Coordinate>,
    pub output_signals: IndexMap<String, Coordinate>,
}

impl TestHarness {
    fn new(
        dram_base_addr: Addr,
        dram_size_bytes: Addr,
        dram_word_size: u32,
        driver: Driver,
        cfg: &FPGATopConfig,
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
            driver: Driver,
            cfg: cfg,
            // FIXME:... proper index setting
            axi_idx_o: AXI4TargetOutIdx::new("mem_axi4_0".to_string(), output_signals, &cfg.emul),
            axi_idx_i: AXI4TargetInIdx::new("mem_axi4_0".to_string(), input_signals, &cfg.emul),
            axi_rdy: AXI4ReadyBits::default(),
            tsi_idx_o: TSITargetOutIdx::new("serial_tl_0".to_string(), output_signals, &cfg.emul),
            tsi_idx_i: TSITargetInIdx::new("serial_tl_0".to_string(), input_signals, &cfg.emul),
            tsi_rdy: TSIReadyBits::default(),
            io_stream_bytes: io_stream_bytes,
            tot_procs: cfg.emul.total_procs(),
            input_signals: input_signals,
            output_signals: output_signals
        }
    }

    fn construct_axi_input(self: &mut Self, ivec: &mut BitVec<usize, Lsb0>) {
        ivec.set(self.axi_idx_i.aw_ready, self.axi_rdy.aw);

        ivec.set(self.axi_idx_i.w_ready,  self.axi_rdy.w);

        if !self.axi.b.is_empty() && self.axi_rdy.b {
            let b = self.axi.b.pop_front().unwrap();
            ivec.set(self.axi_idx_i.b_valid, 1);
            for (i, id_idx) in self.axi_idx_i.b_id.iter().enumerate() {
                ivec.set(id_idx, b.id >> i);
            }
            for (i, resp_idx) in self.axi_idx_i.b_resp.iter().enumerate() {
                ivec.set(resp_idx, b.resp >> i);
            }
        }

        ivec.set(self.axi_idx_i.ar_ready, self.axi_rdy.ar);

        if !self.axi.r.is_empty() && self.axi_rdy.r {
            let r = self.axi.r.pop_front().unwrap();
            ivec.set(self.axi_idx_i.r_valid, 1);
            for (i, id_idx) in self.axi_idx_i.r_id.iter().enumerate() {
                ivec.set(id_idx, r.id >> i);
            }
            for (i, resp_idx) in self.axi_idx_i.r_resp.iter().enumerate() {
                ivec.set(resp_idx, r.resp >> i);
            }
            for (i, data_idx) in self.axi_idx_i.r_data.iter().enumerate() {
                ivec.set(data_idx, r.data >> i);
            }
            ivec.set(self.axi_idx_i.r_last, r.last);
        }
    }

    fn construct_tsi_input(self: &mut Self, ivec: &mut BitVec<usize, Lsb0>) {
        ivec.set(self.tsi_idx_i.out_ready, self.tsi_rdy.out);
        if !self.tsi.i.is_empty() && self.tsi_rdy.in_ {
            let tsi_req = self.tsi.i.pop_front().unwrap();
            ivec.set(self.tsi_idx_i.in_valid, 1);
            for (i, idx) in self.tsi_idx_i.in_bits.iter().enumerate() {
                ivec.set(idx, tsi_req >> idx);
            }
        }
    }

    fn construct_ivec(self: &mut Self) -> Vec<u8> {
        let mut bit_vec: BitVec<usize, Lsb0> = BitVec::new();
        for _ in 0..self.tot_procs {
            bit_vec.push(false);
        }

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

    fn parse_axi_output(self: &mut Self, ovec: &BitVec<usize, Lsb0>) {
        self.axi_rdy.b = ovec.get(self.axi_idx_o.b_ready).unwrap();
        self.axi_rdy.r = ovec.get(self.axi_idx_o.r_ready).unwrap();

        if self.axi_rdy.aw && ovec.get(self.axi_idx_o.aw_valid).unwrap() {
            let mut addr = 0;
            for (i, idx) in self.axi_idx_o.aw_addr.iter().enumerate() {
                addr |= ovec.get(idx).unwrap() << i;
            }
            let mut size = 0;
            for (i, idx) in self.axi_idx_o.aw_size.iter().enumerate() {
                size |= ovec.get(idx).unwrap() << i;
            }
            let mut len = 0;
            for (i, idx) in self.axi_idx_o.aw_len.iter().enumerate() {
                len |= ovec.get(idx).unwrap() << i;
            }
            self.axi.aw.push_back(AXI4AW::from_addr_size_len(addr, size, len));
        }

        if self.axi_rdy.w && ovec.get(self.axi_idx_o.w_valid).unwrap() {
            let mut strb = 0;
            for (i, idx) in self.axi_idx_o.w_strb.iter().enumerate() {
                strb |= ovec.get(idx).unwrap() << i;
            }
            let mut data = 0;
            for (i, idx) in self.axi_idx_o.w_data.iter().enumerate() {
                data |= ovec.get(idx).unwrap() << i;
            }
            let last = ovec.get(self.axi_idx_o.w_last).unwrap();
            self.axi.w.push_back(AXI4W::from_data_strb_last(data, strb, last));
        }


        if self.axi_rdy.ar && ovec.get(self.axi_idx_o.ar_valid).unwrap() {
            let mut addr = 0;
            for (i, idx) in self.axi_idx_o.ar_addr.iter().enumerate() {
                addr |= ovec.get(idx).unwrap() << i;
            }
            let mut size = 0;
            for (i, idx) in self.axi_idx_o.ar_size.iter().enumerate() {
                size |= ovec.get(idx).unwrap() << i;
            }
            let mut len = 0;
            for (i, idx) in self.axi_idx_o.ar_len.iter().enumerate() {
                len |= ovec.get(idx).unwrap() << i;
            }
            self.axi.ar.push_back(AXI4AR::from_addr_size_len(addr, size, len));
        }
    }

    fn parse_tsi_output(self: &mut Self, ovec: &BitVec<usize, Lsb0>) {
        self.tsi_rdy.in_ = ovec.get(self.tsi_idx_o.in_ready).unwrap();
        if !self.tsi_rdy.out && ovec.get(self.tsi_idx_o.out_valid).unwrap() {
            let mut bits = 0;
            for (i, idx) in self.tsi_idx_o.out_bits.iter().enumerate() {
                bits |= ovec.get(idx).unwrap() << i;
            }
            self.tsi.o.push_back(bits);
        }
    }

    fn parse_ovec(self: &mut Self, ovec: &Vec<u8>) {
        let ovec_bit: BitVec<usize, Lsb0> = BitVec::from_vec(ovec);
        self.parse_axi_output(&ovec_bit);
        self.parse_tsi_output(&ovec_bit);
    }

    fn step(self: &mut Self) {
        let ivec = self.construct_ivec();
        self.driver.io_bridge.push(&mut self.driver.simif, &ivec)?;

        let mut ovec = vec![0u8; ivec.len()];
        'poll_io_out: loop {
            let read_bytes = self.driver.io_bridge.pull(&mut self.driver.simif, &mut ovec)?;
            if read_bytes == 0 {
                ;
            } else {
                break 'poll_io_out;
            }
        }

        self.parse_ovec(&ovec)

        // parse the outputs
        // push requests to DRAM or FESVR

        // self.dram.step();
        // self.fesvr.step();
    }
}

// TODO : Proper ready bit setting
// TODO : DRAM state update and request serving
// TODO : reset signals
