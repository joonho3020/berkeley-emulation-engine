package emulator

import chisel3._
import chisel3.util._
import chisel3.util.Decoupled
import chisel3.experimental._
import chisel3.experimental.hierarchy.{instantiable, public}

object SRAMInputTypes extends ChiselEnum {
  val SRAMRdEn,
      SRAMWrEn,
      SRAMRdAddr,
      SRAMWrAddr,
      SRAMWrData,
      SRAMWrMask,
      SRAMRdWrEn,
      SRAMRdWrMode,
      SRAMRdWrAddr = Value
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
    io.prim   := SRAMInputTypes.SRAMRdWrAddr
    io.offset := io.idx - cfg.sram_rdwr_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_rdwr_mode_offset.U) {
    io.prim   := SRAMInputTypes.SRAMRdWrMode
    io.offset := io.idx - cfg.sram_rdwr_mode_offset.U
  } .elsewhen (io.idx >= cfg.sram_rdwr_en_offset.U) {
    io.prim   := SRAMInputTypes.SRAMRdWrEn
    io.offset := io.idx - cfg.sram_rdwr_en_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_mask_offset.U) {
    io.prim   := SRAMInputTypes.SRAMWrMask
    io.offset := io.idx - cfg.sram_wr_mask_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_data_offset.U) {
    io.prim   := SRAMInputTypes.SRAMWrData
    io.offset := io.idx - cfg.sram_wr_data_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_addr_offset.U) {
    io.prim   := SRAMInputTypes.SRAMWrAddr
    io.offset := io.idx - cfg.sram_wr_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_rd_addr_offset.U) {
    io.prim   := SRAMInputTypes.SRAMRdAddr
    io.offset := io.idx - cfg.sram_rd_addr_offset.U
  } .elsewhen (io.idx >= cfg.sram_wr_en_offset.U) {
    io.prim   := SRAMInputTypes.SRAMWrEn
    io.offset := io.idx - cfg.sram_wr_en_offset.U
  } .elsewhen (io.idx >= cfg.sram_rd_en_offset.U) {
    io.prim   := SRAMInputTypes.SRAMRdEn
    io.offset := io.idx - cfg.sram_rd_en_offset.U
  }
}

class SRAMMaskedWriteData(cfg: EmulatorConfig) extends Module {
  val io = IO(new Bundle {
    val wr_mask_bits   = Input (UInt(cfg.sram_width_bits.W))
    val width_bits     = Input (UInt(cfg.sram_width_bits.W))
    val wr_mask        = Input (UInt(cfg.large_sram_width.W))
    val wr_data        = Input (UInt(cfg.large_sram_width.W))
    val rd_data        = Input (UInt(cfg.large_sram_width.W))
    val masked_wr_data = Output(UInt(cfg.large_sram_width.W))
  })
  when (io.wr_mask_bits === 0.U) {
    io.masked_wr_data := io.wr_data
  } .otherwise {
    io.masked_wr_data := (io.wr_data & io.wr_mask) | (io.rd_data & ~io.wr_mask)
  }
}

class PerProcessorSRAMBundle(cfg: EmulatorConfig) extends Bundle {
  val idx   = Input (UInt(cfg.sram_unique_indices_bits.W))
  val valid = Input (Bool())
  val ip    = Input (UInt(cfg.num_bits.W))
  val op    = Output(UInt(cfg.num_bits.W))
}

class SRAMProcessorConfigBundle(cfg: EmulatorConfig) extends Bundle {
  val single_port_ram = Bool()
  val wmask_bits      = UInt(cfg.sram_width_bits.W)
  val width_bits      = UInt(cfg.sram_width_bits.W)
}

class SRAMProcessorBundle(cfg: EmulatorConfig) extends Bundle {
  val ports      = Vec(cfg.num_procs, new PerProcessorSRAMBundle(cfg))
  val cfg_in     = Input(new SRAMProcessorConfigBundle(cfg))
  val run        = Input(Bool())
  val host_steps = Input(UInt(cfg.index_bits.W))
  val init       = Output(Bool())
}

class SRAMInputs(cfg: EmulatorConfig) extends Bundle {
  val rd_en   = Bool()
  val wr_en   = Bool()
  val rd_addr = UInt(cfg.sram_addr_bits.W)
  val wr_addr = UInt(cfg.sram_addr_bits.W)
  val wr_data = UInt(cfg.large_sram_width.W)
  val wr_mask = UInt(cfg.large_sram_width.W)
}

