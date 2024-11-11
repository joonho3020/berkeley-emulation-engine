package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental.hierarchy.{Definition, Instance}

class BoardDebugBundle(cfg: EmulatorConfig) extends Bundle {
  import cfg._
  val bdbg = Vec(num_mods, new EModuleDebugBundle(cfg))
}

class BoardBundle(cfg: EmulatorConfig) extends Bundle {
  import cfg._

  val cfg_in = Vec(num_mods, Input(new EModuleConfigBundle(cfg)))
  val init = Output(Bool())
  val insts = Vec(num_mods, Flipped(Decoupled(Instruction(cfg))))

  val run = Input(Bool())
  val io = Vec(num_mods, new EModuleIOBitsBundle(cfg))
  val dbg = if (cfg.debug) Some(new BoardDebugBundle(cfg)) else None
}

class Board(cfg: EmulatorConfig) extends Module {
  import cfg._
  val io = IO(new BoardBundle(cfg))

  val mdef = Definition(new EModule(cfg))
  val mods = Seq.fill(num_mods)(Instance(mdef))
  val global_switch = Module(new GlobalSwitch(cfg))
  for (i <- 0 until num_mods) {
    for (j <- 0 until num_procs) {
      global_switch.io.ports(i)(j) <> mods(i).io.sw_glb(j)
    }
    mods(i).io.cfg_in := io.cfg_in(i)
    mods(i).io.inst <> io.insts(i)
    mods(i).io.run := io.run
    mods(i).io.io <> io.io(i)
  }

  io.init := mods.map(_.io.init).reduce(_ && _)
  io.dbg.map(_.bdbg.zipWithIndex.map { case(dbg, i) => {
    dbg := mods(i).io.dbg.get
    dontTouch(dbg)
  }})
}
