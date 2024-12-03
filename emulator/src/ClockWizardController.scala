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

  val max_mmio_regs = 5
  val mmio = Module(new AXI4MMIOModule(max_mmio_regs, cfg.axil.axi4BundleParams, 0x10000))
  AXI4MMIOModule.tieoff(mmio)
  dontTouch(mmio.io.axi)



  val aw_q = Queue.irrevocable(io.axil.aw, 4)
  val w_q  = Queue.irrevocable(io.axil. w, 4)
  val b_q  = Queue.irrevocable(mmio.io.axi.b, 4)
  val ar_q = Queue.irrevocable(io.axil.ar, 4)
  val r_q  = Queue.irrevocable(mmio.io.axi.r, 4)


  mmio.io.axi.aw <> aw_q
  mmio.io.axi.w  <> w_q
  io.axil.b <> b_q

  mmio.io.axi.ar <> ar_q
  io.axil.r <> r_q

  val fingerprint_reg = RegInit(BigInt("AAC0FFEE", 16).U(32.W))

  val pll_locked = RegNext(io.clk_wiz_locked)

  val pll_reset = RegInit(false.B)
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

  mmio.io.ctrl(3).wr.ready := true.B
  when (mmio.io.ctrl(3).wr.valid) {
    fingerprint_reg := mmio.io.ctrl(3).wr.bits
  }
  mmio.io.ctrl(3).rd.valid := true.B
  mmio.io.ctrl(3).rd.bits := fingerprint_reg

  val pll_reset_cycle = RegInit(10.U(32.W))

  when (mmio.io.ctrl(4).wr.valid) {
    pll_reset_cycle := mmio.io.ctrl(4).wr.bits
  }

  val pll_reset_cntr = RegInit(0.U(32.W))
  when (pll_reset) {
    pll_reset_cntr := pll_reset_cntr + 1.U
    when (pll_reset_cycle === pll_reset_cntr - 1.U) {
      pll_reset_cntr := 0.U
      pll_reset := false.B
    }
  } .otherwise {
    pll_reset_cntr := 0.U
  }
}