case class SRAMProcessorAnno(
  target: firrtl.annotations.ReferenceTarget,
  customData: String) extends firrtl.annotations.SingleTargetAnnotation[firrtl.annotations.ReferenceTarget] {
  // This method is required to map the annotation to another target
  def duplicate(n: firrtl.annotations.ReferenceTarget): SRAMProcessorAnno = this.copy(target = n)
}

@instantiable
class SRAMProcessor(cfg: EmulatorConfig, large_sram: Boolean) extends Module {
  import cfg._
  @public val io = IO(new SRAMProcessorBundle(cfg))

  // PC must have log2(sram_entries) bits for initialization
  val pc   = RegInit(0.U(log2Ceil(sram_entries + 1).W))
  val init = RegInit(false.B)
  val cur  = RegInit(0.U(1.W))
  val inputs = Seq.fill(2)(RegInit(0.U.asTypeOf(new SRAMInputs(cfg))))
  val prev_input = Reg(new SRAMInputs(cfg))

  // To cut critical path in FPGA
  val cfg_regs = Reg(new SRAMProcessorConfigBundle(cfg))
  cfg_regs := io.cfg_in

  val cur_sram_entries = if (large_sram) cfg.large_sram_entries else cfg.sram_entries
  val cur_sram_width   = if (large_sram) cfg.large_sram_width else cfg.sram_width
  val sram = SyncReadMem(cur_sram_entries, UInt(cur_sram_width.W))

  annotate(new ChiselAnnotation {
    override def toFirrtl: SRAMProcessorAnno = {
      SRAMProcessorAnno(sram.toAbsoluteTarget, sram.pathName)
    }
  })

  io.init := init

  val rec = Wire(UInt(1.W))
  rec := cur + 1.U


  // Pipeline registers to cut critical path
  val pl_ip    = io.ports.map(port => ShiftRegister(port.ip   , cfg.sram_ip_pl))
  val pl_idx   = io.ports.map(port => ShiftRegister(port.idx  , cfg.sram_ip_pl))
  val pl_valid = io.ports.map(port => ShiftRegister(port.valid, cfg.sram_ip_pl))

  val decs = Seq.fill(num_procs)(Module(new SRAMIndexDecoder(cfg)))
  for (i <- 0 until num_procs) {
    decs(i).io.idx := pl_idx(i)
  }

  println(s"sram_addr_width_max: ${sram_addr_width_max}")

  val sram_addr_width_max_log2 = log2Ceil(sram_addr_width_max)

  val ip_shift_offsets = Seq.fill(num_procs)(Wire(UInt(sram_addr_width_max.W)))
  ip_shift_offsets.zip(pl_ip).zip(pl_valid).zipWithIndex.foreach({
    case (((iso, ip), valid), i) => {
      val ip_shift_offset = Wire(UInt(sram_addr_width_max.W))
      ip_shift_offset := ip << decs(i).io.offset(sram_addr_width_max_log2-1, 0)
      iso := Mux(valid && io.run, ip_shift_offset, 0.U)
    }
  })

