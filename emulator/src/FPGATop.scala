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
  // Adds a extra DMA stream engine to check for XDMA DMA transactions
  debug: Boolean,

  // XDMA AXI4 parameters for DMA
  axi:  FPGATopAXI4DMAParams,

  // XDMA AXI4-lite parameters for MMIO
  axil: FPGATopAXI4MMIOParams,

  // Emulation platform configuration
  emul: EmulatorConfig)

case object FPGATopConfigKey extends Field[FPGATopParams]

class FPGATop(implicit p: Parameters) extends LazyModule {
  val cfg = p(FPGATopConfigKey)

  println("================= Emulator configuration =======================");
  println(cfg)
  println("================================================================");

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

  val axiDMASlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves    = Seq(
        AXI4SlaveParameters(
          address       = Seq(AddressSet(0, (BigInt(1) << cfg.axi.addrBits) - 1)),
          resources     = (new MemoryDevice).reg,
          regionType    = RegionType.UNCACHED, // cacheable
          executable    = false,
          supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
          supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
          interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  axiDMASlaveNode := AXI4Buffer() := axiDMAMasterNode

// val targetIOAddrSize = BigInt(1) << 12
// val axiDMATargetIOSlaveNode = AXI4SlaveNode(Seq(
// AXI4SlavePortParameters(
// slaves = Seq(AXI4SlaveParameters(
// address = Seq(AddressSet(0, targetIOAddrSize - 1)),
// resources     = (new MemoryDevice).reg,
// regionType    = RegionType.UNCACHED,
// executable    = false,
// supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
// supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
// interleavedId = Some(0))),
// beatBytes = cfg.axi.dataBits / 8)))

// val axiDMAInstSlaveNode = AXI4SlaveNode(Seq(
// AXI4SlavePortParameters(
// slaves = Seq(AXI4SlaveParameters(
// address = Seq(AddressSet(targetIOAddrSize, targetIOAddrSize - 1)),
// resources     = (new MemoryDevice).reg,
// regionType    = RegionType.UNCACHED,
// executable    = false,
// supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
// supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
// interleavedId = Some(0))),
// beatBytes = cfg.axi.dataBits / 8)))

// val axiDMATestSlaveNode = if (cfg.debug) {
// Some(AXI4SlaveNode(Seq(
// AXI4SlavePortParameters(
// slaves = Seq(AXI4SlaveParameters(
// address = Seq(AddressSet(2 * targetIOAddrSize, targetIOAddrSize - 1)),
// resources     = (new MemoryDevice).reg,
// regionType    = RegionType.UNCACHED,
// executable    = false,
// supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
// supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
// interleavedId = Some(0))),
// beatBytes = cfg.axi.dataBits / 8))))
// } else {
// None
// }

// val dmaXbarNode = AXI4Xbar()
// dmaXbarNode := AXI4Buffer() := axiDMAMasterNode
// axiDMATargetIOSlaveNode := AXI4Buffer() := dmaXbarNode
// axiDMAInstSlaveNode     := AXI4Buffer() := dmaXbarNode
// axiDMATestSlaveNode.map(_ := AXI4Buffer() := dmaXbarNode)

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
        supportsWrite = TransferSizes(cfg.axil.dataBits / 8, cfg.axil.dataBits / 8),
        supportsRead  = TransferSizes(cfg.axil.dataBits / 8, cfg.axil.dataBits / 8),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  axiMMIOSlaveNode := AXI4Buffer() := axiMMIOMasterNode

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopParams) extends LazyModuleImp(outer) {
  println(cfg.axi)

  val io_dma_axi4_master = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  outer.axiDMAMasterNode.out.head._1 <> io_dma_axi4_master

  val io_dma_axi4_slave = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
  io_dma_axi4_slave <> outer.axiDMASlaveNode.in.head._1

  dontTouch(io_dma_axi4_master)
  dontTouch(io_dma_axi4_slave)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val dataBits = cfg.axi.axi4BundleParams.dataBits
  val io_stream_width = (((total_procs + dataBits - 1) / dataBits) * dataBits).toInt
  println(s"io_stream_width: ${io_stream_width}")
  println(s"total_procs: ${total_procs}")

  val stream_converter = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    widthBits_1   = io_stream_width,
    bufferDepth_1 = 4,
    widthBits_2   = cfg.axi.axi4BundleParams.dataBits,
    bufferDepth_2 = 128,
    widthBits_3   = io_stream_width,
    bufferDepth_3 = 4,
    addressSpaceBits = 12))

  stream_converter.io.axi <> io_dma_axi4_slave

  stream_converter.io.enq_2.valid := false.B
  stream_converter.io.enq_2.bits  := 0.U
  stream_converter.io.deq_2.ready := false.B

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

  val num_regs = 3 * cfg.emul.num_mods + 8
  val mcr = Module(new MCRFile(num_regs)(nasti_lite_params))

  val routeSel: UInt => UInt = (addr: UInt) => {
    (addr >= 0.U && addr < (num_regs << 2).U).asUInt
  }
  val nasti_router = Module(new NastiRouter(1, routeSel)(nasti_lite_params))
  nasti_router.io.master <> m_nasti_lite
  mcr.io.nasti <> nasti_router.io.slave(0)
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
      board.io.insts(i).valid := stream_converter.io.deq_2.valid
      board.io.insts(i).bits  := stream_converter.io.deq_2.bits.asTypeOf(Instruction(cfg.emul))
      stream_converter.io.deq_2.ready := board.io.insts(i).ready
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
      board.io.io(i).i(j) := stream_converter.io.deq_1.bits >> (idx * cfg.emul.num_bits)
    }
  }

  stream_converter.io.enq_1.bits := Cat(board.io.io.flatMap(io => io.o).reverse)

  val board_run = DecoupledHelper(
    stream_converter.io.deq_1.valid,
    stream_converter.io.enq_1.ready)

  val last_step = cur_step === host_steps - 1.U
  board.io.run := board_run.fire()
  stream_converter.io.deq_1.ready := board_run.fire(stream_converter.io.deq_1.valid, last_step)
  stream_converter.io.enq_1.valid := board_run.fire(stream_converter.io.enq_1.ready, last_step)

  when (board.io.run) {
    cur_step := Mux(last_step, 0.U, cur_step + 1.U)
  }

  when (stream_converter.io.enq_1.fire) {
    target_cycle := target_cycle + 1.U
  }

  MCRFile.bind_readonly_reg(stream_converter.io.deq_cnt_1, mcr,            3 * cfg.emul.num_mods + 2)
  MCRFile.bind_readonly_reg(stream_converter.io.enq_cnt_1, mcr,            3 * cfg.emul.num_mods + 3)
  MCRFile.bind_readonly_reg(target_cycle & ((BigInt(1) << 32) - 1).U, mcr, 3 * cfg.emul.num_mods + 4)
  MCRFile.bind_readonly_reg(target_cycle >> 32,                       mcr, 3 * cfg.emul.num_mods + 5)

  val fingerprint_reg = RegInit(0.U(32.W))
  MCRFile.bind_readwrite_reg(fingerprint_reg, mcr, 3 * cfg.emul.num_mods + 6)

  val dma_test_q = Module(new Queue(UInt(io_stream_width.W), 4))
  dma_test_q.io.enq <> stream_converter.io.deq_3
  stream_converter.io.enq_3 <> dma_test_q.io.deq
  MCRFile.bind_readonly_reg(dma_test_q.io.count, mcr, 3 * cfg.emul.num_mods + 7)
}
