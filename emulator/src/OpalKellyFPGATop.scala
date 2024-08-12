package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled

case class OpalKellyConfig(
  wire_bits: Int = 16
)


class xpm_cdc_single(
  sync_stages: Int, src_input_reg: Int
) extends BlackBox(
    Map("DEST_SYNC_FF" -> sync_stages, "SRC_INPUT_REG" -> src_input_reg)
) {
  val io = IO(new Bundle {
    val src_clk =  Input(Clock())
    val src_in  =  Input(UInt(1.W))
    val dest_clk = Input(Clock())
    val dest_out = Output(UInt(1.W))
  })
}

class xpm_cdc_handshake(
  external: Int,
  dest_sync_ff: Int,
  init_sync_ff: Int,
  sim_assert_chk: Int,
  src_sync_ff: Int,
  width: Int
) extends BlackBox(
  Map(
    "DEST_EXT_HSK" -> external,
    "DEST_SYNC_FF" -> dest_sync_ff,
    "INIT_SYNC_FF" -> init_sync_ff,
    "SIM_ASSERT_CHK" -> sim_assert_chk,
    "SRC_SYNC_FF" -> src_sync_ff,
    "WIDTH"         -> width)
) {
  val io = IO(new Bundle {
    val src_in = Input(UInt(width.W))
    val src_send = Input(Bool())
    val src_clk =  Input(Clock())
    val dest_ack = Input(Bool())
    val dest_clk = Input(Clock())
    val dest_out = Output(UInt(width.W))
    val src_rcv = Output(Bool())
    val dest_req = Output(Bool())
  })
}

class DecoupledClockCrossingModule(wire_ins: Int, wire_bits: Int) extends Module {
  val io = IO(new Bundle {
    val in_clock = Input(Clock())
    val in_reset = Input(Bool())
    val in  = Flipped(Decoupled(Vec(wire_ins, UInt(wire_bits.W))))
    val out =         Decoupled(Vec(wire_ins, UInt(wire_bits.W)))
  })
  val handshake_cdc = Module(new xpm_cdc_handshake(1, 4, 1, 0, 4, wire_ins * wire_bits))
  handshake_cdc.io.src_clk  := io.in_clock
  handshake_cdc.io.dest_clk := clock
  handshake_cdc.io.dest_ack := handshake_cdc.io.dest_req
  handshake_cdc.io.src_in   := Cat(io.in.bits.reverse)

  withClockAndReset (io.in_clock, io.in_reset) {
    val handshake_done = RegInit(false.B)
    val prev_in_valid = RegNext(io.in.valid)
    handshake_cdc.io.src_send := io.in.valid && !handshake_done
    when (prev_in_valid && !io.in.valid && handshake_done) {
      handshake_done := false.B
    } .elsewhen (handshake_cdc.io.src_rcv) {
      handshake_done := true.B
    }
  }
  val src_rcv_cdc = Module(new xpm_cdc_single(4, 0))
  src_rcv_cdc.io.src_clk := io.in_clock
  src_rcv_cdc.io.dest_clk := clock
  src_rcv_cdc.io.src_in := handshake_cdc.io.src_rcv

  val src_rcv_prev = RegNext(src_rcv_cdc.io.dest_out)
  val src_rcv_pulse = !src_rcv_prev && src_rcv_cdc.io.dest_out.asBool
  io.out.valid := src_rcv_pulse
  for (i <- 0 until wire_ins) {
    val start = i * wire_bits
    io.out.bits(i) := handshake_cdc.io.dest_out(start + wire_bits - 1, start)
  }

  val ready_cdc = Module(new xpm_cdc_single(4, 0))
  ready_cdc.io.src_clk := clock
  ready_cdc.io.dest_clk := io.in_clock
  ready_cdc.io.src_in := io.out.ready
  io.in.ready := ready_cdc.io.dest_out
}

class OpalKellyClockCrossingModule(cfg: ModuleConfig, fpga_cfg: OpalKellyConfig) extends Module {
  import cfg._
  assert(num_bits == 1)

  val wire_bits = fpga_cfg.wire_bits
  val insn_bits = Instruction(cfg).getWidth
  val wire_ins_per_insn = (insn_bits.toFloat / wire_bits.toFloat).ceil.toInt
  val wire_ins_per_io =   (module_sz.toFloat / wire_bits.toFloat).ceil.toInt

