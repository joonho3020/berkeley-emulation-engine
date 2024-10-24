package emulator


import chisel3._
import _root_.circt.stage.ChiselStage

object Main extends App {
  val config = new EmulatorConfig
  ChiselStage.emitSystemVerilogFile(
    new Board(config),
    firtoolOpts = Array(
      "-disable-all-randomization",
      "-strip-debug-info",
      "--lowering-options=disallowLocalVariables,noAlwaysComb,verifLabels,disallowPortDeclSharing"))
}
