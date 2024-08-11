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
  val dmem_bits   = max_steps * num_bits
}

class EmulatorModuleConfigBundle(cfg: ModuleConfig) extends Bundle {
  import cfg._
  val host_steps  = UInt(index_bits.W)
  val used_procs  = UInt(index_bits.W)
}

class EmulatorModuleBundle(cfg: ModuleConfig) extends Bundle {
  import cfg._
  val cfg_in = Input(new EmulatorModuleConfigBundle(cfg))
  val run  = Input(Bool())
  val init = Output(Bool())
  val inst = Flipped(Decoupled(Instruction(cfg)))
  val i_bits = Vec(module_sz, Input (UInt(num_bits.W)))
  val o_bits = Vec(module_sz, Output(UInt(num_bits.W)))

  val dbg = Vec(module_sz, Output(new ProcessorDebugBundle(cfg)))
}

class EmulatorModule(cfg: ModuleConfig) extends Module {
  import cfg._

  val io = IO(new EmulatorModuleBundle(cfg))

  val procs = Seq.fill(module_sz)(Module(new Processor(cfg)))
  val switch = Module(new Switch(cfg))
  for (i <- 0 until module_sz) {
    switch.io.ports(i) <> procs(i).io.swp
  }

  for (i <- 0 until module_sz) {
    procs(i).io.io_i   := io.i_bits(i)
    procs(i).io.run    := io.run
    procs(i).io.host_steps := io.cfg_in.host_steps
    io.o_bits(i) := procs(i).io.io_o
  }

  for (i <- 0 until module_sz) {
    io.dbg(i) := procs(i).io.dbg
  }
  dontTouch(io.dbg)

  // instruction scan chain
  for (i <- 0 until module_sz - 1) {
    procs(i+1).io.init_i := procs(i).io.init_o
    if (i % ireg_skip == ireg_skip - 1) {
      val q = Module(new Queue(Instruction(cfg), 1))
      q.io.enq <> procs(i+1).io.inst_o
      procs(i).io.inst_i <> q.io.deq
    } else {
      procs(i).io.inst_i <> procs(i+1).io.inst_o
    }
  }
  procs(0).io.init_i := true.B
  procs(0).io.inst_o.ready := false.B
  procs(module_sz-1).io.inst_i <> io.inst

  io.init := procs.zipWithIndex.map { case(p, i) => {
    Mux(i.U < io.cfg_in.used_procs, p.io.init_o, true.B)
  }}.reduce(_ && _)
}
