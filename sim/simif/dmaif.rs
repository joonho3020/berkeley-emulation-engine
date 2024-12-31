use std::fmt::Debug;
use crate::simif::simif::*;

pub trait DMAPush: PushDMAAddrs {
    fn push(self: &Self, sim: &mut Box<dyn SimIf>, data: &Vec<u8>) -> Result<u32, SimIfErr> {
        let empty_bytes = sim.read(self.empty_addr())?;
        let pushed_bytes = if empty_bytes >= data.len() as u32 {
            sim.push(self.deq_addr(), data)?
        } else {
            0
        };
        Ok(pushed_bytes)
    }
}

pub trait DMAPull: PullDMAAddrs {
    fn pull(self: &Self, sim: &mut Box<dyn SimIf>, data: &mut Vec<u8>) -> Result<u32, SimIfErr> {
        let filled_bytes = sim.read(self.filled_addr())?;
        let pulled_bytes = if filled_bytes >= data.len() as u32 {
            sim.pull(self.enq_addr(), data)?
        } else {
            0
        };
        return Ok(pulled_bytes);
    }
}

pub trait PushDMAAddrs {
    fn deq_addr(self: &Self) -> u32;
    fn empty_addr(self: &Self) -> u32;
}

pub trait PullDMAAddrs {
    fn enq_addr(self: &Self) -> u32;
    fn filled_addr(self: &Self) -> u32;
}

#[derive(Debug, Default)]
pub struct DMAAddrRegs {
    pub addr: u32,
    pub filled: u32,
    pub empty: u32
}

impl DMAAddrRegs {
    pub fn new(addr: u32, filled: u32, empty: u32) -> Self {
        Self {
            addr: addr,
            filled: filled,
            empty: empty
        }
    }
}

#[derive(Debug)]
pub struct PullDMAIf {
    pub addr: u32,
    pub mmio: u32,
}

impl PullDMAAddrs for PullDMAIf {
    fn enq_addr(self: &Self) -> u32 {
        self.addr
    }
    fn filled_addr(self: &Self) -> u32 {
        self.mmio
    }
}

impl PullDMAIf {
    pub fn new(addr: u32, mmio: u32) -> Self {
        Self {
            addr: addr,
            mmio: mmio
        }
    }
}

impl DMAPull for PullDMAIf {
}

#[derive(Debug)]
pub struct PushDMAIf {
    pub addr: u32,
    pub mmio: u32,
}

impl PushDMAAddrs for PushDMAIf {
    fn deq_addr(self: &Self) -> u32 {
        self.addr
    }
    fn empty_addr(self: &Self) -> u32 {
        self.mmio
    }
}

impl PushDMAIf {
    pub fn new(addr: u32, mmio: u32) -> Self {
        Self {
            addr: addr,
            mmio: mmio
        }
    }
}

impl DMAPush for PushDMAIf {
}

#[derive(Debug)]
pub struct PushPullDMAIf {
    pub addr: u32,
    pub filled: u32,
    pub empty: u32,
}

impl PushDMAAddrs for PushPullDMAIf {
    fn deq_addr(self: &Self) -> u32 {
        self.addr
    }
    fn empty_addr(self: &Self) -> u32 {
        self.empty
    }
}

impl PullDMAAddrs for PushPullDMAIf {
    fn enq_addr(self: &Self) -> u32 {
        self.addr
    }
    fn filled_addr(self: &Self) -> u32 {
        self.filled
    }
}

impl PushPullDMAIf {
    pub fn new(addr: u32, filled: u32, empty: u32) -> Self {
        Self {
            addr: addr,
            filled: filled,
            empty: empty
        }
    }
}

impl DMAPush for PushPullDMAIf {
}

impl DMAPull for PushPullDMAIf {
}
