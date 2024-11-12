package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._
import freechips.rocketchip.util.DecoupledHelper

case class FPGATopAXI4DMAParams(
  addrBits:  Int,
  dataBits:  Int,
  idBits:    Int,
  maxFlight: Option[Int] = None,
) {
  def axi4BundleParams = AXI4BundleParameters(
    addrBits = addrBits,
    dataBits = dataBits,
    idBits   = idBits,
  )
}

case class FPGATopAXI4MMIOParams(
  addrBits:  Int,
  dataBits:  Int,
  idBits:    Int,
  maxFlight: Option[Int] = None,
) {
  def axi4BundleParams = AXI4BundleParameters(
    addrBits = addrBits,
    dataBits = dataBits,
    idBits   = idBits,
  )
}

case class FPGATopParams(
  debug: Boolean,
  axi:  FPGATopAXI4DMAParams,
  axil: FPGATopAXI4MMIOParams,
  emul: EmulatorConfig)

case object FPGATopConfigKey extends Field[FPGATopParams]

class FPGATop(implicit p: Parameters) extends LazyModule {
  val cfg = p(FPGATopConfigKey)

  println(cfg)

   // AXI4 Master Node with a single master port
  val axiDMAMasterNode = AXI4MasterNode(Seq(
    AXI4MasterPortParameters(
      masters = Seq(AXI4MasterParameters(
        name      = "cpu-managed-axi4",
        id        = IdRange(0, 1 << cfg.axi.idBits),
        aligned   = false,
        // None = infinite, else is a per-ID cap
        maxFlight = cfg.axi.maxFlight)
      ))))

