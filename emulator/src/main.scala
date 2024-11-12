package emulator

import chisel3._
import _root_.circt.stage.ChiselStage
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.amba.axi4.AXI4BundleParameters
import freechips.rocketchip.diplomacy._

object Main extends App {
  implicit val p: Parameters = Parameters((site, here, up) => {
    case FPGATopConfigKey =>
      FPGATopParams(
        debug = true,
        FPGATopAXI4DMAParams (64, 512, 4, None),
        FPGATopAXI4MMIOParams(64,  32, 4, None),
        EmulatorConfig(
          debug = false
        )
      )
  })

  val fpgatop = LazyModule(new FPGATop)
  ChiselStage.emitSystemVerilogFile(
    fpgatop.module,
    firtoolOpts = Array(
      "-disable-all-randomization",
      "-strip-debug-info",
      "--lowering-options=disallowLocalVariables,noAlwaysComb,verifLabels,disallowPortDeclSharing"))
}
