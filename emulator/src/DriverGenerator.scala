package emulator
import scala.collection.mutable.ListBuffer
import java.io.{BufferedWriter, FileWriter}

abstract class MMap {
  def str: String
}

class DMAIf(
  val addr: Int,
  val filled: Option[Int],
  val empty: Option[Int],
  val name: String
) extends MMap {
    def str: String = {
      val if_type = (filled, empty) match {
        case (Some(f), Some(e)) => {
          "PushPullDMAIf"
        }
        case (Some(f), None) => {
          "PullDMAIf"
        }
        case (None, Some(e)) => {
          "PushDMAIf"
        }
        case _ => {
          assert(false)
        }
      }
      var ret = s"${name}: ${if_type}::new(0x${Integer.toHexString(addr)}"

      filled match {
        case Some(f) => ret += s", 0x${Integer.toHexString(f)}"
        case None    => ()
      }
      empty match {
        case Some(e) => ret += s", 0x${Integer.toHexString(e)}"
        case None    => ()
      }
      ret += ")"
      return ret
    }
}

class MMIOIf(
  val addr: Int,
  val read: Boolean,
  val write: Boolean,
  val name: String
) extends MMap {
  def str: String = {
      val mmio_type = (read, write) match {
        case (true, true) => {
          "RdWrMMIOIf"
        }
        case (true, false) => {
          "RdMMIOIf"
        }
        case (false, true) => {
          "WrMMIOIf"
        }
        case _ => {
          assert(false)
        }
      }
      return s"${name}: ${mmio_type}::new(0x${Integer.toHexString(addr)})"
  }
}

case class SRAMConfigAddr(ptype: Int, mask: Int, width: Int)

class SRAMConfigVecIf(val cfgs: Seq[SRAMConfigAddr]) extends MMap {
  def str: String = {
    var ret = "vec![\n"
    cfgs.zipWithIndex.foreach({ case (cfg, i) => {
      ret += s"          SRAMConfig::new(${cfg.ptype}, ${cfg.mask}, ${cfg.width})"
      if (i != cfgs.length - 1) {
        ret += ",\n"
      }
    }})
    ret += "\n        ]"
    return ret
  }
}

case class DebugInitCntrAddr(addr: Int)

class DebugProcInitCntIf(val cntrs: Seq[DebugInitCntrAddr]) extends MMap {
  def str: String = {
    var ret = "vec![\n"
    cntrs.zipWithIndex.foreach({ case (cntr, i) => {
      ret += s"          RdMMIOIf::new(0x${Integer.toHexString(cntr.addr)})"
      if (i != cntrs.length - 1) {
        ret += ",\n"
      }
    }})
    ret += "\n        ]"
    return ret
  }
}

class ControlIf(val name: String) extends MMap {
  var regs = ListBuffer[MMIOIf]()
  var srams = ListBuffer[SRAMConfigAddr]()
  var dbg_init_cntr = ListBuffer[DebugInitCntrAddr]()

  def add_reg(r: MMIOIf): Unit = {
    regs.append(r)
  }

  def add_sram(s: SRAMConfigAddr): Unit = {
    srams.append(s)
  }

  def sram_cfg_vec: SRAMConfigVecIf = {
    new SRAMConfigVecIf(srams.toSeq)
  }

  def add_dbg_mmio(addr: Int): Unit = {
    dbg_init_cntr.append(DebugInitCntrAddr(addr))
  }

  def dbg_cfg_vec: DebugProcInitCntIf = {
    new DebugProcInitCntIf(dbg_init_cntr.toSeq)
  }

  def str: String = {
    val sram = sram_cfg_vec
    val dbg  = dbg_cfg_vec

    var ret = s"""${name}: ControlIf {
        sram: ${sram.str},
        dbg_init_cntrs: ${dbg.str},"""
    regs.foreach(r => {
      ret += s"""
        ${r.str},"""
    })
    ret += "\n      }\n"
    return ret
  }
}

class ClockWizardControlIf extends MMap {
  def str: String = {
    var ret = s"""clkwiz_ctrl: ClockWizardControlIf {
           pll_locked: RdMMIOIf::new(0x10000),
           pll_reset:  WrMMIOIf::new(0x10004),
           fpga_top_resetn:  WrMMIOIf::new(0x10008),
           fingerprint:  RdWrMMIOIf::new(0x1000c),
           pll_reset_cycle:  WrMMIOIf::new(0x10010),"""
    ret += "\n      }\n"
    return ret
  }
}

class DriverMemoryMap extends MMap {
  var dmas  = ListBuffer[DMAIf]()
  var ctrl = new ControlIf("ctrl_bridge")

  val clkwiz_ctrl = new ClockWizardControlIf

  def str: String = {
    var ret = s"""
    impl Driver {
      pub fn try_from_simif(simif: Box<dyn SimIf>) -> Self {
        Self {
          simif: simif,"""
    dmas.foreach(dma => {
      ret += s"""
      ${dma.str},"""
    })
    ret += "\n"
    ret += clkwiz_ctrl.str
    ret += ",\n"
    ret += "      " + ctrl.str
    ret += "    }\n"
    ret += "  }\n"
    ret += "}"

    return ret
  }

  def write_to_file(out: String): Unit = {
    val file = new BufferedWriter(new FileWriter(out))
    file.write(str)
    file.close()
  }
}
