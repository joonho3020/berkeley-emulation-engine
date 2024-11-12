package emulator

import chisel3._
import _root_.circt.stage.ChiselStage
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.amba.axi4.AXI4BundleParameters
import freechips.rocketchip.diplomacy._

object Main {
  def main(args: Array[String]): Unit = {

    if (args.contains("--help")) {
      println("""Usage: Main
        [--debug     x]
        [--max-step  x]
        [--num-mods  x]
        [--num-procs x]
        [--imem-lat  x]
        [--inter-proc-nw-lat     x]
        [--inter-mod-nw-lat-lat  x]
        [--sram-width    x]
        [--sram-entries  x]
        """)
      System.exit(0)
    }

    var debug: Boolean   = false
    var max_steps:   Int = 128
    var num_procs:   Int = 8
    var num_mods:    Int = 9
    var imem_lat:    Int = 1
    var num_prims:   Int = 9
    var inter_proc_nw_lat: Int = 0
    var inter_mod_nw_lat:  Int = 0
    var sram_width:   Int = 16
    var sram_entries: Int = 16

    args.sliding(9, 9).toList.collect {
      case Array("--debug",                x) => debug     = x.toBoolean
      case Array("--max-steps",            x) => max_steps = x.toInt
      case Array("--num-mods",             x) => num_mods  = x.toInt
      case Array("--num-procs",            x) => num_procs = x.toInt
      case Array("--imem-lat",             x) => imem_lat  = x.toInt
      case Array("--inter-proc-nw-lat",    x) => inter_proc_nw_lat = x.toInt
      case Array("--inter-mod-nw-lat-lat", x) => inter_mod_nw_lat  = x.toInt
      case Array("--sram-width",           x) => sram_width   = x.toInt
      case Array("--sram-entries",         x) => sram_entries = x.toInt
    }

    implicit val p: Parameters = Parameters((site, here, up) => {
      case FPGATopConfigKey =>
        FPGATopParams(
          debug = true,
          FPGATopAXI4DMAParams (64, 512, 4, None),
          FPGATopAXI4MMIOParams(64,  32, 4, None),
          EmulatorConfig(
            max_steps = max_steps,
            num_procs = num_procs,
            num_mods  = num_mods,
            imem_lat  = imem_lat,
            inter_mod_nw_lat = inter_mod_nw_lat,
            inter_proc_nw_lat = inter_proc_nw_lat,
            sram_width = sram_width,
            sram_entries = sram_entries,
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

}
