package emulator

import chisel3._
import chisel3.util._

class GlobalSwitchPort(cfg: EmulatorConfig) extends Bundle {
  val op  = Output(UInt(cfg.num_bits.W))  // processor output bit
  val ip  = Input (UInt(cfg.num_bits.W))  // processor consume bit
}

class LocalSwitchPort(cfg: EmulatorConfig) extends Bundle {
  val id = Output(UInt(cfg.switch_bits.W)) // process consume id
  val op = Output(UInt(cfg.num_bits.W))    // processor output bit
  val ip = Input (UInt(cfg.num_bits.W))    // processor consume bit
}

class LocalSwitch(cfg: EmulatorConfig) extends Module {
  import cfg._

  val io = IO(new Bundle {
    val ports = Vec(cfg.num_procs, Flipped(new LocalSwitchPort(cfg)))
  })

  for (i <- 0 until num_procs) {
    io.ports(i).ip := DontCare
  }

// val pipelined_id = io.ports.map(x => ShiftRegister(x.id, cfg.inter_proc_nw_lat))
  val pipelined_op = io.ports.map(x => ShiftRegister(x.op, cfg.inter_proc_nw_lat))

  // Xbar
  for (i <- 0 until num_procs) {
    for (j <- 0 until num_procs) {
      when (j.U === io.ports(i).id) {
        io.ports(i).ip := pipelined_op(j)
      }
    }
  }
}

class GlobalSwitch(cfg: EmulatorConfig) extends Module {
  val io = IO(new Bundle {
    val ports = Vec(cfg.num_mods, Vec(cfg.num_procs, Flipped(new GlobalSwitchPort(cfg))))
  })

  val topo = cfg.global_network_topology
  for ((src, dst) <- topo) {
    io.ports(dst.mod)(dst.proc).ip := ShiftRegister(io.ports(src.mod)(src.proc).op, cfg.inter_mod_nw_lat)
  }
}
