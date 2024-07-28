use crate::fsim::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::instruction::Instruction;
use crate::primitives::{Primitives, Circuit};
use std::fmt::Debug;
use std::cmp::max;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
    host_steps: usize,   // Total number of host machine cycles to emulate one target cycle
    iprocs: Vec<usize>, // Processor indices that have input IO ports
    oprocs: Vec<usize>, // Processor indices that have output IO ports
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
        }
    }

    /// Given a circuit that went through the compiler pass,
    /// return a Module with instructions from the compiler pass mapped
    pub fn from_circuit(c: Circuit) -> Self {
        let all_insts = c.emulator.instructions;
        let nprocs = all_insts.len();
        let mut max_pc = 0;
        for nidx in c.graph.node_indices() {
            let node = c.graph.node_weight(nidx).unwrap();
            max_pc = max(max_pc, node.get_info().pc + c.emulator.cfg.network_latency);
        }

        let host_steps = max_pc + 1;
        let mut module = Module::new(nprocs, host_steps as usize);
        module.set_insts(all_insts);

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
        println!("self.iprocs: {:?}", self.iprocs);
        println!("self.oprocs: {:?}", self.oprocs);
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
        self.print();
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_switch_out());
        }
    }

    fn set_inputs(self: &mut Self, ibits: Vec<Bit>) {
        assert!(
            ibits.len() == self.iprocs.len(),
            "expected {} input bits, got {} bits",
            self.iprocs.len(),
            ibits.len()
        );
        for (ibit, iproc) in ibits.iter().zip(self.iprocs.iter()) {
            self.procs[*iproc].set_io_i(*ibit);
        }
    }

    fn get_outputs(self: &mut Self) -> Vec<Bit> {
        let mut ret = vec![];
        for oproc in self.oprocs.iter() {
            ret.push(self.procs[*oproc].get_io_o());
        }
        ret
    }

    pub fn run_cycle(self: &mut Self, ibits: Vec<Bit>) -> Vec<Bit> {
        self.set_inputs(ibits);
        for _ in 0..self.host_steps {
            self.step();
        }
        self.get_outputs()
    }
}
