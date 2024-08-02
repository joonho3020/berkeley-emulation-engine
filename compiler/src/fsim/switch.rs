use crate::common::*;

#[derive(Default, Debug)]
pub struct Switch {
    ports: Vec<Bit>,
}

impl Switch {
    pub fn new(nprocs: usize) -> Self {
        Switch {
            ports: vec![0; nprocs],
        }
    }

    pub fn get_port_val(self: &Self, pid: usize) -> Bit {
        self.ports[pid]
    }

    pub fn set_port_val(self: &mut Self, pid: usize, val: Bit) {
        self.ports[pid] = val;
    }
}
