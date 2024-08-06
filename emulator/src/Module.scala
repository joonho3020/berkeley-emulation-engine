package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled

case class ModuleConfig(
  max_steps: Int = 128, // Maximum host steps that can be run
  num_bits:  Int = 1,   // Width of the datapath
  module_sz: Int = 64,  // Number of processor in a module
  num_prims: Int = 6,   // Number of primitives
  lut_inputs: Int = 3,  // Number of lut inputs
  ireg_skip: Int  = 4,  // Insert queues for instruction scan chain every ireg_skip processors
) {
  val index_bits = log2Ceil(max_steps)
  val switch_bits = log2Ceil(module_sz)
  val opcode_bits = log2Ceil(num_prims)
  val lut_bits    = 1 << lut_inputs
}

class ModuleBundle(cfg: ModuleConfig) extends Bundle {
  import cfg._
  val inst = Flipped(Decoupled(Instruction(cfg)))
  val run  = Input(Bool())
  val i_bits = Vec(module_sz, Input(UInt(num_bits.W)))
}

class EmulatorModule(cfg: ModuleConfig) extends Module {
  import cfg._

  val io = IO(new ModuleBundle(cfg))

  val procs = Seq.fill(module_sz)(Module(new Processor(cfg)))
  val switch = Module(new Switch(cfg))
  for (i <- 0 until module_sz) {
    switch.io.ports(i) <> procs(i).io.swp
  }

  for (i <- 0 until module_sz) {
    procs(i).io.io_i   := io.i_bits(i)
    procs(i).io.run    := io.run
    procs(i).io.config := DontCare
  }

  // instruction scan chain
  for (i <- 0 until module_sz - 1) {
    if (i % ireg_skip == ireg_skip - 1) {
      val q = Module(new Queue(Instruction(cfg), 1))
      q.io.enq <> procs(i+1).io.inst_o
      procs(i).io.inst_i <> q.io.deq
    } else {
      procs(i).io.inst_i <> procs(i+1).io.inst_o
    }
  }
  procs(0).io.inst_o.ready := false.B
  procs(module_sz-1).io.inst_i <> io.inst
}
