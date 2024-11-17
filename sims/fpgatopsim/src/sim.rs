use crate::dut::*;
use crate::dut_if::*;
use crate::sim_if::*;
use crate::axi::*;
use bee::common::config::PlatformConfig;

#[derive(Debug, Default, Clone)]
pub struct FPGATopConfig {
    pub axi:  AXI4Config,
    pub axil: AXI4Config,
    pub emul: PlatformConfig
}


#[derive(Debug)]
pub struct Sim {
    pub cfg: FPGATopConfig,
    pub dut: *mut VFPGATop,
    vcd: *mut VerilatedVcdC,
    cycle: u32
}

impl Sim {
    pub const MAX_LEN: u32 = 255;

    pub unsafe fn try_new(cfg: &FPGATopConfig) -> Self {
        let dut = FPGATop_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);
        Self {
            cfg: cfg.clone(),
            dut: dut,
            vcd: vcd,
            cycle: 0
        }
    }

    pub unsafe fn finish(self: &mut Self) {
        close_trace(self.vcd);
        FPGATop_delete(self.dut);
    }

    pub fn max_len(self: &Self) -> u32 {
        Self::MAX_LEN
    }
}

impl SimIf for Sim {
    fn finish(self: &mut Self) {
        unsafe { self.finish(); }
    }

    fn step(self: &mut Self) {
        let time = self.cycle * 2;
        unsafe {
            FPGATop_eval(self.dut);
            dump_vcd(self.vcd, time);

            poke_clock(self.dut, 1);
            FPGATop_eval(self.dut);
            dump_vcd(self.vcd, time + 1);

            poke_clock(self.dut, 0);
        }
        self.cycle += 1;
    }

    fn push(self:  &mut Self, addr: u32, data: &Vec<u8>) -> Result<u32, SimIfErr> {
        unsafe { dma_write(self, addr, data.len() as u32, data); }
        return Ok(data.len() as u32);
    }

    fn pull(self:  &mut Self, addr: u32, data: &mut Vec<u8>) -> Result<u32, SimIfErr> {
        let size = data.len() as u32;
        unsafe { dma_read(self, addr, data, size); }
        return Ok(size);
    }

    fn read(self:  &mut Self, addr: u32) -> Result<u32, SimIfErr> {
        return Ok(unsafe { mmio_read(self, addr) });
    }

    fn write(self: &mut Self, addr: u32, data: u32) -> Result<(), SimIfErr> {
        unsafe { mmio_write(self, addr, data) };
        return Ok(());
    }
}
