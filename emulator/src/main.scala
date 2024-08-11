package emulator


import chisel3._
import _root_.circt.stage.ChiselStage

object Main extends App {
  val config = new ModuleConfig(max_steps = 8, module_sz = 8)
  val opalkelly = new OpalKellyConfig(wire_bits = 16)

  ChiselStage.emitSystemVerilogFile(
    new OpalKellyFPGATop(config, opalkelly),
    firtoolOpts = Array(
      "-disable-all-randomization",
      "-strip-debug-info",
      "--lowering-options=disallowLocalVariables,noAlwaysComb,verifLabels,disallowPortDeclSharing"))
}
