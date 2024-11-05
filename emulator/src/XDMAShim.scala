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
  axil_params: AXI4BundleParameters,
  emul_params: EmulatorConfig
) extends Module {
  import emul_params._

  val io = IO(new Bundle {
    val m_axil = Flipped(new AXI4Bundle(axil_params))
    val cfg_in = Vec(num_mods, Output(new EModuleConfigBundle(emul_params)))
    val init   = Input(Bool())
  })

  // TODO: Fill this in according to the AXI4 address

}

class BoardDMAModule(
  axi_params: AXI4BundleParameters,
  emul_params: EmulatorConfig
) extends Module {
  import emul_params._

  val io = IO(new Bundle {
    val m_axi      = Flipped(new AXI4Bundle(axi_params))

    val host_steps = Input(UInt(emul_params.index_bits.W))
    val insts = Vec(num_mods, Decoupled(Instruction(emul_params)))
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
    val m_axil = Flipped(new AXI4Bundle(axil_params))
    val m_axi  = Flipped(new AXI4Bundle(axi_params))
  })

  val mmio_bridge = Module(new BoardMMIOModule(axil_params, emul_params))
  mmio_bridge.io.m_axil <> io.m_axil

  val dma_bridge = Module(new BoardDMAModule(axi_params, emul_params))
  dma_bridge.io.m_axi <> io.m_axi

  val board = Module(new Board(emul_params))

  board.io.io    <> dma_bridge.io.io
  board.io.run   := dma_bridge.io.run
  board.io.insts <> dma_bridge.io.insts
}

