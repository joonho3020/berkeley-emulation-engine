use crate::fsim::processor::*;
use crate::fsim::switch::*;
use std::fmt::Debug;

pub struct Module {
    switch: Switch,
    procs: Vec<Processor>,
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
        }
    }

    pub fn step(self: &mut Self) {
        for (_, proc) in self.procs.iter_mut().enumerate() {
            let switch_in_idx = proc.get_switch_in_id() as usize;
            proc.set_switch_in(self.switch.get_port_val(switch_in_idx));
            proc.step();
        }
        for (i, proc) in self.procs.iter_mut().enumerate() {
            self.switch.set_port_val(i, proc.get_switch_out());
        }
    }

    pub fn run(self: &mut Self) {
        // FIXME
        for s in 0..10 {
            self.step();
            println!("{} {:?}", s, self);
        }
    }
}
