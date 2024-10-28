package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled
import chisel3.experimental.hierarchy.{instantiable, public}

object SRAMInputTypes extends ChiselEnum {
  val SRAMRdEn, SRAMWrEn, SRAMRdAddr, SRAMWrAddr, SRAMWrData, SRAMWrMask, SRAMRdWrEn, SRAMRdWrMode, SRAMRdWrAddr = Value
}

class SRAMIndexDecoder(cfg: EmulatorConfig) extends Module {
  val io = IO(new Bundle {
    val idx = Input(UInt(cfg.sram_unique_indices_bits.W))
    val prim = Output(SRAMInputTypes())
    val offset = Output(UInt(cfg.sram_offset_decode_bits.W))
  })

  io.prim   := DontCare
  io.offset := DontCare

  when (io.idx >= cfg.sram_other_offset.U) {
    io.prim   := DontCare
    io.offset := DontCare
  } .elsewhen (io.idx >= cfg.sram_rdwr_addr_offset.U) {
    io.prim   := SRAMRdWrAddr
    io.offset := io.idx - cfg.sram_rdwr_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_rdwr_mode_offset.U) {
    io.prim   := SRAMRdWrMode
    io.offset := io.idx - cfg.sram_rdwr_mode_offset.U
  } .elsewhen (io.idx >= cfg.sram_rdwr_en_offset.U) {
    io.prim   := SRAMRdWrEn
    io.offset := io.idx - cfg.sram_rdwr_en_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_mask_offset.U) {
    io.prim   := SRAMWrMask
    io.offset := io.idx - cfg.sram_wr_mask_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_data_offset.U) {
    io.prim   := SRAMWrData
    io.offset := io.idx - cfg.sram_wr_data_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_addr_offset.U) {
    io.prim   := SRAMWrAddr
    io.offset := io.idx - cfg.sram_wr_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_rd_addr_offset.U) {
    io.prim   := SRAMRdAddr
    io.offset := io.idx - cfg.sram_rd_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_en_offset.U) {
    io.prim   := SRAMWrEn
    io.offset := io.idx - cfg.sram_wr_en_offset.U
  } .elsewhen (io.idx >= cfg.sram_rd_en_offset.U) {
    io.prim   := SRAMRdEn
    io.offset := io.idx - cfg.sram_rd_en_offset.U
  }
}

class SRAMMaskedWriteData(cfg: EmulatorConfig) extends Module {
  val io = IO(new Bundle {
    val wr_mask_bits = Input(UInt(cfg.sram_width_bits.W))
    val wr_mask      = Input(UInt(cfg.sram_width_bits.W))
    val wr_data    = Input(UInt(cfg.sram_width.W))
    val rd_data    = Input(UInt(cfg.sram_width.W))
    val masked_wr_data = Output(UInt(cfg.sram_width.W))
  })


  val wr_data_mask = Wire(UInt(cfg.sram_width.W))
  for (i <- 0 until cfg.sram_width) {
    // val num_bits_per_mask = Wire(UInt(cfg.sram_width_bits.W))
    // num_bits_per_mask := cfg.sram_width.U(cfg.sram_width_bits.W) / io.wmask_bits
    // wr_data_mask(i) := (io.wmask >> (i.U / num_bits_per_mask)) & 1.U
    // wr_data_mask(i) := (io.wmask >> (i.U * io.wmask_bits / cwf.sram_width.U)) & 1.U
    wr_data_mask(i) := (io.wr_mask >> (i.U * io.wr_mask_bits >> cwf.sram_width_bits.U)) & 1.U
  }

  when (io.wmask_bits === 0.U) {
    io.masked_wr_data := io.wr_data
  } .otherwise {
    io.masked_wr_data := (io.wr_data & wr_data_mask) | (io.rd_data & ~wr_data_mask)
  }
}

class PerProcessorSRAMBundle(cfg: EmulatorConfig) extends Bundle {
  val idx   = Input (UInt(cfg.sram_unique_indices_bits.W))
  val valid = Input (Bool())
  val ip    = Input (UInt(cfg.num_bits.W))
  val op    = Output(UInt(cfg.num_bits.W))
}

class SRAMProcessorConfigBundle(cfg: EmulatorConfig) extends Bundle {
  val single_port_ram = Input(Bool())
  val wmask_bits      = Input(UInt(cfg.sram_width_bits.W))
  val width_bits      = Input(UInt(cfg.sram_width_bits.W))
}

class SRAMProcessorBundle(cfg: EmulatorConfig) extends Bundle {
  val ports = Vec(cfg.num_procs, new PerProcessorSRAMBundle(cfg))
  val cfg   = new SRAMProcessorConfigBundle(cfg)
  val run   = Input(Bool())
  val host_steps = Input(UInt(index_bits.W))
  val init  = Output(Bool())
}

