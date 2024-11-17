use std::fmt::Debug;
use crate::sim_if::*;

pub trait MMIORead: MMIOAddr {
    fn read(self: &Self, sim:  &mut Box<dyn SimIf>) -> Result<u32, SimIfErr> {
        sim.read(self.addr())
    }
}

pub trait MMIOWrite: MMIOAddr {
    fn write(self: &Self, sim: &mut Box<dyn SimIf>, data: u32) -> Result<(), SimIfErr> {
        sim.write(self.addr(), data)
    }
}

pub trait MMIOAddr {
    fn addr(self: &Self) -> u32;
}

#[macro_export]
macro_rules! impl_mmio_if {
    ($struct_name:ident) => {
        impl MMIOAddr for $struct_name {
            fn addr(self: &Self) -> u32 {
                self.addr
            }
        }

        impl $struct_name {
            pub fn new(addr: u32) -> Self {
                Self { addr }
            }
        }
    };
}

// Read MMIO Interface
#[derive(Debug)]
pub struct RdMMIOIf {
    pub addr: u32
}
impl_mmio_if!(RdMMIOIf);
impl MMIORead for RdMMIOIf {}

// Write MMIO Interface
#[derive(Debug)]
pub struct WrMMIOIf {
    pub addr: u32
}
impl_mmio_if!(WrMMIOIf);
impl MMIOWrite for WrMMIOIf {}

// Read Write MMIO Interface
#[derive(Debug)]
pub struct RdWrMMIOIf {
    pub addr: u32
}
impl_mmio_if!(RdWrMMIOIf);
impl MMIORead  for RdWrMMIOIf {}
impl MMIOWrite for RdWrMMIOIf {}
