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

  val mem = RegInit(VecInit(Seq.fill(max_steps)(0.U(num_bits.W))))

  if (cfg.debug) {
    val dbg = Cat(mem.reverse)
    io.dbg.map(x => x := dbg)
  }

  // Write
  for (i <- 0 until max_steps) {
    when (i.U === io.wr.idx && io.wr.en) {
      mem(i) := io.wr.bit
    }
  }

  // Read
  for (i <- 0 until lut_inputs) {
    io.rd(i).bit := 0.U
  }
  for (i <- 0 until lut_inputs) {
    for (j <- 0 until max_steps) {
      when (io.rd(i).idx === j.U) {
        io.rd(i).bit := mem(j)
      }
    }
  }
}
