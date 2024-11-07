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

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopParams) extends LazyModuleImp(outer) {

  println(cfg.axi)

  val io_dma_axi4_master = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  outer.axiDMAMasterNode.out.head._1 <> io_dma_axi4_master

  val dma_target_io = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  io_dma_axi4_slave <> outer.axiDMATargetIOSlaveNode.in.head._1

  dontTouch(io_dma_axi4_slave)
  dontTouch(io_dma_axi4_master)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val io_stream_width = Ceil(total_procs / cfg.axi.axi4BundleParams.dataBits) * cfg.axi.axi4BundleParams.dataBits

  val target_io_stream = Module(new AXI4DecoupledConverter(
    cfg.axi.axi4BundleParams,
    0,
    12,
    io_stream_width,
    4))

  val target_inst_stream = Module(new AXI4DecoupledConverter(
    cfg.axi.axi4BundleParams,
    1,
    12,
    cfg.axi.axi4BundleParams.dataBits,
    128))

  val board = Module(new Board(cfg.emul))

// val cfg_in = Vec(num_mods, Input(new EModuleConfigBundle(cfg)))
// val init = Output(Bool())
// val insts = Vec(num_mods, Flipped(Decoupled(Instruction(cfg))))

// val run = Input(Bool())
// val io = Vec(num_mods, new EModuleIOBitsBundle(cfg))

}
