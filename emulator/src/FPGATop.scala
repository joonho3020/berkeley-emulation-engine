package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._

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
        address = Seq(AddressSet(targetIOAddrSize, (BigInt(1) << cfg.axi.addrBits) - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  val dmaXbarNode = AXIXbar()
  dmaXbarNode := AXI4Buffer() := axiDMAMasterNode
  axiDMATargetIOSlaveNode := dmaXbarNode
  axiDMAInstSlaveNode     := dmaXbarNode


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

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopParams) extends LazyModuleImp(outer) {


  println(cfg.axi)

  val io_dma_axi4_master = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  outer.axiDMAMasterNode.out.head._1 <> io_dma_axi4_master

  val dma_axi4_target_io = Wire(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  dma_axi4_target_io <> outer.axiDMATargetIOSlaveNode.in.head._1

  val dma_axi4_inst = Wire(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  dma_axi4_inst <> outer.axiDMAInstSlaveNode.in.head._1

  dontTouch(io_dma_axi4_master)
  dontTouch(dma_axi4_target_io)
  dontTouch(dma_axi4_inst)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val io_stream_width = (math.ceil(total_procs / cfg.axi.axi4BundleParams.dataBits) * cfg.axi.axi4BundleParams.dataBits).toInt

  val target_io_stream = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    widthBits = io_stream_width,
    bufferDepth = 4))

  val target_inst_stream = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    widthBits = cfg.axi.axi4BundleParams.dataBits,
    bufferDepth = 128))

  target_inst_stream.io.enq.valid := false.B
  target_inst_stream.io.enq.bits  := 0.U

  ////////////////////////////////////////////////////////////////////////////
  // MMIO
  ////////////////////////////////////////////////////////////////////////////

  val io_mmio_axi4_master = IO(Flipped(AXI4Bundle(cfg.axil.axi4BundleParams)))
  outer.axiMMIOMasterNode.out.head._1 <> io_mmio_axi4_master
  dontTouch(io_mmio_axi4_master)

  val axil_params = cfg.axil.axi4BundleParams
  val nasti_lite_params = NastiParameters(axil_params.dataBits, axil_params.addrBits, axil_params.idBits)
  val m_nasti_lite = Wire(new NastiIO(nasti_lite_params))
  AXI4NastiAssigner.toNasti(m_nasti_lite, io_mmio_axi4_master)

  val mcr = Module(new MCRFile(4 * cfg.emul.num_mods + 2)(nasti_lite_params))
  mcr.io.nasti <> m_nasti_lite

  // Write Only Register mapping
  // - used_procs (0~num_mods-1)
  // - single_port_ram (0~num_mods-1)
  // - wmask_bits (0~num_mods-1)
  // - width_bits (0~num_mods-1)
  // - host_steps

  val num_mods_log2 = log2Ceil(cfg.emul.num_mods + 1)

  val used_procs = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(used_procs, mcr, 0)

  val single_port_ram = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(single_port_ram, mcr, cfg.emul.num_mods)

  val wmask_bits = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(wmask_bits, mcr, 2 * cfg.emul.num_mods)

  val width_bits = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(width_bits, mcr,  3 * cfg.emul.num_mods)

  val host_steps = RegInit(0.U(cfg.emul.index_bits.W))
  val host_steps_w = Wire(host_steps.cloneType)
  MCRFile.bind_writeonly_reg(host_steps_w, mcr, 4 * cfg.emul.num_mods)




  val board = Module(new Board(cfg.emul))

  // Read Only Register mapping
  // - init
  val init = Reg(board.io.init)
  MCRFile.bind_readonly_reg(init, mcr, 4 * cfg.emul.num_mods + 1)

  for (i <- 0 until cfg.emul.num_mods) {
    board.io.cfg_in(i).host_steps := host_steps
    board.io.cfg_in(i).used_procs := used_procs(i)
    board.io.cfg_in(i).sram.single_port_ram := single_port_ram(i)
    board.io.cfg_in(i).sram.wmask_bits      := wmask_bits(i)
    board.io.cfg_in(i).sram.width_bits      := width_bits(i)
  }

  val cur_inst_mod = RegInit(0.U(log2Ceil(cfg.emul.num_mods + 1).W))
  val cur_insts_pushed = RegInit(0.U(log2Ceil(cfg.emul.insts_per_mod + 1).W))
  for (i <- 0 until cfg.emul.num_mods) {
    when (i.U === cur_inst_mod) {
      board.io.insts(i) <> target_inst_stream.io.deq.asTypeOf(Instruction(cfg.emul))
      when (board.io.insts(i).fire) {
        when (cur_insts_pushed === host_steps * cfg.emul.num_procs.U - 1.U) {
          cur_insts_pushed := 0.U
        } .otherwise {
          cur_insts_pushed := cur_insts_pushed + 1.U
          cur_inst_mod := cur_inst_mod + 1.U
        }
      }
    }
  }

  val cur_step = RegInit(0.U(cfg.emul.index_bits.W))

// val cfg_in = Vec(num_mods, Input(new EModuleConfigBundle(cfg)))
// val init = Output(Bool())
// val insts = Vec(num_mods, Flipped(Decoupled(Instruction(cfg))))

// val run = Input(Bool())
// val io = Vec(num_mods, new EModuleIOBitsBundle(cfg))

}