  val targetIOAddrSize = BigInt(1) << 12
  val axiDMATargetIOSlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves = Seq(AXI4SlaveParameters(
        address = Seq(AddressSet(0, targetIOAddrSize - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  val axiDMAInstSlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves = Seq(AXI4SlaveParameters(
        address = Seq(AddressSet(targetIOAddrSize, targetIOAddrSize - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  val axiDMATestSlaveNode = if (cfg.debug) {
    Some(AXI4SlaveNode(Seq(
      AXI4SlavePortParameters(
        slaves = Seq(AXI4SlaveParameters(
          address = Seq(AddressSet(2 * targetIOAddrSize, targetIOAddrSize - 1)),
          resources     = (new MemoryDevice).reg,
          regionType    = RegionType.UNCACHED,
          executable    = false,
          supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
          supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
          interleavedId = Some(0))),
        beatBytes = cfg.axi.dataBits / 8))))
  } else {
    None
  }

  val dmaXbarNode = AXI4Xbar()
  dmaXbarNode := AXI4Buffer() := axiDMAMasterNode
  axiDMATargetIOSlaveNode := AXI4Buffer() := dmaXbarNode
  axiDMAInstSlaveNode     := AXI4Buffer() := dmaXbarNode
  axiDMATestSlaveNode.map(_ := AXI4Buffer() := dmaXbarNode)

   // AXI4-Lite Master Node with a single master port
  val axiMMIOMasterNode = AXI4MasterNode(Seq(
    AXI4MasterPortParameters(
      masters = Seq(AXI4MasterParameters(
        name      = "ctrl-axi-lite",
        id        = IdRange(0, 1 << cfg.axil.idBits),
        aligned   = false,
        // None = infinite, else is a per-ID cap
        maxFlight = cfg.axil.maxFlight)
      ))))

  val axiMMIOSlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves = Seq(AXI4SlaveParameters(
        address = Seq(AddressSet(0, (BigInt(1) << cfg.axil.addrBits) - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axil.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axil.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  axiMMIOSlaveNode := AXI4Buffer() := axiMMIOMasterNode

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopParams) extends LazyModuleImp(outer) {
  println(cfg.axi)

  val io_dma_axi4_master = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  outer.axiDMAMasterNode.out.head._1 <> io_dma_axi4_master

  val dma_axi4_target_io = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
  dma_axi4_target_io <> outer.axiDMATargetIOSlaveNode.in.head._1

  val dma_axi4_inst = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
  dma_axi4_inst <> outer.axiDMAInstSlaveNode.in.head._1

  dontTouch(io_dma_axi4_master)
  dontTouch(dma_axi4_target_io)
  dontTouch(dma_axi4_inst)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val dataBits = cfg.axi.axi4BundleParams.dataBits
  val io_stream_width = (((total_procs + dataBits - 1) / dataBits) * dataBits).toInt
  println(s"io_stream_width: ${io_stream_width}")
  println(s"total_procs: ${total_procs}")

  val target_io_stream = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    widthBits = io_stream_width,
    bufferDepth = 4))

  target_io_stream.io.axi <> dma_axi4_target_io

  val target_inst_stream = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    widthBits = cfg.axi.axi4BundleParams.dataBits,
    bufferDepth = 128))

  target_inst_stream.io.axi <> dma_axi4_inst

  target_inst_stream.io.enq.valid := false.B
  target_inst_stream.io.enq.bits  := 0.U
  target_inst_stream.io.deq.ready := false.B

  ////////////////////////////////////////////////////////////////////////////
  // MMIO
  ////////////////////////////////////////////////////////////////////////////

  val io_mmio_axi4_master = IO(Flipped(AXI4Bundle(cfg.axil.axi4BundleParams)))
  outer.axiMMIOMasterNode.out.head._1 <> io_mmio_axi4_master
  dontTouch(io_mmio_axi4_master)

  val mmio_axi4_slave = Wire(AXI4Bundle(cfg.axil.axi4BundleParams))
  mmio_axi4_slave <> outer.axiMMIOSlaveNode.in.head._1

  val axil_params = cfg.axil.axi4BundleParams
  val nasti_lite_params = NastiParameters(axil_params.dataBits, axil_params.addrBits, axil_params.idBits)
  val m_nasti_lite = Wire(new NastiIO(nasti_lite_params))
  AXI4NastiAssigner.toNasti(m_nasti_lite, mmio_axi4_slave)

  val mcr = Module(new MCRFile(4 * cfg.emul.num_mods + 3)(nasti_lite_params))
  mcr.io.nasti <> m_nasti_lite
  MCRFile.tieoff(mcr)

  // Write Only Register mapping
  // - used_procs (0~num_mods-1)
  // - single_port_ram (0~num_mods-1)
  // - wmask_bits (0~num_mods-1)
  // - width_bits (0~num_mods-1)
  // - host_steps

  val num_mods_log2 = log2Ceil(cfg.emul.num_mods + 1)

  val single_port_ram = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(single_port_ram, mcr, 0)

  val wmask_bits = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(wmask_bits, mcr, 1 * cfg.emul.num_mods)

  val width_bits = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(width_bits, mcr,  2 * cfg.emul.num_mods)

  val host_steps = RegInit(0.U(cfg.emul.index_bits.W))
  MCRFile.bind_writeonly_reg(host_steps, mcr, 3 * cfg.emul.num_mods)




  ////////////////////////////////////////////////////////////////////////////



  val board = Module(new Board(cfg.emul))

  // Read Only Register mapping
  // - init
  val init = RegNext(board.io.init)
  MCRFile.bind_readonly_reg(init, mcr, 3 * cfg.emul.num_mods + 1)

  for (i <- 0 until cfg.emul.num_mods) {
    board.io.cfg_in(i).host_steps := host_steps
    board.io.cfg_in(i).sram.single_port_ram := single_port_ram(i)
    board.io.cfg_in(i).sram.wmask_bits      := wmask_bits(i)
    board.io.cfg_in(i).sram.width_bits      := width_bits(i)
  }

  // TODO: make this into parallel streams to make the loading faster(?)
  val cur_inst_mod = RegInit(0.U(log2Ceil(cfg.emul.num_mods + 1).W))
  val cur_insts_pushed = RegInit(0.U(log2Ceil(cfg.emul.insts_per_mod + 1).W))

  for (i <- 0 until cfg.emul.num_mods) {
    board.io.insts(i).valid := false.B
    board.io.insts(i).bits  := DontCare
  }

  for (i <- 0 until cfg.emul.num_mods) {
    when (i.U === cur_inst_mod) {
      board.io.insts(i).valid := target_inst_stream.io.deq.valid
      board.io.insts(i).bits  := target_inst_stream.io.deq.bits.asTypeOf(Instruction(cfg.emul))
      target_inst_stream.io.deq.ready := board.io.insts(i).ready
      when (board.io.insts(i).fire) {
        when (cur_insts_pushed === host_steps * cfg.emul.num_procs.U - 1.U) {
          cur_insts_pushed := 0.U
          cur_inst_mod := cur_inst_mod + 1.U
        } .otherwise {
          cur_insts_pushed := cur_insts_pushed + 1.U
        }
      }
    }
  }

  val cur_step = RegInit(0.U(cfg.emul.index_bits.W))
  val target_cycle = RegInit(0.U(64.W))

  // TODO: DRAM interface should go here
  for (i <- 0 until cfg.emul.num_mods) {
    for (j <- 0 until cfg.emul.num_procs) {
      val idx = i * cfg.emul.num_procs + j
      board.io.io(i).i(j) := target_io_stream.io.deq.bits >> (idx * cfg.emul.num_bits)
    }
  }

  target_io_stream.io.enq.bits := Cat(board.io.io.flatMap(io => io.o).reverse)

  val board_run = DecoupledHelper(
    target_io_stream.io.deq.valid,
    target_io_stream.io.enq.ready)

  val last_step = cur_step === host_steps - 1.U
  board.io.run := board_run.fire()
  target_io_stream.io.deq.ready := board_run.fire(target_io_stream.io.deq.valid, last_step)
  target_io_stream.io.enq.valid := board_run.fire(target_io_stream.io.enq.ready, last_step)

  when (board.io.run) {
    cur_step := Mux(last_step, 0.U, cur_step + 1.U)
  }

  when (target_io_stream.io.enq.fire) {
    target_cycle := target_cycle + 1.U
  }

  MCRFile.bind_readonly_reg(target_io_stream.io.deq_cnt, mcr,              3 * cfg.emul.num_mods + 2)
  MCRFile.bind_readonly_reg(target_io_stream.io.enq_cnt, mcr,              3 * cfg.emul.num_mods + 3)
  MCRFile.bind_readonly_reg(target_cycle & ((BigInt(1) << 32) - 1).U, mcr, 3 * cfg.emul.num_mods + 4)
  MCRFile.bind_readonly_reg(target_cycle >> 32,                       mcr, 3 * cfg.emul.num_mods + 5)

  val fingerprint_reg = RegInit(0.U(32.W))
  MCRFile.bind_readwrite_reg(fingerprint_reg, mcr, 3 * cfg.emul.num_mods + 6)

  outer.axiDMATestSlaveNode match {
    case Some(node) => {
      val x = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
      x <> node.in.head._1
      dontTouch(x)

      val test_stream = Module(new AXI4DecoupledConverter(
        axiParams = cfg.axi.axi4BundleParams,
        widthBits = io_stream_width,
        bufferDepth = 4))

      test_stream.io.axi <> x

      val q = Module(new Queue(UInt(io_stream_width.W), 4))
      q.io.enq <> test_stream.io.deq
      test_stream.io.enq <> q.io.deq

      MCRFile.bind_readonly_reg(q.io.count, mcr, 3 * cfg.emul.num_mods + 7)
    }
    case _ => { }
  }
}
