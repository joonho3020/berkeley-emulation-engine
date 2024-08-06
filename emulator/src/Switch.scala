package emulator

import chisel3._
import chisel3.util._

class SwitchPort(cfg: ModuleConfig) extends Bundle {
  val id = Output(UInt(cfg.switch_bits.W)) // process consume id
  val o  = Output(UInt(cfg.num_bits.W))    // processor output bit
  val i  = Input (UInt(cfg.num_bits.W))    // processor consume bit
}

class Switch(cfg: ModuleConfig) extends Module {
  import cfg._

  val io = IO(new Bundle {
    val ports = Vec(cfg.module_sz, Flipped(new SwitchPort(cfg)))
  })

  val o_prev = Seq.fill(cfg.module_sz)(Reg(UInt(num_bits.W)))
  for (i <- 0 until module_sz) {
    o_prev(i) := io.ports(i).o
  }

  for (i <- 0 until module_sz) {
    io.ports(i).i := DontCare
  }

  // Xbar
  for (i <- 0 until module_sz) {
    for (j <- 0 until module_sz) {
      when (j.U === io.ports(i).id) {
        io.ports(i).i := o_prev(j)
      }
    }
  }
}
