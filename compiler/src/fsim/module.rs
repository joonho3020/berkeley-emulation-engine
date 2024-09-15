use crate::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::primitives::{Primitives, PlatformConfig, NodeMapInfo};
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use std::fmt::Debug;

/// Represents a group of emulation `Processor`s connected together using a
/// all to all communication switch
pub struct Module {
    /// Unique module id
    pub id: u32,

    /// All to all communication switch
    pub switch: Switch,

    /// `Processor`s in this `Module`
    pub procs: Vec<Processor>,

    /// Total number of host machine cycles to emulate one target cycle
    pub host_steps: u32,

    /// Signal mapping
    pub signal_map: IndexMap<String, NodeMapInfo>
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module[\n  {:?}\n{:#?}\n]", self.switch, self.procs)
    }
}

impl Module {
    pub fn new(id: u32, cfg: &PlatformConfig, host_steps_: u32) -> Self {
        Module {
            id: id,
            switch: Switch::new(cfg.num_procs, cfg.inter_proc_nw_lat),
            procs: (0..cfg.num_procs).map(|i| Processor::new(i as u32, host_steps_, cfg)).collect_vec(),
            host_steps: host_steps_,
            signal_map: IndexMap::new()
        }
    }

    pub fn print_2(self: &Self) {
        println!("----- LDM -----");
        for (i, proc) in self.procs.iter().enumerate() {
            print!("{} ", i);
            proc.print_ldm();
        }
        println!("----- SDM -----");
        for (i, proc) in self.procs.iter().enumerate() {
            print!("{} ", i);
            proc.print_sdm();
        }
        println!("\n----- SigMap ----");
        print!("{:?}", self.signal_map);
    }

    pub fn print(self: &Self) {
        println!("-------   Module: {} ------", self.id);

        print!("      ");
        for (i, _) in self.procs.iter().enumerate() {
            print!("   {:02}   ", i);
        }
        print!("\n");

        for pc in 0..self.host_steps {
            if pc == self.procs[0].pc {
                print!("->  {:02}", pc);
            } else {
                print!("    {:02}", pc);
            }
            for (_, proc) in self.procs.iter().enumerate() {
                print!(" | {}{} | ", proc.ldm[pc as usize], proc.sdm[pc as usize]);
            }
            print!("\n");
        }
    }

    pub fn print_sigmap(self: &Self) {
        println!("{:#?}", self.signal_map);
    }

    pub fn compute(self: &mut Self) {
        // compute fout
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.compute();
        }
    }

    pub fn set_local_switch_out(self: &mut Self) {
        // swizzle outputs
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_local_switch_out());
        }
    }

    pub fn set_local_switch_in(self: &mut Self) {
        // set inputs
        for (_, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_local_switch_in(self.switch.get_port_val(switch_in_idx));
        }
    }

    pub fn update_dmem_and_pc(self: &mut Self) {
        // consume network inputs and update processor state
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.update_sdm_and_pc();
        }
    }

    fn step(self: &mut Self) {
        // compute fout
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.compute();
        }

        // swizzle outputs
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_local_switch_out());
        }

        // set inputs
        for (_, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_local_switch_in(self.switch.get_port_val(switch_in_idx));
        }

        // consume network inputs and update processor state
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.update_sdm_and_pc();
        }

        // update switch network
        self.switch.run_cycle();
    }

    pub fn peek(self: &Self, signal: &str) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => Some(self.procs[map.info.coord.proc as usize].ldm[map.info.pc as usize]),
            None => None,
        }
    }

    pub fn poke(self: &mut Self, signal: &str, val: Bit) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => {
                let inst = self.procs[map.info.coord.proc as usize].imem[map.info.pc as usize].clone();
                if inst.opcode == Primitives::Input {
                    self.procs[map.info.coord.proc as usize].set_io_i(val);
                    Some(val)
                } else {
                    println!("Signal {} to poke is not a Input", signal);
                    None
                }
            }
            None => {
                println!("Cannot find signal {} to poke", signal);
                None
            }
        }
    }

    pub fn run_cycle(self: &mut Self, input_stimuli: &IndexMap<u32, Vec<(&str, Bit)>>) {
        for step in 0..self.host_steps {
            match input_stimuli.get(&(step as u32)) {
                Some(vec) => {
                    for (sig, bit) in vec.iter() {
                        self.poke(sig, *bit);
                    }
                }
                None => {}
            };
            self.step();
        }
    }

    pub fn run_cycle_verbose(self: &mut Self, input_stimuli: &IndexMap<u32, Vec<(&str, Bit)>>) {
        println!("-------------------- run cycle ----------------------");
        for step in 0..self.host_steps {
            match input_stimuli.get(&(step as u32)) {
                Some(vec) => {
                    for (sig, bit) in vec.iter() {
                        self.poke(sig, *bit);
                    }
                }
                None => {}
            };
            self.step();
            self.print();
        }
    }

    pub fn nodeindex(self: &Self, signal: &str) -> Option<NodeIndex> {
        match self.signal_map.get(signal) {
            Some(map) => Some(map.idx),
            None => None,
        }
    }
}
