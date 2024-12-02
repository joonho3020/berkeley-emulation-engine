package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._
import freechips.rocketchip.util.DecoupledHelper
import java.io._


class ClockWizardControllerBundle(cfg: FPGATopParams) extends Bundle {
  val axil = Flipped(AXI4Bundle(cfg.axil.axi4BundleParams))
  val clk_wiz_locked = Input(Bool())
  val clk_wiz_reset  = Output(Bool())
  val fpga_top_ctrl_resetn = Output(Bool())
}

class ClockWizardController(cfg: FPGATopParams) extends Module {
  val io = IO(new ClockWizardControllerBundle(cfg))

  ////////////////////////////////////////////////////////////////////////////
  // MMIO
  ////////////////////////////////////////////////////////////////////////////

  val max_mmio_regs = 3
  val mmio = Module(new AXI4MMIOModule(max_mmio_regs, cfg.axil.axi4BundleParams, 0x10000))
  AXI4MMIOModule.tieoff(mmio)
  dontTouch(mmio.io.axi)

  mmio.io.axi <> io.axil

  val pll_locked = RegNext(io.clk_wiz_locked)

  val pll_reset = RegInit(true.B)
  io.clk_wiz_reset := pll_reset

  val fpga_top_resetn = RegInit(false.B)
  io.fpga_top_ctrl_resetn := fpga_top_resetn

  mmio.io.ctrl(0).rd.valid := true.B
  mmio.io.ctrl(0).rd.bits  := pll_locked

  mmio.io.ctrl(1).wr.ready := true.B
  when (mmio.io.ctrl(1).wr.valid) {
    pll_reset := mmio.io.ctrl(1).wr.bits
  }

  mmio.io.ctrl(2).wr.ready := true.B
  when (mmio.io.ctrl(2).wr.valid) {
    fpga_top_resetn := mmio.io.ctrl(2).wr.bits
  }
}
