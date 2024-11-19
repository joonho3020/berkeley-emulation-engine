package emulator

import chisel3._
import chisel3.util._

class DataMemoryReadPort(cfg: EmulatorConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Output(UInt(cfg.num_bits.W))
}

class DataMemoryWritePort(cfg: EmulatorConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Input(UInt(cfg.num_bits.W))
  val en  = Input(Bool())
}

class DataMemory(cfg: EmulatorConfig) extends Module {
  import cfg._
  val io = IO(new Bundle {
    val rd = Vec(lut_inputs, new DataMemoryReadPort(cfg))
    val wr = new DataMemoryWritePort(cfg)
    val dbg = if (cfg.debug) Some(Output(UInt(dmem_bits.W))) else None
  })

  require(num_bits == 1)

  val mem = RegInit(0.U(dmem_bits.W))
  if (cfg.debug) {
    io.dbg.map(_ := mem)
  }

  // Write
  when (io.wr.en) {
    val woh = 1.U << io.wr.idx
    val wmask = ~woh
    mem := (mem & wmask) | (woh & (io.wr.bit << io.wr.idx))
  }

  // Read
  for (i <- 0 until lut_inputs) {
    io.rd(i).bit := mem >> io.rd(i).idx
  }
}
