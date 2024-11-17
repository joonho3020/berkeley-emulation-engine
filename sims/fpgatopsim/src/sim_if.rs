use std::fmt::Debug;
use crate::axi::*;
use bee::common::config::PlatformConfig;

pub type SimIfErr = Box<dyn std::error::Error>;

#[derive(Debug, Default, Clone)]
pub struct FPGATopConfig {
    pub axi:  AXI4Config,
    pub axil: AXI4Config,
    pub emul: PlatformConfig
}

pub trait SimIf: Debug {
    fn finish(self: &mut Self);
    fn step(self: &mut Self);
    fn push(self:  &mut Self, addr: u32, data: &Vec<u8>) -> Result<u32, SimIfErr>;
    fn pull(self:  &mut Self, addr: u32, data: &mut Vec<u8>) -> Result<u32, SimIfErr>;
    fn read(self:  &mut Self, addr: u32) -> Result<u32, SimIfErr>;
    fn write(self: &mut Self, addr: u32, data: u32) -> Result<(), SimIfErr>;
}

pub trait DMAOps: DMAAddrs {
    fn push(self: &Self, sim: &mut Box<dyn SimIf>, data: &Vec<u8>) -> Result<u32, SimIfErr> {
        let empty_bytes = sim.read(self.empty_addr())?;
        let pushed_bytes = if empty_bytes >= data.len() as u32 {
            sim.push(self.enq_addr(), data)?
        } else {
            0
        }; Ok(pushed_bytes)
    }

    fn pull(self: &Self, sim: &mut Box<dyn SimIf>, data: &mut Vec<u8>) -> Result<u32, SimIfErr> {
        let filled_bytes = sim.read(self.filled_addr())?;
        let pulled_bytes = if filled_bytes >= data.len() as u32 {
            sim.pull(self.deq_addr(), data)?
        } else {
            0
        };
        return Ok(pulled_bytes);
    }
}

pub trait DMAAddrs {
    fn enq_addr(self: &Self) -> u32;
    fn deq_addr(self: &Self) -> u32;
    fn filled_addr(self: &Self) -> u32;
    fn empty_addr(self: &Self) -> u32;
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
pub struct DMAIf {
    pub dma_addrs: DMAAddrRegs
}

impl DMAAddrs for DMAIf {
    fn enq_addr(self: &Self) -> u32 {
        self.dma_addrs.addr
    }
    fn deq_addr(self: &Self) -> u32 {
        self.dma_addrs.addr
    }
    fn filled_addr(self: &Self) -> u32 {
        self.dma_addrs.filled
    }
    fn empty_addr(self: &Self) -> u32 {
        self.dma_addrs.empty
    }
}

impl DMAOps for DMAIf {
}

impl DMAIf {
    pub fn new(addrs: DMAAddrRegs) -> Self {
        Self {
            dma_addrs: addrs
        }
    }
}

pub trait MMIOOps: MMIOAddr {
    fn read(self: &Self, sim:  &mut Box<dyn SimIf>) -> Result<u32, SimIfErr> {
        sim.read(self.addr())
    }
    fn write(self: &Self, sim: &mut Box<dyn SimIf>, data: u32) -> Result<(), SimIfErr> {
        sim.write(self.addr(), data)
    }
}

pub trait MMIOAddr {
    fn addr(self: &Self) -> u32;
}

#[derive(Debug)]
pub struct MMIOIf {
    pub mmio_addr: u32
}

impl MMIOAddr for MMIOIf {
    fn addr(self: &Self) -> u32 {
        self.mmio_addr
    }
}

impl MMIOOps for MMIOIf {
}

impl MMIOIf {
    pub fn new(addr: u32) -> Self {
        Self {
            mmio_addr: addr
        }
    }
}

#[derive(Debug)]
pub struct SRAMConfig {
    pub ptype: MMIOIf,
    pub mask: MMIOIf,
    pub width: MMIOIf,
}

impl SRAMConfig {
    pub fn new(paddr: u32, maddr: u32, waddr: u32) -> Self {
        Self {
            ptype: MMIOIf::new(paddr),
            mask: MMIOIf::new(maddr),
            width: MMIOIf::new(waddr),
        }
    }
}

#[derive(Debug)]
pub struct ControlIf {
    pub sram: Vec<SRAMConfig>,
    pub host_steps: MMIOIf,
    pub target_cycle_lo: MMIOIf,
    pub target_cycle_hi: MMIOIf,
    pub fingerprint: MMIOIf,
    pub init_done: MMIOIf,
}

#[derive(Debug)]
pub struct Driver
{
    pub simif: Box<dyn SimIf>,
    pub io_bridge:   DMAIf,
    pub inst_bridge: DMAIf,
    pub dbg_bridge:  DMAIf,
    pub ctrl_bridge: ControlIf
}

// TODO: Use macros
