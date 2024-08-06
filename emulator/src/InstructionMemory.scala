package emulator

import chisel3._
import chisel3.util._

class Operand(cfg: ModuleConfig) extends Bundle {
  val rs = UInt(cfg.index_bits.W)
  val local = Bool()
}

class Instruction(cfg: ModuleConfig) extends Bundle {
  val opcode = UInt(cfg.opcode_bits.W)
  val lut    = UInt(cfg.lut_bits.W)
  val ops    = Vec(cfg.lut_inputs, new Operand(cfg))
  val sin    = UInt(cfg.switch_bits.W)
}

object Instruction {
  def apply(cfg: ModuleConfig): Instruction = {
    new Instruction(cfg)
  }

  val NOP    = 0
  val Input  = 1
  val Output = 2
  val Lut    = 3
  val Gate   = 4
  val Latch  = 5
}

class InstructionMemory(cfg: ModuleConfig) extends Module {
  import cfg._

  val io = IO(new Bundle {
    val pc    = Input(UInt(index_bits.W))
    val wen   = Input(Bool())
    val winst = Input (new Instruction(cfg))
    val rinst = Output(new Instruction(cfg))
  })

  io.rinst := DontCare

  val mem = Seq.fill(cfg.max_steps)(Reg(Instruction(cfg)))
  for (i <- 0 until max_steps) {
    when (i.U === io.pc) {
      when (io.wen) {
        mem(i) := io.winst
      } .otherwise {
        io.rinst := mem(i)
      }
    }
  }
}