  val recv_rd_en = Wire(UInt(1.W))
  val recv_rd_en_vec = Wire(Vec(num_procs, UInt(1.W)))
  decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMRdEn ||
      d.io.prim === SRAMInputTypes.SRAMRdWrEn)
        .zip(ip_shift_offsets)
        .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
        .zip(recv_rd_en_vec)
        .map({ case (bit, rd_en) => rd_en := bit })
  recv_rd_en := recv_rd_en_vec.reduceTree(_ | _)

  val recv_wr_en = Wire(UInt(1.W))
  val recv_wr_en_vec = Wire(Vec(num_procs, UInt(1.W)))
  decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMWrEn ||
      d.io.prim === SRAMInputTypes.SRAMRdWrMode)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .zip(recv_wr_en_vec)
    .map({ case (bit, wr_en) => wr_en := bit })
  recv_wr_en := recv_wr_en_vec.reduceTree(_ | _)

  val recv_rd_addr = Wire(UInt(sram_addr_width_max.W))
  val recv_rd_addr_vec = Wire(Vec(num_procs, UInt(sram_addr_width_max.W)))
  decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMRdAddr ||
      d.io.prim === SRAMInputTypes.SRAMRdWrAddr)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .zip(recv_rd_addr_vec)
    .map({ case (bits, rd_addr) => rd_addr := bits })
  recv_rd_addr := recv_rd_addr_vec.reduceTree(_ | _)

  val recv_wr_addr = Wire(UInt(sram_addr_width_max.W))
  val recv_wr_addr_vec = Wire(Vec(num_procs, UInt(sram_addr_width_max.W)))
  decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrAddr)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .zip(recv_wr_addr_vec)
    .map({ case (bits, wr_addr) => wr_addr := bits })
  recv_wr_addr := recv_wr_addr_vec.reduceTree(_ | _)

  val recv_wr_data = Wire(UInt(sram_addr_width_max.W))
  val recv_wr_data_vec = Wire(Vec(num_procs, UInt(sram_addr_width_max.W)))
  decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrData)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .zip(recv_wr_data_vec)
    .map({ case (bits, wr_data) => wr_data := bits })
  recv_wr_data := recv_wr_data_vec.reduceTree(_ | _)

  val recv_wr_mask = Wire(UInt(sram_addr_width_max.W))
  val recv_wr_mask_vec = Wire(Vec(num_procs, UInt(sram_addr_width_max.W)))
  decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrMask)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .zip(recv_wr_mask_vec)
    .map({ case (bits, wr_mask) => wr_mask := bits })
  recv_wr_mask := recv_wr_mask_vec.reduceTree(_ | _)

  for (i <- 0 until 2) {
    when (rec === i.U) {
      inputs(i).rd_en   := inputs(i).rd_en   | recv_rd_en
      inputs(i).wr_en   := inputs(i).wr_en   | recv_wr_en
      inputs(i).rd_addr := inputs(i).rd_addr | recv_rd_addr
      inputs(i).wr_addr := inputs(i).wr_addr | recv_wr_addr
      inputs(i).wr_data := inputs(i).wr_data | recv_wr_data
      inputs(i).wr_mask := inputs(i).wr_mask | recv_wr_mask
    }
  }

  val wen = Wire(Bool())
  val ren = Wire(Bool())
  val waddr = Wire(UInt(sram_addr_bits.W))
  val raddr = Wire(UInt(sram_addr_bits.W))

  val cur_input = Mux(cur === 0.U, inputs(0), inputs(1))

  when (cfg_regs.single_port_ram) {
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
  masked_wr_data.io.wr_mask_bits := cfg_regs.wmask_bits
  masked_wr_data.io.width_bits   := cfg_regs.width_bits
  masked_wr_data.io.wr_mask      := cur_input.wr_mask
  masked_wr_data.io.wr_data      := cur_input.wr_data
  masked_wr_data.io.rd_data      := rdata

  val wcond = io.run && wen && pc === cfg.sram_rd_lat.U
  val wport_waddr = Mux(wcond, waddr, pc)
  val wport_wdata = Mux(wcond, masked_wr_data.io.masked_wr_data, 0.U)
  when (wcond || !init) {
    sram.write(wport_waddr, wport_wdata)
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

      when (cur === 0.U) {
        inputs(0).rd_en   := false.B
        inputs(0).wr_en   := false.B
        inputs(0).rd_addr := 0.U
        inputs(0).wr_addr := 0.U
        inputs(0).wr_data := 0.U
        inputs(0).wr_mask := 0.U
      } .otherwise {
        inputs(1).rd_en   := false.B
        inputs(1).wr_en   := false.B
        inputs(1).rd_addr := 0.U
        inputs(1).wr_addr := 0.U
        inputs(1).wr_data := 0.U
        inputs(1).wr_mask := 0.U
      }

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

  when (wcond && cfg.debug.asBool) {
    Logger.logInfo("RTL:  SRAM single_port: %d width: %d wmask: %d ren: %d wen: %d wr_addr: 0x%x wr_data: 0x%x wr_mask: 0x%x rd_addr: 0x%x\n",
      cfg_regs.single_port_ram,
      cfg_regs.width_bits,
      cfg_regs.wmask_bits,
      cur_input.rd_en,
      cur_input.wr_en,
      cur_input.wr_addr,
      cur_input.wr_data,
      cur_input.wr_mask,
      cur_input.rd_addr)
  }
}
