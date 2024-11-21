package emulator

import chisel3._
import _root_.circt.stage.ChiselStage
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.amba.axi4.AXI4BundleParameters
import freechips.rocketchip.diplomacy._
import chisel3.stage.ChiselGeneratorAnnotation
import circt.stage.{ChiselStage, FirtoolOption}
import scala.collection.mutable.ListBuffer
import java.io.{BufferedWriter, FileWriter}
import java.nio.file.Paths;
import java.nio.file.Files;

object Main {
  def makeTop(fpgatop_params: FPGATopParams): LazyModule = {
    implicit val p: Parameters = Parameters((site, here, up) => {
      case FPGATopConfigKey => fpgatop_params
    })

    val fpgatop = LazyModule(new FPGATop)
    return fpgatop
  }

  def main(args: Array[String]): Unit = {
    if (args.contains("--help")) {
      println("""Usage: Main
        [--o         x]
        [--debug     x]
        [--max-step  x]
        [--num-mods  x]
        [--num-procs x]
        [--imem-lat  x]
        [--inter-proc-nw-lat     x]
        [--inter-mod-nw-lat  x]
        [--sram-width    x]
        [--sram-entries  x]
        [--blackbox-dmem  x]
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
    var blackbox_dmem: Boolean = false

    args.sliding(2, 2).toList.collect {
      case Array("--debug",             x) => debug     = x.toBoolean
      case Array("--max-steps",         x) => max_steps = x.toInt
      case Array("--num-mods",          x) => num_mods  = x.toInt
      case Array("--num-procs",         x) => num_procs = x.toInt
      case Array("--imem-lat",          x) => imem_lat  = x.toInt
      case Array("--inter-proc-nw-lat", x) => inter_proc_nw_lat = x.toInt
      case Array("--inter-mod-nw-lat",  x) => inter_mod_nw_lat  = x.toInt
      case Array("--sram-width",        x) => sram_width   = x.toInt
      case Array("--sram-entries",      x) => sram_entries = x.toInt
      case Array("--blackbox-dmem",     x) => blackbox_dmem = x.toBoolean
    }

    val cfg = FPGATopParams(
          debug = debug,
          FPGATopAXI4DMAParams (64, 512,  4, None),
          FPGATopAXI4MMIOParams(25,  32, 12, None),
          EmulatorConfig(
            max_steps = max_steps,
            num_procs = num_procs,
            num_mods  = num_mods,
            imem_lat  = imem_lat,
            inter_mod_nw_lat = inter_mod_nw_lat,
            inter_proc_nw_lat = inter_proc_nw_lat,
            sram_width = sram_width,
            sram_entries = sram_entries,
            blackbox_dmem = blackbox_dmem,
            debug = false
          )
        )

    val lzy = makeTop(cfg)

    Files.createDirectories(Paths.get(cfg.outdir));

    val anno_seq = (new ChiselStage).execute(
      Array("--target", "systemverilog"),
      Seq(ChiselGeneratorAnnotation(() => lzy.module),
        FirtoolOption("--disable-all-randomization"),
        FirtoolOption("-strip-debug-info"),
        FirtoolOption("--lowering-options=disallowLocalVariables,noAlwaysComb,verifLabels,disallowPortDeclSharing"),
        FirtoolOption("--disable-annotation-unknown"),
        FirtoolOption("--disable-annotation-classless"),
        FirtoolOption("--export-module-hierarchy"),
        FirtoolOption("--annotation-file=annos.json"),
        FirtoolOption("--split-verilog"),
        FirtoolOption("-o"),
        FirtoolOption(cfg.outdir),
      ))

// val file = new BufferedWriter(new FileWriter(s"${cfg.outdir}/FPGATop.anno"))
// file.write(
// anno_seq.filter(_ match {
// case SRAMProcessorAnno(target, string) => true
// case _ => false
// }).toSeq.toString()
// )
// file.close()
  }
}