class SRAMInputs(cfg: EmulatorConfig) extends Bundle {
  val rd_en   = Bool()
  val wr_en   = Bool()
  val rd_addr = UInt(cfg.sram_addr_bits.W)
  val wr_addr = UInt(cfg.sram_addr_bits.W)
  val wr_data = UInt(cfg.sram_width.W)
  val wr_mask = UInt(cfg.sram_width.W)
}

@instantiable
class SRAMProcessor(cfg: EmulatorConfig) extends Module {
  import cfg._
  @public val io = IO(new SRAMProcessorBundle(cfg))

  val init = RegInit(false.B)
  val pc = RegInit(0.U(index_bits.W))
  val cur = RegInit(0.U(1.W))
  val inputs = Seq.fill(2)(Reg(new SRAMInputs(cfg)))
  val prev_input = Reg(new SRAMInputs(cfg))
  val sram = SyncReadMem(cfg.sram_entries, UInt(cfg.sram_width.W))

  io.init := init

  val cur_input = Mux(cur === 1.U, inputs(1), inputs(0))
  val rec_input = Mux(cur === 1.U, inputs(0), inputs(1))

  val decs = Seq.fill(num_procs)(Module(new SRAMIndexDecoder(cfg)))
  for (i <- 0 until num_procs) {
    decs(i).io.idx := io.ports(i).idx
  }

  for (i <- 0 until num_procs) {
    val ip_shift_offset = io.ports(i).ip << decs(i).io.offset
    when (io.ports(i).valid && io.run) {
      switch (decs(i).io.prim) {
        is (SRAMRdEn) {
          rec_input.rd_en := io.ports(i).ip.asBool
        }
        is (SRAMWrEn) {
          rec_input.wr_en := io.ports(i).ip.asBool
        }
        is (SRAMRdAddr) {
          rec_input.rd_addr := rec_input.rd_addr | ip_shift_offset
        }
        is (SRAMWrAddr) {
          rec_input.wr_addr := rec_input.wr_addr | ip_shift_offset
        }
        is (SRAMWrData) {
          rec_input.wr_data := rec_input.wr_data | ip_shift_offset
        }
        is (SRAMWrMask) {
          rec_input.wr_mask := rec_input.wr_mask | ip_shift_offset
        }
        is (SRAMRdWrEn) {
          rec_input.rd_en := io.ports(i).ip.asBool
        }
        is (SRAMRdWrMode) {
          rec_input.wr_en := io.ports(i).ip.asBool
        }
        is (SRAMRdWrAddr) {
          rec_input.rd_addr := rec_input.rd_addr | ip_shift_offset
        }
      }
    }
  }

  val wen = Wire(Bool())
  val ren = Wire(Bool())
  val waddr = Wire(UInt(sram_addr_bits.W))
  val raddr = Wire(UInt(sram_addr_bits.W))

  when (io.cfg.single_port_ram) {
    ren   := !cur_input.wr_en && cur_input.rd_en
    wen   :=  cur_input.wr_en && cur_input.rd_en
    waddr := cur_input.rd_addr
  } .otherwise {
    ren   := cur_input.rd_en
    wen   := cur_input.wr_en
    waddr := cur_input.wr_addr
  }

  when (ren) {
    raddr := cur_input.rd_addr
  } .otherwise {
    raddr := prev_input.rd_addr
  }

  val sram_rport_addr = Mux(wen && pc === 0.U, waddr, raddr)
  val rdata = sram.read(sram_rport_addr, io.run)

  val masked_wr_data = Module(new SRAMMaskedWriteData(cfg))
  masked_wr_data.io.wr_mask_bits := io.wmask_bits
  masked_wr_data.io.wr_mask      := cur_input.wr_mask
  masked_wr_data.io.wr_data      := cur_input.wr_data
  masked_wr_data.io.rd_data      := rdata

  when (io.run && wen && self.pc === cfg.sram_rd_lat.U) {
    sram.write(waddr, masked_wr_data.io.masked_wr_data)
  } .elsewhen (!init) {
    sram.write(pc, 0.U)
  }

  for (i <- 0 until num_procs) {
    io.ports(i).op := rdata >> io.ports(i).idx
  }

  when (io.run) {
    when (pc === io.host_steps - 1.U) {
      pc := 0.U
      when (ren) {
        prev_input := cur_input
      }

      cur_input.rd_en   := false.B
      cur_input.wr_en   := false.B
      cur_input.rd_addr := 0.U
      cur_input.wr_addr := 0.U
      cur_input.wr_data := 0.U
      cur_input.wr_mask := 0.U

      cur := cur + 1.U
    } .otherwise {
      pc := pc + 1.U
    }
  } .elsewhen (!init) {
    pc := pc + 1.U
    when (pc === (sram_entries - 1).U) {
      pc := 0.U
      init := true.B
    }
  }
}
