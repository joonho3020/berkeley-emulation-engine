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

  val dma_axi4_slave = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
  dma_axi4_slave <> outer.axiDMASlaveNode.in.head._1

  val dma_loopback = Module(new AXI4LoopBack(cfg.axi.axi4BundleParams))
  dma_loopback.io.axi <> dma_axi4_slave
  dma_loopback.io.deq.ready := true.B
  dma_loopback.io.enq.valid := true.B
  dma_loopback.io.enq.bits  := BigInt("FEADCAFE", 16).U(cfg.axi.axi4BundleParams.dataBits.W)

  ////////////////////////////////////////////////////////////////////////////
  // MMIO
  ////////////////////////////////////////////////////////////////////////////

  val io_mmio_axi4_master = IO(Flipped(AXI4Bundle(cfg.axil.axi4BundleParams)))
  outer.axiMMIOMasterNode.out.head._1 <> io_mmio_axi4_master
  dontTouch(io_mmio_axi4_master)

  val mmio_axi4_slave = Wire(AXI4Bundle(cfg.axil.axi4BundleParams))
  mmio_axi4_slave <> outer.axiMMIOSlaveNode.in.head._1

  val mmio_loopback = Module(new AXI4LoopBack(cfg.axil.axi4BundleParams))
  mmio_loopback.io.axi <> mmio_axi4_slave
  mmio_loopback.io.deq.ready := true.B
  mmio_loopback.io.enq.valid := true.B
  mmio_loopback.io.enq.bits  := BigInt("FEADCAFE", 16).U(cfg.axil.axi4BundleParams.dataBits.W)
}
