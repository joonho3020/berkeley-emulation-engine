use crate::fsim::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::instruction::Instruction;
use crate::primitives::{Circuit, NodeInfo, Primitives};
use indexmap::IndexMap;
use std::cmp::max;
use std::fmt::Debug;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
    host_steps: usize, // Total number of host machine cycles to emulate one target cycle
    iprocs: Vec<usize>, // Processor indices that have input IO ports
    oprocs: Vec<usize>, // Processor indices that have output IO ports
    signal_map: IndexMap<String, NodeInfo>,
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
    pub fn from_circuit(c: Circuit) -> Self {
        let all_insts = c.emulator.instructions;

        // get max pc for entire emulator
        let nprocs = all_insts.len();
        let mut max_pc = 0;
        for nidx in c.graph.node_indices() {
            let node = c.graph.node_weight(nidx).unwrap();
            max_pc = max(max_pc, node.get_info().pc + c.emulator.cfg.network_latency);
        }

        let host_steps = max_pc + 1;
        let mut module = Module::new(nprocs, host_steps as usize);

        // set instructions
        module.set_insts(all_insts);

        // set signal mapping
        module.set_signal_map(c.emulator.signal_map);

        return module;
    }

    pub fn set_insts(self: &mut Self, all_insts: Vec<Vec<Instruction>>) {
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

    pub fn set_signal_map(self: &mut Self, signal_map: IndexMap<String, NodeInfo>) {
        self.signal_map = signal_map
    }

    fn print(self: &Self) {
        print!("    ");
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
        for (_, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_switch_in(self.switch.get_port_val(switch_in_idx));
        }
        for (_, proc) in self.procs.iter_mut().enumerate() {
            proc.step();
        }
        // self.print();
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_switch_out());
        }
    }

    pub fn peek(self: Self, signal: String) -> Result<Bit, String> {
        let map = self.signal_map.get(&signal);
        match map {
            Some(info) => Ok(self.procs[info.proc as usize].ldm[info.pc as usize]),
            None => Err(format!("Cannot find signal {} to peek", signal).to_string()),
        }
    }

    pub fn poke(self: &mut Self, signal: String, val: Bit) -> Result<Bit, String> {
        let map = self.signal_map.get(&signal);
        match map {
            Some(info) => {
                let inst = self.procs[info.proc as usize].imem[info.pc as usize].clone();
                if inst.opcode == Primitives::Input {
                    self.procs[info.proc as usize].set_io_i(val);
                    Ok(val)
                } else {
                    Err(format!("Signal {} to poke is not a Input", signal).to_string())
                }
            }
            None => Err(format!("Cannot find signal {} to poke", signal).to_string()),
        }
    }

    pub fn get_outputs(self: &mut Self) -> Vec<Bit> {
        let mut ret = vec![];
        for oproc in self.oprocs.iter() {
            ret.push(self.procs[*oproc].get_io_o());
        }
        ret
    }

    pub fn run_cycle(self: &mut Self) {
        for _ in 0..self.host_steps {
            self.step();
        }
    }
}
