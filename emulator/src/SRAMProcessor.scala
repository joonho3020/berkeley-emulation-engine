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
    val wr_mask_bits   = Input(UInt(cfg.sram_width_bits.W))
    val width_bits     = Input(UInt(cfg.sram_width_bits.W))
    val wr_mask        = Input(UInt(cfg.sram_width_bits.W))
    val wr_data        = Input(UInt(cfg.sram_width.W))
    val rd_data        = Input(UInt(cfg.sram_width.W))
    val masked_wr_data = Output(UInt(cfg.sram_width.W))
  })

  val wr_data_mask_bits = Seq.fill(cfg.sram_width)(Wire(UInt(1.W)))
  for (i <- 0 until cfg.sram_width) {
    // val num_bits_per_mask = Wire(UInt(cfg.sram_width_bits.W))
    // num_bits_per_mask := io.width_bits / io.wmask_bits
    // wr_data_mask(i) := (io.wmask >> (i.U / num_bits_per_mask)) & 1.U
    // wr_data_mask(i) := (io.wmask >> (i.U * io.wmask_bits / io.width_bits)) & 1.U
    wr_data_mask_bits(i) := (io.wr_mask >> (i.U * io.wr_mask_bits / io.width_bits)) & 1.U
  }

  val wr_data_mask = Wire(UInt(cfg.sram_width.W))
  wr_data_mask := Cat(wr_data_mask_bits.reverse)

  when (io.wr_mask_bits === 0.U) {
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
  val ports      = Vec(cfg.num_procs, new PerProcessorSRAMBundle(cfg))
  val cfg_in     = new SRAMProcessorConfigBundle(cfg)
  val run        = Input(Bool())
  val host_steps = Input(UInt(cfg.index_bits.W))
  val init       = Output(Bool())
}

class SRAMInputs(cfg: EmulatorConfig) extends Bundle {
  val rd_en   = Bool()
  val wr_en   = Bool()
  val rd_addr = UInt(cfg.sram_addr_bits.W)
  val wr_addr = UInt(cfg.sram_addr_bits.W)
  val wr_data = UInt(cfg.sram_width.W)
  val wr_mask = UInt(cfg.sram_width.W)
}

case class SRAMProcessorAnno(
  target: firrtl.annotations.ReferenceTarget,
  customData: String) extends firrtl.annotations.SingleTargetAnnotation[firrtl.annotations.ReferenceTarget] {
  // This method is required to map the annotation to another target
  def duplicate(n: firrtl.annotations.ReferenceTarget): SRAMProcessorAnno = this.copy(target = n)
}

@instantiable
class SRAMProcessor(cfg: EmulatorConfig) extends Module {
  import cfg._
  @public val io = IO(new SRAMProcessorBundle(cfg))

  val init = RegInit(false.B)
  val pc   = RegInit(0.U(index_bits.W))
  val cur  = RegInit(0.U(1.W))
  val inputs = Seq.fill(2)(RegInit(0.U.asTypeOf(new SRAMInputs(cfg))))
  val prev_input = Reg(new SRAMInputs(cfg))
  val sram = SyncReadMem(cfg.sram_entries, UInt(cfg.sram_width.W))

  annotate(new ChiselAnnotation {
    override def toFirrtl: SRAMProcessorAnno = {
      SRAMProcessorAnno(sram.toTarget, "SRAMProcessorAnno")
    }
  })

  io.init := init

  val rec = Wire(UInt(1.W))
  rec := cur + 1.U

  val decs = Seq.fill(num_procs)(Module(new SRAMIndexDecoder(cfg)))
  for (i <- 0 until num_procs) {
    decs(i).io.idx := io.ports(i).idx
  }

  val ip_shift_offsets = Seq.fill(num_procs)(Wire(UInt(sram_addr_width_max.W)))
  ip_shift_offsets.zip(io.ports).zipWithIndex.foreach({ case ((iso, p), i) => {
    iso := Mux(p.valid && io.run, p.ip << decs(i).io.offset, 0.U)
  }})

  val recv_rd_en = Wire(UInt(1.W))
  recv_rd_en := decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMRdEn ||
      d.io.prim === SRAMInputTypes.SRAMRdWrEn)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

  val recv_wr_en = Wire(UInt(1.W))
  recv_wr_en := decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMWrEn ||
      d.io.prim === SRAMInputTypes.SRAMRdWrMode)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

  val recv_rd_addr = Wire(UInt(sram_addr_width_max.W))
  recv_rd_addr := decs.map(d =>
      d.io.prim === SRAMInputTypes.SRAMRdAddr ||
      d.io.prim === SRAMInputTypes.SRAMRdWrAddr)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

  val recv_wr_addr = Wire(UInt(sram_addr_width_max.W))
  recv_wr_addr := decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrAddr)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

  val recv_wr_data = Wire(UInt(sram_addr_width_max.W))
  recv_wr_data := decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrData)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

  val recv_wr_mask = Wire(UInt(sram_addr_width_max.W))
  recv_wr_mask := decs
    .map(_.io.prim === SRAMInputTypes.SRAMWrMask)
    .zip(ip_shift_offsets)
    .map({ case (prim_match, iso) => Mux(prim_match, iso, 0.U) })
    .reduce(_ | _)

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

  when (io.cfg_in.single_port_ram) {
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
  masked_wr_data.io.wr_mask_bits := io.cfg_in.wmask_bits
  masked_wr_data.io.width_bits   := io.cfg_in.width_bits
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
}
