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

class EModuleBundle(cfg: EmulatorConfig) extends Bundle {
  val cfg_in = Input(new EModuleConfigBundle(cfg))
  val init = Output(Bool())
  val inst = Flipped(Decoupled(Instruction(cfg)))

  val run  = Input(Bool())
  val io   = new EModuleIOBitsBundle(cfg)
  val sw_glb = Vec(cfg.num_procs, new GlobalSwitchPort(cfg))

  val dbg = if (cfg.debug) Some(new EModuleDebugBundle(cfg)) else None
}

@instantiable
class EModule(cfg: EmulatorConfig, large_sram: Boolean) extends Module {
  import cfg._

  @public val io = IO(new EModuleBundle(cfg))

  val host_steps = Reg(UInt(index_bits.W))
  host_steps := io.cfg_in.host_steps

  val pdef = Definition(new Processor(cfg))
  val procs: Seq[Instance[Processor]] = Seq.fill(num_procs)(Instance(pdef))

  val sdef = Definition(new SRAMProcessor(cfg, large_sram))
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

  // instruction scan chain
  for (i <- 0 until num_procs - 1) {
    procs(i+1).io.isc.init_i := procs(i).io.isc.init_o
    if (i % ireg_skip == ireg_skip - 1) {
      val q = Module(new Queue(Instruction(cfg), 1))
      q.io.enq <> procs(i+1).io.isc.inst_o
      procs(i).io.isc.inst_i <> q.io.deq
    } else {
      procs(i).io.isc.inst_i <> procs(i+1).io.isc.inst_o
    }
  }
  procs(0).io.isc.init_i := true.B
  procs(0).io.isc.inst_o.ready := false.B
  procs(num_procs-1).io.isc.inst_i <> io.inst

  val procs_init = procs.map { p => {
    p.io.isc.init_o
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
}
