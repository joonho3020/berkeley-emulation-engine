use crate::common::*;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

#[derive(Default, Clone, Debug)]
pub struct Token<T> {
    pub value: T,
    pub cycle: Cycle
}

#[derive(Default, Clone, Debug)]
pub struct ReadReq {
    pub addr: Bits,
}

#[derive(Default, Clone, Debug)]
pub struct ReadResp<T> {
    pub data: T,
}

#[derive(Default, Clone, Debug)]
pub struct ReadPort<T> {
    lat: Cycle,
    reqs: Vec<Token<ReadReq>>,
    resps: Vec<ReadResp<T>>,
    cycle: Cycle
}

impl<T: Default + Clone> ReadPort<T> {
    pub fn new(lat: Cycle) -> Self {
        ReadPort {
            lat  : lat as Cycle,
            reqs : vec![],
            resps: vec![],
            cycle: 0
        }
    }

    pub fn run_cycle(self: &mut Self) {
        match self.reqs.first() {
            Some(token) => {
                if token.cycle <= self.cycle {
                    self.reqs.remove(0);
                }
            }
            None => {}
        }
        self.cycle += 1;
    }

    pub fn cur_req(self: &Self) -> Option<ReadReq> {
        match self.reqs.first() {
            Some(t) => {
                if t.cycle <= self.cycle {
                    Some(t.value.clone())
                } else {
                    None
                }
            }
            None => {
                None
            }
        }
    }

    pub fn submit_req(self: &mut Self, req: ReadReq) {
        self.reqs.push(Token {
            value: req,
            cycle: self.cycle + self.lat
        });
    }

    pub fn cur_resp(self: &mut Self) -> Option<ReadResp<T>> {
        match self.resps.first() {
            Some(_) => Some(self.resps.remove(0)),
            None => None
        }
    }

    pub fn set_resp(self: &mut Self, data: T) {
        self.resps.push(ReadResp { data: data.clone() });
    }
}

#[derive(Default, Clone, Debug)]
pub struct WriteReq<T: Default + Clone> {
    pub addr: Bits,
    pub data: T
}

#[derive(Default, Clone, Debug)]
pub struct WritePort<T: Default + Clone> {
    lat: Cycle,
    reqs: Vec<Token<WriteReq<T>>>,
    cycle: Cycle
}

impl<T: Default + Clone> WritePort<T> {
    pub fn new(lat: Cycle) -> Self {
        WritePort {
            lat: lat as Cycle,
            reqs: vec![],
            cycle: 0
        }
    }

    pub fn run_cycle(self: &mut Self) {
        match self.reqs.first() {
            Some(token) => {
                if token.cycle <= self.cycle {
                    self.reqs.remove(0);
                }
            }
            None => {}
        }
        self.cycle += 1;
    }

    pub fn cur_req(self: &Self) -> Option<WriteReq<T>> {
        match self.reqs.first() {
            Some(t) => {
                if t.cycle <= self.cycle {
                    Some(t.value.clone())
                } else {
                    None
                }
            }
            None => {
                None
            }
        }
    }

    pub fn submit_req(self: &mut Self, req: WriteReq<T>) {
        self.reqs.push(Token {
            value: req,
            cycle: self.cycle + self.lat
        });
    }
}



#[derive(Default, Clone, Debug)]
pub struct AbstractMemory<T: Default + Clone> {
    pub data: Vec<T>,
    rports: Vec<ReadPort<T>>,
    wports: Vec<WritePort<T>>
}

impl<T: Default + Clone> AbstractMemory<T> {
    pub fn new(entries: u32, rd_lat: Cycle, rd_ports: u32, wr_lat: Cycle, wr_ports: u32) -> Self {
        AbstractMemory {
            data  : vec![T::default();           entries  as usize],
            rports: vec![ReadPort::new(rd_lat);  rd_ports as usize],
            wports: vec![WritePort::new(wr_lat); wr_ports as usize]
        }
    }

    pub fn update_rd_ports(self: &mut Self) {
        for rport in self.rports.iter_mut() {
            match rport.cur_req() {
                Some(req) => {
                    rport.set_resp(self.data.get(req.addr as usize).unwrap().clone())
                }
                None => {}
            }
        }
    }

    fn update_wr_ports(self: &mut Self) {
        for wport in self.wports.iter_mut() {
            match wport.cur_req() {
                Some(req) => {
                    self.data[req.addr as usize] = req.data;
                }
                None => {}
            }
        }
    }

    pub fn run_cycle(self: &mut Self) {
        for rport in self.rports.iter_mut() {
            rport.run_cycle();
        }
        for wport in self.wports.iter_mut() {
            wport.run_cycle();
        }
        self.update_wr_ports();
    }

    pub fn get_rport(self: &mut Self, i: u32) -> &mut ReadPort<T> {
        self.rports.get_mut(i as usize).unwrap()
    }

    pub fn get_wport(self: &mut Self, i: u32) -> &mut WritePort<T> {
        self.wports.get_mut(i as usize).unwrap()
    }
}

impl<T: Default + Clone> Index<usize> for AbstractMemory<T> {
    type Output = T;
    fn index<'a>(&'a self, i: usize) -> &'a T {
        &self.data[i]
    }
}

impl<T: Default + Clone> IndexMut<usize> for AbstractMemory<T> {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut T {
        &mut self.data[i]
    }
}
