package emulator

import chisel3._
import chisel3.util._


class DataMemoryReadPort(cfg: ModuleConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Output(UInt(cfg.num_bits.W))
}

class DataMemoryWritePort(cfg: ModuleConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Input(UInt(cfg.num_bits.W))
}

class DataMemory(cfg: ModuleConfig) extends Module {
  val io = IO(new Bundle {
    val rd = Vec(cfg.lut_inputs, new DataMemoryReadPort(cfg))
    val wr = new DataMemoryWritePort(cfg)
  })

  val mem = Seq.fill(cfg.max_steps)(Reg(UInt(cfg.num_bits.W)))

  // Write
  for (i <- 0 until cfg.max_steps) {
    when (i.U === io.wr.idx) {
      mem(i) := io.wr.bit
    }
  }

  // Read
  for (i <- 0 until cfg.lut_inputs) {
    io.rd(i).idx := DontCare
  }

  for (i <- 0 until cfg.lut_inputs) {
    for (j <- 0 until cfg.max_steps) {
      when (io.rd(i).idx === j.U) {
        io.rd(i).bit := mem(j)
      }
    }
  }
}
