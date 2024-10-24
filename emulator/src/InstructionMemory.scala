package emulator

import chisel3._
import chisel3.util._

class Operand(cfg: EmulatorConfig) extends Bundle {
  val rs = UInt(cfg.index_bits.W)
  val local = Bool()
}

class SwitchInfo(cfg: EmulatorConfig) extends Bundle {
  val idx   = UInt(cfg.switch_bits.W)
  val local = Bool()
  val fwd   = Bool()
}

class Instruction(cfg: EmulatorConfig) extends Bundle {
  val opcode = UInt(cfg.opcode_bits.W)
  val lut    = UInt(cfg.lut_bits.W)
  val ops    = Vec(cfg.lut_inputs, new Operand(cfg))
  val sinfo  = new SwitchInfo(cfg)
  val mem    = Bool()
}

object Instruction {
  def apply(cfg: EmulatorConfig): Instruction = {
    new Instruction(cfg)
  }

  val NOP      = 0
  val Input    = 1
  val Output   = 2
  val Lut      = 3
  val ConstLut = 4
  val Gate     = 5
  val Latch    = 6
  val SRAMIn   = 7
  val SRAMOut  = 8
}

class AbstractInstMem(cfg: EmulatorConfig) extends Module {
  val io = IO(new Bundle {
    val pc    = Input(UInt(cfg.index_bits.W))
    val wen   = Input(Bool())
    val winst = Input (new Instruction(cfg))
    val rinst = Output(new Instruction(cfg))
  })
}

class CombInstMem(cfg: EmulatorConfig) extends AbstractInstMem(cfg) {
  require(cfg.imem_lat == 0)

  io.rinst := DontCare

  val mem = Seq.fill(cfg.max_steps)(Reg(Instruction(cfg)))
  for (i <- 0 until cfg.max_steps) {
    when (i.U === io.pc) {
      when (io.wen) {
        mem(i) := io.winst
      } .otherwise {
        io.rinst := mem(i)
      }
    }
  }
}

class SRAMInstMem(cfg: EmulatorConfig) extends AbstractInstMem(cfg) {
  require(cfg.imem_lat == 1)

  io.rinst := DontCare

  val inst_bits = Instruction(cfg).getWidth
  val mem = SyncReadMem(cfg.max_steps, UInt(inst_bits.W))
  val port = mem(io.pc)
  when (io.wen) {
    port := io.winst.asUInt
  } .otherwise {
    io.rinst := port.asTypeOf(Instruction(cfg))
  }
}

class InstMem(cfg: EmulatorConfig) extends AbstractInstMem(cfg) {
  val mem = if (cfg.imem_lat == 0) {
    Module(new CombInstMem(cfg))
  } else {
    Module(new SRAMInstMem(cfg))
  }

  io <> mem.io
}
