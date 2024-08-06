package emulator


import chisel3._
import _root_.circt.stage.ChiselStage

object Main extends App {
  ChiselStage.emitSystemVerilogFile(
    new OpalKellyEmulatorModuleTop(new ModuleConfig, new OpalKellyConfig),
    firtoolOpts = Array(
      "-disable-all-randomization",
      "-strip-debug-info",
      "--lowering-options=disallowLocalVariables,noAlwaysComb,verifLabels,disallowPortDeclSharing"))
}
