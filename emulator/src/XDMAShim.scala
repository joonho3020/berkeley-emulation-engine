package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental.hierarchy.{Definition, Instance}


// class BoardBundle(cfg: EmulatorConfig) extends Bundle {
// import cfg._

// val cfg_in = Vec(num_mods, Input(new EModuleConfigBundle(cfg))) -> MMIO
// val init = Output(Bool()) -> MMIO
// val insts = Vec(num_mods, Flipped(Decoupled(Instruction(cfg)))) -> DMA

// val run = Input(Bool()) -> ...
// val io = Vec(num_mods, new EModuleIOBitsBundle(cfg))  -> DMA
// val dbg = if (cfg.debug) Some(new BoardDebugBundle(cfg)) else None
// }


class BoardMMIOModule(
  nasti_params: NastiParameters,
  emul_params: EmulatorConfig
) extends Module {
  import emul_params._

  val io = IO(new Bundle {
    val m_nasti = Flipped(new NastiIO(nasti_params))
    val cfg_in = Vec(num_mods, Output(new EModuleConfigBundle(emul_params)))
    val init   = Input(Bool())
  })

  // TODO: Fill this in according to the AXI4 address
  val mcr = Module(new MCRFile(4 * num_mods + 2)(nasti_params))
  mcr.io.nasti <> io.m_nasti

  // Write Only Register mapping
  // - used_procs (0~num_mods-1)
  // - single_port_ram (0~num_mods-1)
  // - wmask_bits (0~num_mods-1)
  // - width_bits (0~num_mods-1)
  // - host_steps

  val num_mods_log2 = log2Ceil(emul_params.num_mods + 1)

  val used_procs = Seq.fill(emul_params.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(used_procs, mcr, 0)

  val single_port_ram = Seq.fill(emul_params.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(single_port_ram, mcr, num_mods)

  val wmask_bits = Seq.fill(emul_params.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(wmask_bits, mcr, num_mods)

  val width_bits = Seq.fill(emul_params.num_mods)(RegInit(0.U(num_mods_log2.W)))
  MCRFile.bind_writeonly_reg_array(width_bits, mcr, num_mods)

  val host_steps = RegInit(0.U(num_bits.W))
  val host_steps_w = Wire(host_steps.cloneType)
  MCRFile.bind_writeonly_reg()

  // Read Only Register mapping
  // - init
  val init = Reg(io.init)
  MCRFile.bind_readonly_reg(init, mcr, 4 * num_mods + 1)
}

class BoardDMAModule(
  nasti_params: NastiParameters,
  emul_params: EmulatorConfig
) extends Module {
  import emul_params._

  val io = IO(new Bundle {
    val m_nasti    = Flipped(new NastiIO(nasti_params))
    val host_steps = Input(UInt(emul_params.index_bits.W))
    val insts      = Vec(num_mods, Decoupled(Instruction(emul_params)))
    val run        = Output(Bool())
    val io         = Vec(num_mods, Flipped(new EModuleIOBitsBundle(emul_params)))
  })

  // TODO: ...
}

class XDMAShim(
  axil_params: AXI4BundleParameters,
  axi_params: AXI4BundleParameters,
  emul_params: EmulatorConfig
) extends Module {
  val io = IO(new Bundle {
    val m_axi_lite = Flipped(new AXI4Bundle(axil_params))
    val m_axi  = Flipped(new AXI4Bundle(axi_params))
  })

  val nasti_lite_params = NastiParameters(axil_params.dataBits, axil_params.addrBits, axil_params.idBits)
  val m_nasti_lite = Wire(new NastiIO(nasti_lite_params))
  AXI4NastiAssigner.toNasti(m_nasti_lite, io.m_axi_lite)

  val mmio_bridge = Module(new BoardMMIOModule(nasti_lite_params, emul_params))
  mmio_bridge.io.m_nasti <> m_nasti_lite

  val nasti_params = NastiParameters(axi_params.dataBits, axi_params.addrBits, axi_params.idBits)
  val m_nasti = Wire(new NastiIO(nasti_params))
  AXI4NastiAssigner.toNasti(m_nasti, io.m_axi)

  val dma_bridge = Module(new BoardDMAModule(nasti_params, emul_params))
  dma_bridge.io.m_nasti <> m_nasti 

  val board = Module(new Board(emul_params))

  board.io.io    <> dma_bridge.io.io
  board.io.run   := dma_bridge.io.run
  board.io.insts <> dma_bridge.io.insts
}

