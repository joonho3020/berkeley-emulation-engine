package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled

class ProcessorConfigBundle(cfg: ModuleConfig) extends Bundle {
  val host_steps = UInt(cfg.index_bits.W)
}

class ProcessorBundle(cfg: ModuleConfig) extends Bundle {
  import cfg._
  val run  = Input(Bool())
  val host_steps  = Input(UInt(index_bits.W))

  val init_o = Output(Bool())
  val inst_i = Flipped(Decoupled(Instruction(cfg)))
  val init_i = Input(Bool())
  val inst_o = Decoupled(Instruction(cfg))

  val swp  = new SwitchPort(cfg)

  val io_i = Input (UInt(num_bits.W))
  val io_o = Output(UInt(num_bits.W))
}

class Processor(cfg: ModuleConfig) extends Module {
  import cfg._

  val io = IO(new ProcessorBundle(cfg))
  io.io_o := DontCare

  val pc = RegInit(0.U(index_bits.W))
  val init = RegInit(false.B)
  io.init_o := init

  val imem = Module(new InstructionMemory(cfg))
  imem.io.pc := pc
  imem.io.wen := false.B
  imem.io.winst := io.inst_i.bits

  io.inst_o.valid := false.B
  io.inst_o.bits  := DontCare
  io.inst_i.ready := false.B

  when (!init) {
    when (!io.init_i) {
      io.inst_o <> io.inst_i
    } .otherwise {
      io.inst_i.ready := true.B
      when (io.inst_i.valid) {
        when (pc === io.host_steps - 1.U) {
          pc := 0.U
          init := true.B
        } .otherwise {
          pc := pc + 1.U
        }
        imem.io.wen := true.B
      }
    }

  } .otherwise {
    when (io.run) {
      pc := Mux(pc === io.host_steps - 1.U, 0.U, pc + 1.U)
    }
  }

  val inst = imem.io.rinst
  val ldm = Module(new DataMemory(cfg))
  val sdm = Module(new DataMemory(cfg))

  for (i <- 0 until cfg.lut_inputs) {
    ldm.io.rd(i).idx := inst.ops(i).rs
    sdm.io.rd(i).idx := inst.ops(i).rs
  }

  val ops = Seq.fill(cfg.lut_inputs)(Wire(UInt(num_bits.W)))
  val lut_idx = Cat(ops.reverse)
  for (i <- 0 until cfg.lut_inputs) {
    ops(i) := Mux(inst.ops(i).local, ldm.io.rd(i).bit, sdm.io.rd(i).bit)
  }

  val fout = Wire(UInt(num_bits.W))
  fout := DontCare
  switch (inst.opcode) {
    is (Instruction.NOP.U) {
      fout := 0.U
    }
    is (Instruction.Input.U) {
      fout := io.io_i
    }
    is (Instruction.Lut.U) {
      fout := inst.lut >> (lut_idx * num_bits.U)
    }
    is (Instruction.Output.U) {
      io.io_o := ops(0)
      fout := ops(0)
    }
    is (Instruction.Gate.U) {
      fout := ops(0)
    }
    is (Instruction.Latch.U) {
      fout := ops(0)
    }
  }

  sdm.io.wr.idx := pc
  sdm.io.wr.bit := io.swp.i

  ldm.io.wr.idx := pc
  ldm.io.wr.bit := fout

  io.swp.o := fout
  io.swp.id := inst.sin
}
