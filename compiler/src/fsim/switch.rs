use crate::common::*;
use crate::fsim::memory::Token;

#[derive(Default, Clone, Debug)]
struct SwitchPort {
    msgs: Vec<Token<Bit>>,
    cycle: Cycle,
    lat: Cycle
}

impl SwitchPort {
    pub fn new(lat: Cycle) -> Self {
        SwitchPort {
            msgs: vec![],
            cycle: 0,
            lat: lat
        }
    }

    pub fn run_cycle(self: &mut Self) {
        match self.msgs.first() {
            Some(token) => {
                if token.cycle <= self.cycle {
                    self.msgs.remove(0);
                }
            }
            None => {}
        }
        self.cycle += 1;
    }

    pub fn cur_req(self: &Self) -> Option<Bit> {
        match self.msgs.first() {
            Some(token) => {
                if token.cycle <= self.cycle {
                    Some(token.value)
                } else {
                    None
                }
            }
            None => {
                None
            }
        }
    }

    pub fn submit_req(self: &mut Self, data: Bit) {
        self.msgs.push(Token {
            value: data,
            cycle: self.cycle + self.lat
        });
    }
}

#[derive(Default, Debug)]
pub struct Switch {
    ports: Vec<SwitchPort>,
}

impl Switch {
    pub fn new(nprocs: u32, lat: Cycle) -> Self {
        Switch {
            ports: vec![SwitchPort::new(lat); nprocs as usize],
        }
    }

    pub fn get_port_val(self: &Self, pid: usize) -> Bit {
        match self.ports[pid].cur_req() {
            Some(x) => x,
            None    => 0 // TODO: should return proper X instead of zero when a value is unknown
        }
    }

    pub fn set_port_val(self: &mut Self, pid: usize, val: Bit) {
        self.ports[pid].submit_req(val);
    }

    pub fn run_cycle(self: &mut Self) {
        for p in self.ports.iter_mut() {
            p.run_cycle();
        }
    }
}
