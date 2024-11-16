package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._
import freechips.rocketchip.util.DecoupledHelper

class CtrlBundle(wBits: Int) extends Bundle {
  val rd = Flipped(Decoupled(UInt(wBits.W)))
  val wr = Decoupled(UInt(wBits.W))
}

class AXI4MMIOModule(numRegs: Int, cfg: AXI4BundleParameters) extends Module {
  val io = IO(new Bundle {
    val axi = Flipped(AXI4Bundle(cfg))
    val ctrl = Vec(numRegs, new CtrlBundle(cfg.dataBits))
  })

  val addr_offset = log2Ceil(cfg.dataBits / 8)
  println(s"addr_offset: ${addr_offset}")

  require(numRegs >= 1)
  require(numRegs <= (1 << cfg.addrBits))

  io.axi.b.bits.resp := 0.U(2.W)
  io.axi.b.bits.id := DontCare
  io.axi.r.bits.id := DontCare
  io.axi.r.bits.last := DontCare

  val max_idx = (numRegs - 1).U

  val ridx = io.axi.ar.bits.addr >> addr_offset.U
  val ridx_invalid = ridx > max_idx
  val read_fire = DecoupledHelper(
    io.axi.ar.valid,
    io.axi.r.ready)

  io.axi.r.bits.resp := 0.U(2.W)
  io.axi.r.bits.data := DontCare

  io.ctrl.map(ctrl => {
    ctrl.rd.ready := false.B
    ctrl.wr.valid := false.B
    ctrl.wr.bits  := DontCare
  })

  dontTouch(ridx)
  dontTouch(ridx_invalid)

  val rinput_valid = io.ctrl.zipWithIndex.map({ case (ctrl, i) => {
    Mux(i.U === ridx, ctrl.rd.valid, false.B)
  }}).reduce(_ || _)

  io.axi.ar.ready := read_fire.fire(io.axi.ar.valid, rinput_valid || ridx_invalid)
  io.axi.r.valid  := read_fire.fire(io.axi.r.ready,  rinput_valid || ridx_invalid)

  io.ctrl.zipWithIndex.map({ case (ctrl, i) => {
    when (i.U === ridx) {
      ctrl.rd.ready := read_fire.fire()
      io.axi.r.bits.data := ctrl.rd.bits
    }
  }})

  val widx = io.axi.aw.bits.addr >> addr_offset.U
  val widx_invalid = widx > max_idx
  val write_fire = DecoupledHelper(
    io.axi.aw.valid,
    io.axi.w.valid,
    io.axi.b.ready)

  dontTouch(widx)
  dontTouch(widx_invalid)

  val woutput_ready = io.ctrl.zipWithIndex.map({ case(ctrl, i) => {
    Mux(i.U === widx, ctrl.wr.ready, false.B)
  }}).reduce(_ || _)

  io.axi.aw.ready := write_fire.fire(io.axi.aw.valid, woutput_ready || widx_invalid)
  io.axi.w.ready := write_fire.fire(io.axi.w.valid,   woutput_ready || widx_invalid)
  io.axi.b.valid := write_fire.fire(io.axi.b.ready,   woutput_ready || widx_invalid)

  io.ctrl.zipWithIndex.map({ case (ctrl, i) => {
    when (i.U === widx) {
      ctrl.wr.valid := write_fire.fire()
      ctrl.wr.bits  := io.axi.w.bits.data
    }
  }})
}

object AXI4MMIOModule {
  var idx = 0;

  def tieoff(mmio: AXI4MMIOModule): Unit = {
    mmio.io.ctrl.map(rw => {
      rw.wr.ready := false.B
      rw.rd.valid := false.B
      rw.rd.bits  := DontCare
    })
  }

  def bind_readonly_reg(reg: Data, mmio: AXI4MMIOModule): Unit = {
    assert(mmio.io.ctrl(idx).wr.valid === false.B)
    mmio.io.ctrl(idx).wr.ready := false.B
    mmio.io.ctrl(idx).rd.valid := true.B
    mmio.io.ctrl(idx).rd.bits  := reg
    idx += 1;
  }

  def bind_writeonly_reg(reg: Data, mmio: AXI4MMIOModule): Unit = {
    mmio.io.ctrl(idx).rd.valid := false.B
    mmio.io.ctrl(idx).wr.ready := true.B
    when (mmio.io.ctrl(idx).wr.valid) {
      reg := mmio.io.ctrl(idx).wr.bits
    }
    idx += 1;
  }

  def bind_writeonly_reg_array(regs: Seq[Data], mmio: AXI4MMIOModule): Unit = {
    regs.foreach(r => AXI4MMIOModule.bind_writeonly_reg(r, mmio))
  }

  def bind_readwrite_reg(reg: Data, mmio: AXI4MMIOModule): Unit = {
    mmio.io.ctrl(idx).rd.valid := true.B
    mmio.io.ctrl(idx).rd.bits := reg

    mmio.io.ctrl(idx).wr.ready := true.B
    when (mmio.io.ctrl(idx).wr.valid) {
      reg := mmio.io.ctrl(idx).wr.bits
    }
    idx += 1
  }

  def bind_readwrite_reg_array(regs: Seq[Data], mmio: AXI4MMIOModule): Unit = {
    regs.foreach(r => AXI4MMIOModule.bind_readwrite_reg(r, mmio))
  }
}
