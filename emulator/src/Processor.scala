package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled
import chisel3.experimental.hierarchy.{instantiable, public}

class ProcessorDebugBundle(cfg: EmulatorConfig) extends Bundle {
  val ldm = UInt(cfg.dmem_bits.W)
  val sdm = UInt(cfg.dmem_bits.W)
  val ops = Vec(cfg.lut_inputs, UInt(cfg.num_bits.W))
}

class ProcInstInitBundle(cfg: EmulatorConfig) extends Bundle {
  val inst = Flipped(Decoupled(Instruction(cfg)))
}

class ProcessorBundle(cfg: EmulatorConfig) extends Bundle {
  import cfg._
  val run  = Input(Bool())
  val host_steps  = Input(UInt(index_bits.W))

  val inst = Flipped(Decoupled(Instruction(cfg)))

  val sw_loc = new LocalSwitchPort(cfg)
  val sw_glb = new GlobalSwitchPort(cfg)

  val io_i = Input (UInt(num_bits.W))
  val io_o = Output(UInt(num_bits.W))

  val sram_port = Flipped(new PerProcessorSRAMBundle(cfg))

  val dbg = if (cfg.debug) Some(Output(new ProcessorDebugBundle(cfg))) else None
}

@instantiable
class Processor(cfg: EmulatorConfig) extends Module {
  import cfg._

  @public val io = IO(new ProcessorBundle(cfg))

  val io_o = RegInit(0.U(num_bits.W))
  io.io_o := io_o

  val pc = RegInit(0.U(index_bits.W))

  val imem = Module(new InstMem(cfg))
  imem.io.wen := false.B
  imem.io.winst := io.isc.inst_i.bits

  val ldm = Module(new DataMemory(cfg))
  val sdm = Module(new DataMemory(cfg))

  val init = RegInit(false.B)
  io.isc.inst.ready := !init

  when (!init) {
    when (io.isc.inst.fire()) {
      imem.io.wen := true.B
      when (pc === io.host_steps - 1.U) {
        pc := 0.U
        init := true.B
      } .otherwise {
        pc := pc + 1.U
      }
    }
  } .otherwise {
    when (io.run) {
      pc := Mux(pc === io.host_steps - 1.U, 0.U, pc + 1.U)
    }
  }

  // -------------------------- Fetch -----------------------------------
  imem.io.pc := pc

  // -------------------------- Decode -----------------------------------
  val fd_inst = imem.io.rinst
  dontTouch(fd_inst)

  for (i <- 0 until cfg.lut_inputs) {
    ldm.io.rd(i).idx := fd_inst.ops(i).rs
    sdm.io.rd(i).idx := fd_inst.ops(i).rs
  }

  // -------------------------- Execute -----------------------------------
  val de_inst = if (cfg.dmem_rd_lat == 1) {
    RegNext(fd_inst)
  } else {
    fd_inst
  }
  dontTouch(de_inst)


  val ops = Seq.fill(cfg.lut_inputs)(Wire(UInt(num_bits.W)))
  val lut_idx = Cat(ops.reverse)
  for (i <- 0 until cfg.lut_inputs) {
    ops(i) := Mux(de_inst.ops(i).local, ldm.io.rd(i).bit, sdm.io.rd(i).bit)
  }

  val f_out = Wire(UInt(num_bits.W))
  f_out := 0.U
  switch (de_inst.opcode) {
    is (Instruction.NOP.U) {
      f_out := 0.U
    }
    is (Instruction.Input.U) {
      f_out := io.io_i
    }
    is (Instruction.Lut.U) {
      f_out := de_inst.lut >> (lut_idx * num_bits.U)
    }
    is (Instruction.Output.U) {
      when (init) {
        io_o := ops(0)
      }
      f_out := ops(0)
    }
    is (Instruction.Gate.U) {
      f_out := ops(0)
    }
    is (Instruction.Latch.U) {
      f_out := ops(0)
    }
    is (Instruction.SRAMOut.U) {
      f_out := io.sram_port.op
    }
    is (Instruction.SRAMIn.U) {
      f_out := ops(0)
    }
  }

  val dmem_wr_en  = (pc >= cfg.fetch_decode_lat.U) && io.run
  val dmem_wr_idx = (pc -  cfg.fetch_decode_lat.U)

  ldm.io.wr.en  := dmem_wr_en
  ldm.io.wr.idx := dmem_wr_idx
  ldm.io.wr.bit := Mux(io.run, f_out, 0.U)

  val s_fwd = Reg(UInt(num_bits.W))
  val s_in = Mux(de_inst.sinfo.local, io.sw_loc.ip, io.sw_glb.ip)
  sdm.io.wr.en  := dmem_wr_en
  sdm.io.wr.idx := dmem_wr_idx
  sdm.io.wr.bit := Mux(io.run, s_in, 0.U)
  s_fwd := s_in

  val s_out = Mux(io.run && de_inst.sinfo.fwd, s_fwd, f_out)
  io.sw_loc.id := de_inst.sinfo.idx
  io.sw_loc.op := s_out
  io.sw_glb.op := s_out

  io.sram_port.ip    := f_out
  io.sram_port.valid := de_inst.mem
  io.sram_port.idx   := Cat(de_inst.ops.map(_.rs).tail.reverse)

  if (cfg.debug) {
    io.dbg.map(x => x.ldm := ldm.io.dbg.get)
    io.dbg.map(x => x.sdm := sdm.io.dbg.get)
    io.dbg.map(x => x.ops := ops)
  }

  when (!init && io.isc.init_i) {
    ldm.io.wr.en  := true.B
    ldm.io.wr.idx := pc
    ldm.io.wr.bit := 0.U

    sdm.io.wr.en  := true.B
    sdm.io.wr.idx := pc
    sdm.io.wr.bit := 0.U
  }
}
