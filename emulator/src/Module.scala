package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled

case class ModuleConfig(
  max_steps: Int = 8, // Maximum host steps that can be run
  num_bits:  Int = 1,   // Width of the datapath
  module_sz: Int = 8,  // Number of processor in a module
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

case class OpalKellyConfig(
  wire_bits: Int = 16
)

class OpalKellyEmulatorModuleTop(cfg: ModuleConfig, fpga_cfg: OpalKellyConfig) extends Module {
  import cfg._
  assert(num_bits == 1)

  val wire_bits = fpga_cfg.wire_bits
  val insn_bits = Instruction(cfg).getWidth
  val wire_ins_per_insn = (insn_bits.toFloat / wire_bits.toFloat).ceil.toInt
  val wire_ins_per_io =   (module_sz.toFloat / wire_bits.toFloat).ceil.toInt

  println("----- Emulator Harness ----------------")
  println(f"Instruction bits: ${insn_bits}")
  println(f"wire_ins_per_insn: ${wire_ins_per_insn}")
  println(f"wire_ins_per_io: ${wire_ins_per_io}")

  val io = IO(new Bundle {
    val host_steps = Input(UInt(wire_bits.W))
    val used_procs = Input(UInt(switch_bits.W))
    val insns  = Flipped(Decoupled(Vec(wire_ins_per_insn, Input(UInt(wire_bits.W)))))

    val io_i = Flipped(Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W))))
    val io_o =         Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W)))
  })

  val module = Module(new EmulatorModule(cfg))
  module.io.cfg_in.host_steps := io.host_steps
  module.io.cfg_in.used_procs := io.used_procs

  val insns_q = Module(new Queue(Vec(wire_ins_per_insn, UInt(wire_bits.W)), 2))
  module.io.inst.valid := insns_q.io.deq.valid
  insns_q.io.deq.ready := module.io.inst.ready

  val insns_q_bits = Cat(insns_q.io.deq.bits.reverse)
  val op_start_bit = opcode_bits + lut_bits
  val sin_start_bits = op_start_bit + (1 + index_bits) * lut_inputs
  module.io.inst.bits.opcode := insns_q_bits(opcode_bits-1, 0)
  module.io.inst.bits.lut    := insns_q_bits(opcode_bits+lut_bits-1, opcode_bits)
  for (i <- 0 until lut_inputs) {
    val start = op_start_bit + (1 + index_bits) * i
    module.io.inst.bits.ops(i).rs    := insns_q_bits(index_bits+start-1, start)
    module.io.inst.bits.ops(i).local := insns_q_bits(index_bits+start)
  }
  module.io.inst.bits.sin    := insns_q_bits(switch_bits-1+sin_start_bits, sin_start_bits)

  val insns_val_prev = RegNext(io.insns.valid)
  val insns_val_pulse = !insns_val_prev && io.insns.valid
  insns_q.io.enq.valid := insns_val_pulse
  insns_q.io.enq.bits  := io.insns.bits
  io.insns.ready := insns_q.io.enq.ready

  val io_i_prev = RegNext(io.io_i.valid)
  val io_i_pulse = !io_i_prev && io.io_i.valid

  val io_i_q = Module(new Queue(Vec(wire_ins_per_io, UInt(wire_bits.W)), 2))
  io_i_q.io.enq.valid := io_i_pulse
  io_i_q.io.enq.bits  := io.io_i.bits
  io.io_i.ready := io_i_q.io.enq.ready

  val io_o_q = Module(new Queue(Vec(wire_ins_per_io, UInt(wire_bits.W)), 2))
  io.io_o <> io_o_q.io.deq

  val step = RegInit(0.U(index_bits.W))

  module.io.run := false.B
  io_i_q.io.deq.ready := false.B
  io_o_q.io.enq.valid := false.B

  when (io_i_q.io.deq.valid && io_o_q.io.enq.ready && module.io.init) {
    step := Mux(step === io.host_steps - 1.U, 0.U, step + 1.U)
    when (step === io.host_steps - 1.U) {
      step := 0.U
      io_i_q.io.deq.ready := true.B
      io_o_q.io.enq.valid := true.B
    } .otherwise {
      step := step + 1.U
    }
    module.io.run := true.B
  }

  // set input bits
  for (i <- 0 until module_sz) {
    module.io.i_bits(i) := Cat(io_i_q.io.deq.bits.reverse) >> (i * num_bits)
  }

  // set output bits
  for (i <- 0 until wire_ins_per_io) {
    io_o_q.io.enq.bits(i) := Cat(module.io.o_bits.reverse) >> (i * wire_bits)
  }
}
