use crate::fsim::common::*;
use crate::fsim::processor::*;
use crate::fsim::switch::*;
use crate::instruction::Instruction;
use crate::primitives::Primitives;
use std::fmt::Debug;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
    max_steps: usize,
    iprocs: Vec<usize>,
    oprocs: Vec<usize>,
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module[\n  {:?}\n{:#?}\n]", self.switch, self.procs)
    }
}

impl Module {
    pub fn new(nprocs: usize, max_steps: usize) -> Self {
        Module {
            switch: Switch::new(nprocs),
            procs: vec![Processor::new(max_steps); nprocs],
            max_steps: max_steps,
            iprocs: vec![],
            oprocs: vec![],
        }
    }

    pub fn set_insts(self: &mut Self, all_insts: Vec<Vec<Instruction>>) {
        assert!(self.procs.len() >= all_insts.len());
        for (i, insts) in all_insts.iter().enumerate() {
            for (pc, inst) in insts.iter().enumerate() {
                self.procs[i].set_inst(inst.clone(), pc);
                if inst.opcode == Primitives::Input {
                    self.iprocs.push(i);
                } else if inst.opcode == Primitives::Output {
                    self.oprocs.push(i);
                }
            }
        }
    }

    fn step(self: &mut Self) {
        for (i, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_switch_in(self.switch.get_port_val(switch_in_idx));
            proc.step();
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
        for _ in 0..self.max_steps {
            self.step();
        }
        self.get_outputs()
    }
}
