use std::fmt::Debug;
use crate::simif::mmioif::*;
use crate::simif::dmaif::*;

pub type SimIfErr = Box<dyn std::error::Error>;

pub trait SimIf: Debug {
    fn finish(self: &mut Self);
    fn step(self: &mut Self);
    fn push(self:  &mut Self, addr: u32, data: &Vec<u8>) -> Result<u32, SimIfErr>;
    fn pull(self:  &mut Self, addr: u32, data: &mut Vec<u8>) -> Result<u32, SimIfErr>;
    fn read(self:  &mut Self, addr: u32) -> Result<u32, SimIfErr>;
    fn write(self: &mut Self, addr: u32, data: u32) -> Result<(), SimIfErr>;
}


#[derive(Debug)]
pub struct SRAMConfig {
    pub ptype: WrMMIOIf,
    pub mask: WrMMIOIf,
    pub width: WrMMIOIf,
}

impl SRAMConfig {
    pub fn new(paddr: u32, maddr: u32, waddr: u32) -> Self {
        Self {
            ptype: WrMMIOIf::new(paddr),
            mask: WrMMIOIf::new(maddr),
            width: WrMMIOIf::new(waddr),
        }
    }
}

#[derive(Debug)]
pub struct ControlIf {
    pub pll_locked: RdMMIOIf,
    pub pll_reset: WrMMIOIf,
    pub custom_resetn: WrMMIOIf,

    pub sram: Vec<SRAMConfig>,

    pub fingerprint: RdWrMMIOIf,
    pub host_steps: RdWrMMIOIf,

    pub host_steps_prv_deq: RdMMIOIf,
    pub host_steps_prv_cnt: RdMMIOIf,
    pub host_steps_cur_deq: RdMMIOIf,
    pub host_steps_cur_cnt: RdMMIOIf,

    pub init_done: RdMMIOIf,
    pub tot_insts_pushed: RdMMIOIf,

    pub pcs_are_zero: RdMMIOIf,
    pub dbg_proc_0_init: RdMMIOIf,
    pub dbg_proc_n_init: RdMMIOIf,

    pub midx_mismatch_deq: RdMMIOIf,
    pub midx_mismatch_cnt: RdMMIOIf,

    pub pidx_mismatch_deq: RdMMIOIf,
    pub pidx_mismatch_cnt: RdMMIOIf,

    pub dbg_init_cntrs: Vec<RdMMIOIf>,

    pub target_cycle_lo: RdMMIOIf,
    pub target_cycle_hi: RdMMIOIf,
}

#[derive(Debug)]
pub struct Driver
{
    pub simif: Box<dyn SimIf>,
    pub io_bridge:   PushPullDMAIf,
    pub inst_bridge: PushPullDMAIf,
    pub dbg_bridge:  PushPullDMAIf,
    pub ctrl_bridge: ControlIf
}
