use indexmap::IndexMap;
use std::fs;
use rand::prelude::*;

use crate::common::utils::write_string_to_file;

pub type InputStimuliMap = IndexMap<String, Vec<u64>>;

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub width: u64,
    pub input: bool,
}

/// Parses a verilog file `verilog_str`, searches for the `top` module
/// and returns a list of ports for that module
pub fn get_io(verilog_str: String, top: String) -> Vec<Port> {
    let mut collect_io = false;
    let mut cur_dir_input = true;
    let mut cur_bits_minus_one = 0;
    let mut ret: Vec<Port> = vec![];

    for line in verilog_str.lines() {
        let words: Vec<&str> = line.split(' ').filter(|x| *x != "").collect();
        if collect_io {
            if words.len() >= 1 && words[0] == ");" {
                break;
            } else if words.len() >= 3 {
                cur_dir_input = words[0] == "input";

                let x: Vec<&str> = words[1].split(':').collect();
                cur_bits_minus_one = x[0].replace('[', "").parse().unwrap();
            } else if words.len() == 2 {
                if words[0] == "input" || words[0] == "output" {
                    cur_dir_input = words[0] == "input";
                    cur_bits_minus_one = 0;
                } else {
                    let x: Vec<&str> = words[0].split(':').collect();
                    cur_bits_minus_one = x[0].replace('[', "").parse().unwrap();
                }
            }

            assert!(
                cur_bits_minus_one + 1 <= 64,
                "Can support up to 64 bit wires"
            );

            ret.push({
                Port {
                    name: words[words.len() - 1].replace(",", "").to_string(),
                    width: cur_bits_minus_one + 1,
                    input: cur_dir_input,
                }
            })
        } else if words.len() >= 2 && words[0] == "module" && words[1].replace('(', "") == top {
            collect_io = true;
        }
    }
    return ret;
}

/// Parses a file containing the stimuli of a test module
pub fn get_input_stimuli(file_path: &str) -> InputStimuliMap {
    let input_str = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => "".to_string(),
    };

    let mut ret = IndexMap::new();
    let mut order = IndexMap::new();
    for (i, line) in input_str.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|x| *x != "").collect();
        for (j, w) in words.iter().enumerate() {
            if i == 0 {
                order.insert(j, w.to_string());
                ret.insert(w.to_string(), vec![]);
            } else {
                let k = &order[j];
                ret.get_mut(k).unwrap().push(w.parse().unwrap());
            }
        }
    }

    ret
}

pub fn bitblast_input_stimuli(
    input_stimuli: &InputStimuliMap,
    ports: &Vec<Port>,
) -> InputStimuliMap {
    let mut input_stimuli_blasted: InputStimuliMap = InputStimuliMap::new();
    for port in ports.iter() {
        if !input_stimuli.contains_key(&port.name) {
            continue;
        }

        let stimuli_for_input = input_stimuli.get(&port.name).unwrap();
        if port.width == 1 {
            input_stimuli_blasted.insert(port.name.clone(), stimuli_for_input.clone());
        } else {
            for idx in 0..port.width {
                let mut port_name = port.name.clone();
                port_name.push_str(&format!("[{}]", idx));
                input_stimuli_blasted.insert(port_name.clone(), vec![]);

                for stimuli in stimuli_for_input {
                    let bit = ((*stimuli) >> idx) & 0x1;
                    input_stimuli_blasted.get_mut(&port_name).unwrap().push(bit);
                }
            }
        }
    }
    return input_stimuli_blasted;
}

pub fn bitblasted_port_names(ports: &Vec<Port>) -> Vec<String> {
    let mut ret = vec![];
    for port in ports.iter() {
        if port.width == 1 {
            ret.push(port.name.clone());
        } else {
            for idx in 0..port.width {
                let mut port_name = port.name.clone();
                port_name.push_str(&format!("[{}]", idx));
                ret.push(port_name);
            }
        }
    }
    return ret;
}

pub fn aggregate_bitblasted_values(
    ports: &Vec<Port>,
    blasted_values: &mut InputStimuliMap,
) -> InputStimuliMap {
    let mut aggregated: InputStimuliMap = InputStimuliMap::new();
    for port in ports.iter() {
        if port.input {
            continue;
        }

        aggregated.insert(port.name.clone(), vec![]);
        if port.width == 1 {
            match blasted_values.get_mut(&port.name) {
                Some(v) => {
                    aggregated.get_mut(&port.name).unwrap().append(v);
                }
                None => {
                    continue;
                }
            }
        } else {
            for idx in 0..port.width {
                let mut port_name = port.name.clone();
                port_name.push_str(&format!("[{}]", idx));

                match blasted_values.get(&port_name) {
                    Some(bits) => {
                        for (cycle, bit) in bits.iter().enumerate() {
                            let x = aggregated.get_mut(&port.name).unwrap();
                            if x.len() < (cycle + 1) {
                                x.push(0);
                            }
                            x[cycle] = x[cycle] + (bit << idx);
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
        }
    }
    return aggregated;
}

fn random_number(bits: u32) -> u32 {
    let max_plus_one = 1 << bits;
    let mut rng = rand::thread_rng();
    return rng.gen_range(0..max_plus_one);
}

pub fn generate_random_test_data(file_path: &str, ports: &Vec<Port>, ncycles: u32) -> std::io::Result<()> {
    let mut ret = "".to_string();
    let iports: Vec<Port> = ports.iter().filter(|x| x.input).map(|x| x.clone()).collect();

    for ip in iports.iter() {
        if ip.name != "clock" {
            ret.push_str(&format!("{} ", ip.name));
        }
    }
    ret.push_str("\n");

    for _ in 0..ncycles {
        for ip in iports.iter() {
            if ip.name == "clock" {
                continue;
            } else if ip.name == "reset" {
                let len = ip.name.len(); 
                ret.push_str(&format!("{:width$} ", 0, width = len));
            } else {
                let len = ip.name.len(); 
                ret.push_str(&format!("{:width$} ",
                        random_number(ip.width as u32), width = len));
            }
        }
        ret.push_str("\n");
    }
    write_string_to_file(ret, file_path)?;
    Ok(())
}

pub fn binary_to_decimal(binary: &str) -> Result<u128, std::num::ParseIntError> {
    u128::from_str_radix(binary, 2)
}

fn last_five_chars(s: &str) -> Option<String> {
    let end = s.chars().count();
    if end >= 5 {
        let start = end - 5;
        let last_five: String = s.chars().skip(start).collect();
        return Some(last_five);
    } else {
        return None;
    }
}

pub fn is_clock_signal(s: &str) -> bool {
    match last_five_chars(s) {
        Some(x) => {
            x == "clock"
        }
        None => {
            false
        }
    }
}

pub fn is_reset_signal(s: &str) -> bool {
    match last_five_chars(s) {
        Some(x) => {
            x == "reset"
        }
        None => {
            false
        }
    }
}

pub fn is_clock_tap(s: &str) -> bool {
    s == "clock_tap"
}


pub fn is_debug_reset(s: &str) -> bool {
    (s == "debug_systemjtag_reset") || (s == "debug_reset")
}
