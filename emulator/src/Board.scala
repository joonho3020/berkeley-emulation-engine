package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental.hierarchy.{Definition, Instance, Instantiate}

class BoardDebugBundle(cfg: EmulatorConfig) extends Bundle {
  import cfg._
  val bdbg = Vec(num_mods, new EModuleDebugBundle(cfg))
}

class BoardInstInitBundle(cfg: EmulatorConfig) extends Bundle {
  val inst = new ModuleInstInitBundle(cfg)
  val midx = UInt(log2Ceil(cfg.num_mods).W)
}

class BoardBundle(cfg: EmulatorConfig) extends Bundle {
  import cfg._

  val cfg_in = Vec(num_mods, Input(new EModuleConfigBundle(cfg)))
  val init = Output(Bool())
  val inst = Flipped(Decoupled(new BoardInstInitBundle(cfg)))

  val run = Input(Bool())
  val io = Vec(num_mods, new EModuleIOBitsBundle(cfg))
  val dbg = if (cfg.debug) Some(new BoardDebugBundle(cfg)) else None
  val dbg_proc_0_init = Output(UInt(cfg.num_mods.W))
  val dbg_proc_n_init = Output(UInt(cfg.num_mods.W))
}

class Board(cfg: EmulatorConfig) extends Module {
  import cfg._
  val io = IO(new BoardBundle(cfg))

// val mdef_small = Definition(new EModule(cfg, false))
// val mdef_large = Definition(new EModule(cfg, true))
  val mods = (0 until num_mods).map(i => {
    if (i < cfg.num_mods - cfg.large_sram_cnt) {
      Instantiate(new EModule(cfg, false))
    } else {
      Instantiate(new EModule(cfg, true))
    }
  }).toSeq

  val global_switch = Module(new GlobalSwitch(cfg))
  for (i <- 0 until num_mods) {
    for (j <- 0 until num_procs) {
      global_switch.io.ports(i)(j) <> mods(i).io.sw_glb(j)
    }
    mods(i).io.cfg_in := io.cfg_in(i)
    mods(i).io.run := io.run
    mods(i).io.io <> io.io(i)
  }

  val inst_q = Module(new Queue(new BoardInstInitBundle(cfg), 4))
  inst_q.io.enq <> io.inst

  for (i <- 0 until num_mods) {
    mods(i).io.inst.bits := inst_q.io.deq.bits.inst
    mods(i).io.inst.valid := inst_q.io.deq.valid && (inst_q.io.deq.bits.midx === i.U)
  }

  inst_q.io.deq.ready := mods.zipWithIndex.map({ case(mod, i) => {
    mod.io.inst.ready && (inst_q.io.deq.bits.midx === i.U)
  }}).reduce(_ || _)

  io.init := mods.map(_.io.init).reduce(_ && _)
  io.dbg.map(_.bdbg.zipWithIndex.map { case(dbg, i) => {
    dbg := mods(i).io.dbg.get
    dontTouch(dbg)
  }})

  io.dbg_proc_0_init := Cat(mods.map(_.io.dbg_proc_0_init).reverse)
  io.dbg_proc_n_init := Cat(mods.map(_.io.dbg_proc_n_init).reverse)
}