  val io = IO(new Bundle {
    val host_clock = Input(Clock())
    val host_reset = Input(Bool())
    val host_host_steps = Input(UInt(wire_bits.W))
    val host_used_procs = Input(UInt(switch_bits.W))
    val host_insns = Flipped(Decoupled(Vec(wire_ins_per_insn, UInt(wire_bits.W))))
    val host_io_i  = Flipped(Decoupled(Vec(wire_ins_per_io,   UInt(wire_bits.W))))
    val host_io_o  =         Decoupled(Vec(wire_ins_per_io,   UInt(wire_bits.W)))

    val fpga_host_steps = Output(UInt(wire_bits.W))
    val fpga_used_procs = Output(UInt(switch_bits.W))
    val fpga_insns =         Decoupled(Vec(wire_ins_per_insn, UInt(wire_bits.W)))
    val fpga_io_i  =         Decoupled(Vec(wire_ins_per_io,   UInt(wire_bits.W)))
    val fpga_io_o  = Flipped(Decoupled(Vec(wire_ins_per_io,   UInt(wire_bits.W))))
  })

  if (true) {
    val host_steps_cdc = Module(new xpm_cdc_handshake(1, 4, 1, 0, 4, wire_bits))
    host_steps_cdc.io.dest_ack := host_steps_cdc.io.dest_req
    host_steps_cdc.io.src_clk := io.host_clock
    host_steps_cdc.io.dest_clk := clock
    host_steps_cdc.io.src_in := io.host_host_steps
    io.fpga_host_steps := host_steps_cdc.io.dest_out

    withClockAndReset (io.host_clock, io.host_reset) {
      val sent = RegInit(false.B)
      host_steps_cdc.io.src_send := !sent && (io.host_host_steps =/= 0.U)
      when (host_steps_cdc.io.src_rcv) {
        sent := true.B
      }
    }

    val used_procs_cdc = Module(new xpm_cdc_handshake(1, 4, 1, 0, 4, wire_bits))
    used_procs_cdc.io.dest_ack := used_procs_cdc.io.dest_req
    used_procs_cdc.io.src_clk := io.host_clock
    used_procs_cdc.io.dest_clk := clock
    used_procs_cdc.io.src_in := io.host_used_procs
    io.fpga_used_procs := used_procs_cdc.io.dest_out

    withClockAndReset (io.host_clock, io.host_reset) {
      val sent = RegInit(false.B)
      used_procs_cdc.io.src_send := !sent && (io.host_used_procs =/= 0.U)
      when (used_procs_cdc.io.src_rcv) {
        sent := true.B
      }
    }

    val insns_cdc = Module(new DecoupledClockCrossingModule(wire_ins_per_insn, wire_bits))
    insns_cdc.io.in_clock := io.host_clock
    insns_cdc.io.in_reset := io.host_reset
    insns_cdc.io.in <> io.host_insns
    io.fpga_insns <> insns_cdc.io.out

    val input_cdc = Module(new DecoupledClockCrossingModule(wire_ins_per_io, wire_bits))
    input_cdc.io.in_clock := io.host_clock
    input_cdc.io.in_reset := io.host_reset
    input_cdc.io.in <> io.host_io_i
    io.fpga_io_i <> input_cdc.io.out

    withClockAndReset (io.host_clock, io.host_reset) {
      val output_cdc = Module(new DecoupledClockCrossingModule(wire_ins_per_io, wire_bits))
      output_cdc.io.in_clock := clock
      output_cdc.io.in_reset := reset
      output_cdc.io.in <> io.fpga_io_o
      io.host_io_o <> output_cdc.io.out
    }
  } else {
    io.fpga_host_steps := io.host_host_steps
    io.fpga_used_procs := io.host_used_procs
    io.fpga_insns <> io.host_insns
    io.fpga_io_i  <> io.host_io_i
    io.host_io_o  <> io.fpga_io_o
  }
}

class OpalKellyEmulatorModuleWrapper(cfg: ModuleConfig, fpga_cfg: OpalKellyConfig) extends Module {
  import cfg._
  assert(num_bits == 1)

  val wire_bits = fpga_cfg.wire_bits
  val insn_bits = Instruction(cfg).getWidth
  val wire_ins_per_insn = (insn_bits.toFloat / wire_bits.toFloat).ceil.toInt
  val wire_ins_per_io =   (module_sz.toFloat / wire_bits.toFloat).ceil.toInt

