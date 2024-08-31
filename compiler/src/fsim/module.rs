use crate::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::primitives::{Circuit, NodeMapInfo, Primitives, PlatformConfig};
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use std::fmt::Debug;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
    host_steps: u32, // Total number of host machine cycles to emulate one target cycle
    iprocs: Vec<usize>, // Processor indices that have input IO ports
    oprocs: Vec<usize>, // Processor indices that have output IO ports
    signal_map: IndexMap<String, NodeMapInfo>,
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module[\n  {:?}\n{:#?}\n]", self.switch, self.procs)
    }
}

impl Module {
    pub fn new(nprocs: u32, host_steps_: u32, cfg: &PlatformConfig) -> Self {
        Module {
            switch: Switch::new(nprocs, cfg.network_lat),
            procs: (0..nprocs).map(|i| Processor::new(host_steps_, cfg, i as u32)).collect_vec(),
            host_steps: host_steps_,
            iprocs: vec![],
            oprocs: vec![],
            signal_map: IndexMap::new(),
        }
    }

    /// Given a circuit that went through the compiler pass,
    /// return a Module with instructions from the compiler pass mapped
    pub fn from_circuit(c: &Circuit) -> Self {
    // FIXME: ...
// let all_insts = &c.emulator.instructions;
// let nprocs = all_insts.len() as u32;
// let host_steps = c.emulator.host_steps;

// let mut module = Module::new(nprocs, host_steps, &c.cfg);
// module.set_insts(all_insts.to_vec());
// module.set_signal_map(&c.emulator.signal_map);

// return module;
        return Module::new(10, 10, &c.platform_cfg);
    }

    fn set_insts(self: &mut Self, all_insts: Vec<Vec<Instruction>>) {
        assert!(self.procs.len() >= all_insts.len());
        for (i, insts) in all_insts.iter().enumerate() {
            for (pc, inst) in insts.iter().enumerate() {
                if (pc as u32) < self.host_steps {
                    self.procs[i].set_inst(inst.clone(), pc);
                    if inst.opcode == Primitives::Input {
                        self.iprocs.push(i);
                    } else if inst.opcode == Primitives::Output {
                        self.oprocs.push(i);
                    }
                }
            }
        }
    }

    fn set_signal_map(self: &mut Self, signal_map: &IndexMap<String, NodeMapInfo>) {
        self.signal_map = signal_map.clone()
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

    fn step(self: &mut Self) {
        // compute fout
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.compute();
        }

        // swizzle outputs
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_switch_out());
        }

        // set inputs
        for (_, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_switch_in(self.switch.get_port_val(switch_in_idx));
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
            Some(map) => Some(self.procs[map.info.proc as usize].ldm[map.info.pc as usize]),
            None => None,
        }
    }

    pub fn poke(self: &mut Self, signal: &str, val: Bit) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => {
                let inst = self.procs[map.info.proc as usize].imem[map.info.pc as usize].clone();
                if inst.opcode == Primitives::Input {
                    self.procs[map.info.proc as usize].set_io_i(val);
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
