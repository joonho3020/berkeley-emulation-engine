package emulator

import chisel3._
import chisel3.util._
import scala.math.{min, max}
import freechips.rocketchip.util.{DecoupledHelper, ParameterizedBundle, HellaPeekingArbiter}

class MCRIO(numCRs: Int)(cfg: NastiParameters) extends Bundle {
  val read  = Vec(numCRs, Flipped(Decoupled(UInt(cfg.nastiXDataBits.W))))
  val write = Vec(numCRs,         Decoupled(UInt(cfg.nastiXDataBits.W)))
// val wstrb = Output(UInt(cfg.nastiWStrobeBits.W))
}

class MCRFile(numRegs: Int)(cfg: NastiParameters) extends Module {
  val io = IO(new Bundle {
    val nasti = Flipped(new NastiIO(cfg))
    val mcr   = new MCRIO(numRegs)(cfg)
  })

  dontTouch(io)

  //TODO: Just use a damn state machine.
  val rValid    = RegInit(false.B)
  val arFired   = RegInit(false.B)
  val awFired   = RegInit(false.B)
  val wFired    = RegInit(false.B)
  val wCommited = RegInit(false.B)
  val bId       = Reg(UInt(cfg.idBits.W))
  val rId       = Reg(UInt(cfg.idBits.W))
  val rData     = Reg(UInt(cfg.nastiXDataBits.W))
  val wData     = Reg(UInt(cfg.nastiXDataBits.W))
  val wIndex    = Reg(UInt(log2Up(numRegs).W))
  val rIndex    = Reg(UInt(log2Up(numRegs).W))
// val wStrb     = Reg(UInt(cfg.nastiWStrobeBits.W))

  when(io.nasti.aw.fire) {
    awFired := true.B
    wIndex  := io.nasti.aw.bits.addr >> log2Up(cfg.nastiWStrobeBits)
    bId     := io.nasti.aw.bits.id
    assert(io.nasti.aw.bits.len === 0.U)
  }

  when(io.nasti.w.fire) {
    wFired := true.B
    wData  := io.nasti.w.bits.data
// wStrb  := io.nasti.w.bits.strb
  }

  when(io.nasti.ar.fire) {
    arFired := true.B
    rIndex  := (io.nasti.ar.bits.addr >> log2Up(cfg.nastiWStrobeBits))(log2Up(numRegs) - 1, 0)
    rId     := io.nasti.ar.bits.id
    assert(io.nasti.ar.bits.len === 0.U, "MCRFile only support single beat reads")
  }

  when(io.nasti.r.fire) {
    arFired := false.B
  }

  when(io.nasti.b.fire) {
    awFired   := false.B
    wFired    := false.B
    wCommited := false.B
  }

  when(io.mcr.write(wIndex).fire) {
    wCommited := true.B
  }

  io.mcr.write.foreach { w => w.valid := false.B; w.bits := wData }
  io.mcr.write(wIndex).valid := awFired && wFired && ~wCommited
  io.mcr.read.zipWithIndex.foreach { case (decoupled, idx: Int) =>
    decoupled.ready := (rIndex === idx.U) && arFired && io.nasti.r.ready
  }

  io.nasti.r.bits  := NastiReadDataChannel(rId, io.mcr.read(rIndex).bits)(cfg)
  io.nasti.r.valid := arFired && io.mcr.read(rIndex).valid

  io.nasti.b.bits  := NastiWriteResponseChannel(bId)(cfg)
  io.nasti.b.valid := awFired && wFired && wCommited

  io.nasti.ar.ready := ~arFired
  io.nasti.aw.ready := ~awFired
  io.nasti.w.ready  := ~wFired
}

object MCRFile {
  def tieoff(mcr: MCRFile): Unit = {
    mcr.io.mcr.write.map(w => {
      w.ready := false.B
    })
    mcr.io.mcr.read.map(r => {
      r.valid := false.B
      r.bits := 0.U
    })
  }

  def bind_readonly_reg(reg: Data, mcr: MCRFile, idx: Int): Unit = {
    assert(mcr.io.mcr.write(idx).valid === false.B)
    mcr.io.mcr.write(idx).ready := false.B
    mcr.io.mcr.read(idx).valid := true.B
    mcr.io.mcr.read(idx).bits  := reg
  }

  def bind_writeonly_reg(reg: Data, mcr: MCRFile, idx: Int): Unit = {
    mcr.io.mcr.read(idx).valid := false.B
    mcr.io.mcr.write(idx).ready := true.B
    when (mcr.io.mcr.write(idx).valid) {
      reg := mcr.io.mcr.write(idx).bits
    }
  }

  def bind_writeonly_reg_array(regs: Seq[Data], mcr: MCRFile, offset: Int): Unit = {
    regs.zipWithIndex.foreach({ case (r, i) => MCRFile.bind_writeonly_reg(r, mcr, i + offset) })
  }
}