  val io = IO(new Bundle {
    val host_steps = Input(UInt(wire_bits.W))
    val used_procs = Input(UInt(switch_bits.W))
    val insns  = Flipped(Decoupled(Vec(wire_ins_per_insn, Input(UInt(wire_bits.W)))))

    val io_i = Flipped(Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W))))
    val io_o =         Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W)))

    val i_q_bits_fired = Output(UInt(wire_bits.W))
    val o_q_bits_fired = Output(UInt(wire_bits.W))
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
  val io_o_prev = RegNext(io.io_o.ready)
  val io_o_pulse = !io_o_prev && io.io_o.ready
  io.io_o.valid := io_o_q.io.deq.valid
  io.io_o.bits  := io_o_q.io.deq.bits
  io_o_q.io.deq.ready := io_o_pulse

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

  val i_q_bits_fired = RegInit(0.U(wire_bits.W))
  when (io_i_q.io.deq.fire) {
    i_q_bits_fired := io_i_q.io.deq.bits(0)
  }
  io.i_q_bits_fired := i_q_bits_fired

  val o_q_bits_fired = RegInit(0.U(wire_bits.W))
  when (io_o_q.io.enq.fire) {
    o_q_bits_fired := io_o_q.io.enq.bits(0)
  }
  io.o_q_bits_fired := o_q_bits_fired
}


class OpalKellyFPGATop(cfg: ModuleConfig, fpga_cfg: OpalKellyConfig) extends Module {
  import cfg._

  val wire_bits = fpga_cfg.wire_bits
  val insn_bits = Instruction(cfg).getWidth
  val wire_ins_per_insn = (insn_bits.toFloat / wire_bits.toFloat).ceil.toInt
  val wire_ins_per_io =   (module_sz.toFloat / wire_bits.toFloat).ceil.toInt

  println("----- Emulator Harness ----------------")
  println(f"Instruction bits: ${insn_bits}")
  println(f"wire_ins_per_insn: ${wire_ins_per_insn}")
  println(f"wire_ins_per_io: ${wire_ins_per_io}")

  val io = IO(new Bundle {
    val host_clock = Input(Clock())
    val host_reset = Input(Bool())
    val host_host_steps = Input(UInt(wire_bits.W))
    val host_used_procs = Input(UInt(switch_bits.W))
    val host_insns  = Flipped(Decoupled(Vec(wire_ins_per_insn, Input(UInt(wire_bits.W)))))

    val host_io_i = Flipped(Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W))))
    val host_io_o =         Decoupled(Vec(wire_ins_per_io, UInt(wire_bits.W)))

    val i_q_bits_fired = Output(UInt(wire_bits.W))
    val o_q_bits_fired = Output(UInt(wire_bits.W))
    val fpga_io_o_ready = Output(Bool())
    val host_io_o_ready = Output(Bool())
  })

  val clock_crossing = Module(new OpalKellyClockCrossingModule(cfg, fpga_cfg))
  clock_crossing.io.host_clock := io.host_clock
  clock_crossing.io.host_reset := io.host_reset
  clock_crossing.io.host_host_steps := io.host_host_steps
  clock_crossing.io.host_used_procs := io.host_used_procs
  clock_crossing.io.host_insns <> io.host_insns
  clock_crossing.io.host_io_i  <> io.host_io_i
  io.host_io_o <> clock_crossing.io.host_io_o

  val emulation_module_wrapper = Module(new OpalKellyEmulatorModuleWrapper(cfg, fpga_cfg))
  emulation_module_wrapper.io.host_steps := clock_crossing.io.fpga_host_steps
  emulation_module_wrapper.io.used_procs := clock_crossing.io.fpga_used_procs
  emulation_module_wrapper.io.insns <> clock_crossing.io.fpga_insns
  emulation_module_wrapper.io.io_i <> clock_crossing.io.fpga_io_i
  clock_crossing.io.fpga_io_o <> emulation_module_wrapper.io.io_o

  io.i_q_bits_fired := emulation_module_wrapper.io.i_q_bits_fired
  io.o_q_bits_fired := emulation_module_wrapper.io.o_q_bits_fired
  io.fpga_io_o_ready := clock_crossing.io.fpga_io_o.ready
  io.host_io_o_ready := io.host_io_o.ready
}
