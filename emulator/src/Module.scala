package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled
import chisel3.experimental.hierarchy._

class EModuleConfigBundle(cfg: EmulatorConfig) extends Bundle {
  val host_steps  = UInt(cfg.index_bits.W)
  val sram        = new SRAMProcessorConfigBundle(cfg)
}

class EModuleIOBitsBundle(cfg: EmulatorConfig) extends Bundle {
  val i = Vec(cfg.num_procs, Input (UInt(cfg.num_bits.W)))
  val o = Vec(cfg.num_procs, Output(UInt(cfg.num_bits.W)))
}

class EModuleDebugBundle(cfg: EmulatorConfig) extends Bundle {
  val pdbg = Vec(cfg.num_procs, new ProcessorDebugBundle(cfg))
}

class ModuleInstInitBundle(cfg: EmulatorConfig) extends Bundle {
  val inst = Instruction(cfg)
  val pidx = UInt(log2Ceil(cfg.num_procs).W)
}

class EModuleBundle(cfg: EmulatorConfig) extends Bundle {
  val cfg_in = Input(new EModuleConfigBundle(cfg))
  val init = Output(Bool())
  val inst = Flipped(Decoupled(new ModuleInstInitBundle(cfg)))

  val run  = Input(Bool())
  val io   = new EModuleIOBitsBundle(cfg)
  val sw_glb = Vec(cfg.num_procs, new GlobalSwitchPort(cfg))

  val dbg = if (cfg.debug) Some(new EModuleDebugBundle(cfg)) else None
  val dbg_proc_0_init = Output(Bool())
  val dbg_proc_n_init = Output(Bool())
  val dbg_proc_init_cnt = Output(UInt(log2Ceil(cfg.num_procs + 1).W))

  val pcs_are_zero = Output(Bool())
}

@instantiable
class EModule(cfg: EmulatorConfig) extends Module {
  import cfg._

  @public val io = IO(new EModuleBundle(cfg))

  val host_steps = Reg(UInt(index_bits.W))
  host_steps := io.cfg_in.host_steps

  val pdef = Definition(new Processor(cfg))
  val procs: Seq[Instance[Processor]] = Seq.fill(num_procs)(Instance(pdef))

  val sdef = Definition(new SRAMProcessor(cfg))
  val sram_proc = Instance(sdef)

  val local_switch = Module(new LocalSwitch(cfg))
  for (i <- 0 until num_procs) {
    local_switch.io.ports(i) <> procs(i).io.sw_loc
  }

  for (i <- 0 until num_procs) {
    procs(i).io.io_i   := io.io.i(i)
    procs(i).io.run    := io.run
    procs(i).io.host_steps := host_steps
    io.io.o(i) := procs(i).io.io_o
  }

  for (i <- 0 until num_procs) {
    io.sw_glb(i) <> procs(i).io.sw_glb
  }

  val inst_q = Module(new Queue(new ModuleInstInitBundle(cfg), 2))
  inst_q.io.enq <> io.inst

  for (i <- 0 until num_procs) {
    procs(i).io.inst.bits  := inst_q.io.deq.bits.inst
    procs(i).io.inst.valid := inst_q.io.deq.valid && (inst_q.io.deq.bits.pidx === i.U)
  }

  inst_q.io.deq.ready := procs.zipWithIndex.map({ case(proc, i) => {
    proc.io.inst.ready && (inst_q.io.deq.bits.pidx === i.U)
  }}).reduce(_ || _)

  val procs_init = procs.map { p => {
    !p.io.inst.ready
  }}.reduce(_ && _)

  io.init := procs_init &&  sram_proc.io.init

  sram_proc.io.run := io.run
  sram_proc.io.host_steps := host_steps
  sram_proc.io.cfg_in := io.cfg_in.sram

  for (i <- 0 until num_procs) {
    sram_proc.io.ports(i) <> procs(i).io.sram_port
  }

  io.dbg.map(_.pdbg.zipWithIndex.map { case(dbg, i) => {
    dbg := procs(i).io.dbg.get
  }})

  io.dbg_proc_0_init := !procs(0).io.inst.ready
  io.dbg_proc_n_init := !procs(cfg.num_procs-1).io.inst.ready
  io.dbg_proc_init_cnt := RegNext(procs.map{ p => (!p.io.inst.ready).asUInt }.reduce(_ +& _))
  io.pcs_are_zero := procs.map(_.io.pc_is_zero).reduce(_ && _)
}
