package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._

case class FPGATopConfig(
  axi:  AXI4BundleParameters,
  axil: AXI4BundleParameters)

case object FPGAConfigKey extends Field[FPGATopConfig]

class FPGATop(implicit p: Parameters) extends LazyModule {
  val cfg = p(FPGAConfigKey)

  println(cfg)

   // AXI4 Master Node with a single master port
  val axiNode = AXI4MasterNode(Seq(AXI4MasterPortParameters(
    masters = Seq(AXI4MasterParameters(name = "AXI4Master"))
  )))

  val ioDMA = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves = Seq(AXI4SlaveParameters(
        address = Seq(AddressSet(0, (BigInt(1) << cfg.axi.addrBits) - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  ioDMA := AXI4Buffer() := axiNode

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopConfig) extends LazyModuleImp(outer) {

  println(cfg.axi)


  val io_axi4 = IO(Flipped(AXI4Bundle(cfg.axi)))
  outer.axiNode.out.head._1 <> io_axi4

  val io_if = IO(Flipped(AXI4Bundle(cfg.axi)))
  io_if <> outer.ioDMA.in.head._1

  dontTouch(io_if)

  val gcd = Module(new GCDWithAXI(cfg.axi))
  gcd.io.axi <> io_if
}
