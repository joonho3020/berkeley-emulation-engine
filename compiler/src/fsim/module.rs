use crate::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::primitives::{Circuit, NodeMapInfo, Primitives};
use indexmap::IndexMap;
use petgraph::graph::NodeIndex;
use std::fmt::Debug;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
    host_steps: usize, // Total number of host machine cycles to emulate one target cycle
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
    pub fn new(nprocs: usize, host_steps_: usize) -> Self {
        Module {
            switch: Switch::new(nprocs),
            procs: vec![Processor::new(host_steps_); nprocs],
            host_steps: host_steps_,
            iprocs: vec![],
            oprocs: vec![],
            signal_map: IndexMap::new(),
        }
    }

    /// Given a circuit that went through the compiler pass,
    /// return a Module with instructions from the compiler pass mapped
    pub fn from_circuit(c: &Circuit) -> Self {
        let all_insts = &c.emulator.instructions;
        let nprocs = all_insts.len();
        let host_steps = c.emulator.host_steps;

        let mut module = Module::new(nprocs, host_steps as usize);
        module.set_insts(all_insts.to_vec());
        module.set_signal_map(&c.emulator.signal_map);

        return module;
    }

    fn set_insts(self: &mut Self, all_insts: Vec<Vec<Instruction>>) {
        assert!(self.procs.len() >= all_insts.len());
        for (i, insts) in all_insts.iter().enumerate() {
            for (pc, inst) in insts.iter().enumerate() {
                if pc < self.host_steps {
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

    fn print_2(self: &Self) {
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
    }

    fn print(self: &Self) {
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
                print!(" | {}{} | ", proc.ldm[pc], proc.sdm[pc]);
            }
            print!("\n");
        }
    }

    fn step(self: &mut Self) {
        // compute fout
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.compute_fout();
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
    }

    pub fn peek(self: &Self, signal: &str) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => Some(self.procs[map.info.proc as usize].ldm[map.info.pc as usize]),
            None => None,
        }
    }

    pub fn poke(self: &mut Self, signal: String, val: Bit) -> Option<Bit> {
        match self.signal_map.get(&signal) {
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

    pub fn run_cycle(self: &mut Self) {
        println!("=============================================================");
        for i in 0..self.host_steps {
            self.step();
            println!("pc: {}", i);
            self.print_2();
        }
    }

    pub fn nodeindex(self: &Self, signal: &str) -> Option<NodeIndex> {
        match self.signal_map.get(signal) {
            Some(map) => Some(map.idx),
            None => None,
        }
    }
}
