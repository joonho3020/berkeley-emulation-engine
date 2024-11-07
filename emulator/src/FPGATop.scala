package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.diplomacy._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}

case class FPGATopConfig(
  axi:  AXI4BundleParameters,
  axil: AXI4BundleParameters)

case object FPGAConfigKey extends Field[FPGATopConfig]

class FPGATop(implicit p: Parameters) extends LazyModule {
  val cfg = p(FPGAConfigKey)

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

  AXI4Buffer() := axiNode

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopConfig) extends LazyModuleImp(outer) {
  val (io_in, edgesIn) = outer.ioDMA.out.unzip
  println(io_in)
  println(edgesIn)

}
