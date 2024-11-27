package emulator

import chisel3._
import chisel3.util._
import freechips.rocketchip.amba.axi4._
import org.chipsalliance.cde.config.{Field, Parameters}
import freechips.rocketchip.diplomacy._
import freechips.rocketchip.util.DecoupledHelper
import java.io._

case class FPGATopAXI4DMAParams(
  addrBits:  Int,
  dataBits:  Int,
  idBits:    Int,
  maxFlight: Option[Int] = None,
) {
  def axi4BundleParams = AXI4BundleParameters(
    addrBits = addrBits,
    dataBits = dataBits,
    idBits   = idBits,
  )
}

case class FPGATopAXI4MMIOParams(
  addrBits:  Int,
  dataBits:  Int,
  idBits:    Int,
  maxFlight: Option[Int] = None,
) {
  def axi4BundleParams = AXI4BundleParameters(
    addrBits = addrBits,
    dataBits = dataBits,
    idBits   = idBits,
  )
}

case class FPGATopParams(
  // Adds a extra DMA stream engine to check for XDMA DMA transactions
  debug: Boolean,

  // XDMA AXI4 parameters for DMA
  axi:  FPGATopAXI4DMAParams,

  // XDMA AXI4-lite parameters for MMIO
  axil: FPGATopAXI4MMIOParams,

  // Emulation platform configuration
  emul: EmulatorConfig
) {
  def outdir: String = s"generated-${emul.str}"
}

case object FPGATopConfigKey extends Field[FPGATopParams]

class FPGATop(implicit p: Parameters) extends LazyModule {
  val cfg = p(FPGATopConfigKey)

  println("================= Emulator configuration =======================");
  println(pprint.tokenize(cfg).mkString)
  println("================================================================");

   // AXI4 Master Node with a single master port
  val axiDMAMasterNode = AXI4MasterNode(Seq(
    AXI4MasterPortParameters(
      masters = Seq(AXI4MasterParameters(
        name      = "cpu-managed-axi4",
        id        = IdRange(0, 1 << cfg.axi.idBits),
        aligned   = false,
        // None = infinite, else is a per-ID cap
        maxFlight = cfg.axi.maxFlight)
      ))))

  val axiDMASlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves    = Seq(
        AXI4SlaveParameters(
          address       = Seq(AddressSet(0, (BigInt(1) << cfg.axi.addrBits) - 1)),
          resources     = (new MemoryDevice).reg,
          regionType    = RegionType.UNCACHED, // cacheable
          executable    = false,
          supportsWrite = TransferSizes(cfg.axi.dataBits / 8, 4096),
          supportsRead  = TransferSizes(cfg.axi.dataBits / 8, 4096),
          interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  axiDMASlaveNode := AXI4Buffer() := axiDMAMasterNode

   // AXI4-Lite Master Node with a single master port
  val axiMMIOMasterNode = AXI4MasterNode(Seq(
    AXI4MasterPortParameters(
      masters = Seq(AXI4MasterParameters(
        name      = "ctrl-axi-lite",
        id        = IdRange(0, 1 << cfg.axil.idBits),
        aligned   = false,
        // None = infinite, else is a per-ID cap
        maxFlight = cfg.axil.maxFlight)
      ))))

  val axiMMIOSlaveNode = AXI4SlaveNode(Seq(
    AXI4SlavePortParameters(
      slaves = Seq(AXI4SlaveParameters(
        address = Seq(AddressSet(0, (BigInt(1) << cfg.axil.addrBits) - 1)),
        resources     = (new MemoryDevice).reg,
        regionType    = RegionType.UNCACHED,
        executable    = false,
        supportsWrite = TransferSizes(cfg.axil.dataBits / 8, 4096),
        supportsRead  = TransferSizes(cfg.axil.dataBits / 8, 4096),
        interleavedId = Some(0))),
      beatBytes = cfg.axi.dataBits / 8)))

  axiMMIOSlaveNode := AXI4Buffer() := axiMMIOMasterNode

  lazy val module = new FPGATopImp(this)(cfg)
}

class FPGATopImp(outer: FPGATop)(cfg: FPGATopParams) extends LazyModuleImp(outer) {
  println(cfg.axi)

  var mmap = new DriverMemoryMap

  val io_dma_axi4_master = IO(Flipped(AXI4Bundle(cfg.axi.axi4BundleParams)))
  outer.axiDMAMasterNode.out.head._1 <> io_dma_axi4_master

  val io_dma_axi4_slave = Wire(AXI4Bundle(cfg.axi.axi4BundleParams))
  io_dma_axi4_slave <> outer.axiDMASlaveNode.in.head._1

  val io_debug = IO(new Bundle {
    val st_val = Output(Bool())
    val st_rdy = Output(Bool())
    val tot_pushed = Output(UInt(log2Ceil(cfg.emul.insts_per_mod * cfg.emul.num_mods + 1).W))
    val cur_mod    = Output(UInt(log2Ceil(cfg.emul.num_mods + 1).W))
    val cur_pushed = Output(UInt(log2Ceil(cfg.emul.insts_per_mod + 1).W))
    val sram_proc_init_vec = Output(UInt(cfg.emul.num_mods.W))
    val proc_0_init_vec = Output(UInt(cfg.emul.num_mods.W))
    val proc_n_init_vec = Output(UInt(cfg.emul.num_mods.W))
  })

  dontTouch(io_dma_axi4_master)
  dontTouch(io_dma_axi4_slave)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val dataBits = cfg.axi.axi4BundleParams.dataBits
  val io_stream_width = (((total_procs + dataBits - 1) / dataBits) * dataBits).toInt
  println(s"io_stream_width: ${io_stream_width}")
  println(s"total_procs: ${total_procs}")

  // TODO : Change streamParams to Map for better indexing?
  val stream_converter = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    streamParams = Seq(
      StreamParam(io_stream_width, io_stream_width / dataBits * 2),
      StreamParam(cfg.axi.axi4BundleParams.dataBits, 128),
      StreamParam(io_stream_width, io_stream_width / dataBits * 2)),
    addressSpaceBits = 12))

  stream_converter.io.axi <> io_dma_axi4_slave

  stream_converter.io.streams(1).enq.valid := false.B
  stream_converter.io.streams(1).enq.bits  := 0.U
  stream_converter.io.streams(1).deq.ready := false.B

  ////////////////////////////////////////////////////////////////////////////
  // MMIO
  ////////////////////////////////////////////////////////////////////////////

  val io_mmio_axi4_master = IO(Flipped(AXI4Bundle(cfg.axil.axi4BundleParams)))
  outer.axiMMIOMasterNode.out.head._1 <> io_mmio_axi4_master
  dontTouch(io_mmio_axi4_master)

  val mmio_axi4_slave = Wire(AXI4Bundle(cfg.axil.axi4BundleParams))
  mmio_axi4_slave <> outer.axiMMIOSlaveNode.in.head._1

  val axil_addr_range = 1 << cfg.axil.axi4BundleParams.addrBits
  val axil_data_byts  = cfg.axil.axi4BundleParams.dataBits / 8

  val max_mmio_regs = 3 * cfg.emul.num_mods + 14

  val mmio = Module(new AXI4MMIOModule(max_mmio_regs, cfg.axil.axi4BundleParams))
  AXI4MMIOModule.tieoff(mmio)
  dontTouch(mmio.io.axi)

  mmio.io.axi <> mmio_axi4_slave

  val num_mods_log2 = log2Ceil(cfg.emul.num_mods + 1)

  val single_port_ram = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  val wmask_bits      = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))
  val width_bits      = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(num_mods_log2.W)))

  val ptype_idxs = AXI4MMIOModule.bind_readwrite_reg_array(single_port_ram, mmio)
  val mask_idxs  = AXI4MMIOModule.bind_readwrite_reg_array(wmask_bits,      mmio)
  val width_idxs = AXI4MMIOModule.bind_readwrite_reg_array(width_bits,      mmio)

  ptype_idxs.zip(mask_idxs).zip(width_idxs).foreach({ case((p, m), w) => {
    mmap.ctrl.add_sram(SRAMConfigAddr(p << 2, m << 2, w << 2))
  }})


  val host_steps = RegInit(0.U(cfg.emul.index_bits.W))
  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readwrite_reg(host_steps, mmio) << 2,
    true,
    true,
    "host_steps"))

  val fingerprint_reg = RegInit(BigInt("F00DCAFE", 16).U(32.W))
  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readwrite_reg(fingerprint_reg, mmio) << 2,
    true,
    true,
    "fingerprint"))

  ////////////////////////////////////////////////////////////////////////////

  val board = Module(new Board(cfg.emul))

  // Read Only Register mapping
  // - init
  val init = RegNext(board.io.init)
  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readonly_reg(init, mmio) << 2,
    true,
    false,
    "init_done"))

  for (i <- 0 until cfg.emul.num_mods) {
    board.io.cfg_in(i).host_steps := host_steps
    board.io.cfg_in(i).sram.single_port_ram := single_port_ram(i)
    board.io.cfg_in(i).sram.wmask_bits      := wmask_bits(i)
    board.io.cfg_in(i).sram.width_bits      := width_bits(i)
  }

  // TODO: make this into parallel streams to make the loading faster(?)
  val cur_inst_mod = RegInit(0.U(log2Ceil(cfg.emul.num_mods + 1).W))
  val cur_insts_pushed = RegInit(0.U(log2Ceil(cfg.emul.insts_per_mod + 1).W))
  val tot_insts_pushed = RegInit(0.U(log2Ceil(cfg.emul.insts_per_mod * cfg.emul.num_mods + 1).W))

  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readwrite_reg(cur_inst_mod, mmio) << 2,
    true,
    false,
    "cur_inst_mod"))

  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readwrite_reg(cur_insts_pushed, mmio) << 2,
    true,
    false,
    "cur_insts_pushed"))

  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readwrite_reg(tot_insts_pushed, mmio) << 2,
    true,
    false,
    "tot_insts_pushed"))

  for (i <- 0 until cfg.emul.num_mods) {
    board.io.insts(i).valid := false.B
    board.io.insts(i).bits  := DontCare
  }

  io_debug.cur_pushed := cur_insts_pushed
  io_debug.tot_pushed := tot_insts_pushed
  io_debug.cur_mod    := cur_inst_mod
  io_debug.st_val := DontCare
  io_debug.st_rdy := DontCare
  io_debug.sram_proc_init_vec := board.io.dbg_sram_init
  io_debug.proc_0_init_vec    := board.io.dbg_proc_0_init
  io_debug.proc_n_init_vec    := board.io.dbg_proc_n_init

  for (i <- 0 until cfg.emul.num_mods) {
    when (i.U === cur_inst_mod) {
      board.io.insts(i).valid := stream_converter.io.streams(1).deq.valid
      board.io.insts(i).bits  := stream_converter.io.streams(1).deq.bits.asTypeOf(Instruction(cfg.emul))
      stream_converter.io.streams(1).deq.ready := board.io.insts(i).ready

      io_debug.st_val := stream_converter.io.streams(1).deq.valid
      io_debug.st_rdy := board.io.insts(i).ready


      when (board.io.insts(i).fire) {
        tot_insts_pushed := tot_insts_pushed + 1.U
        when (cur_insts_pushed === host_steps * cfg.emul.num_procs.U - 1.U) {
          cur_insts_pushed := 0.U
          cur_inst_mod := cur_inst_mod + 1.U
        } .otherwise {
          cur_insts_pushed := cur_insts_pushed + 1.U
        }
      }
    }
  }

  val cur_step = RegInit(0.U(cfg.emul.index_bits.W))
  val target_cycle = RegInit(0.U(64.W))


  val stream_deq_skid_buffer = Module(new SkidBufferChain(stream_converter.io.streams(0).deq.bits.cloneType, 4))
  stream_deq_skid_buffer.io.enq <> stream_converter.io.streams(0).deq

  // TODO: DRAM interface should go here
  for (i <- 0 until cfg.emul.num_mods) {
    for (j <- 0 until cfg.emul.num_procs) {
      val idx = i * cfg.emul.num_procs + j
      board.io.io(i).i(j) := stream_deq_skid_buffer.io.deq.bits >> (idx * cfg.emul.num_bits)
    }
  }

  val stream_enq_skid_buffer = Module(new SkidBufferChain(stream_converter.io.streams(0).enq.bits.cloneType, 4))
  stream_converter.io.streams(0).enq <> stream_enq_skid_buffer.io.deq
  stream_enq_skid_buffer.io.enq.bits := Cat(board.io.io.flatMap(io => io.o).reverse)


  val board_run = DecoupledHelper(
    stream_deq_skid_buffer.io.deq.valid,
    stream_enq_skid_buffer.io.enq.ready)

  val last_step = cur_step === host_steps - 1.U
  board.io.run := board_run.fire()
  stream_deq_skid_buffer.io.deq.ready := board_run.fire(stream_deq_skid_buffer.io.deq.valid, last_step)
  stream_enq_skid_buffer.io.enq.valid := board_run.fire(stream_enq_skid_buffer.io.enq.ready, last_step)

  when (board.io.run) {
    cur_step := Mux(last_step, 0.U, cur_step + 1.U)
  }

  when (stream_converter.io.streams(0).enq.fire) {
    target_cycle := target_cycle + 1.U
  }

  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readonly_reg(target_cycle & ((BigInt(1) << 32) - 1).U, mmio) << 2,
    true,
    false,
    "target_cycle_lo"))

  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_readonly_reg(target_cycle >> 32, mmio) << 2,
    true,
    false,
    "target_cycle_hi"))

  mmap.dmas.append(new DMAIf(
    0x0000,
    Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(0).filled_bytes, mmio) << 2),
    Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(0) .empty_bytes, mmio) << 2),
    "io_bridge"))

  // TODO: remove later, just to keep consistency for now
  AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(1).filled_bytes, mmio)

  mmap.dmas.append(new DMAIf(
    0x1000,
    None,
    Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(1) .empty_bytes, mmio) << 2),
    "inst_bridge"))

  mmap.dmas.append(new DMAIf(
    0x2000,
    Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(2).filled_bytes, mmio) << 2),
    Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(2) .empty_bytes, mmio) << 2),
    "dbg_bridge"))

  val dma_test_q = Module(new Queue(UInt(io_stream_width.W), 4))
  dma_test_q.io.enq <> stream_converter.io.streams(2).deq
  stream_converter.io.streams(2).enq <> dma_test_q.io.deq


  println(s"""=================== Simulator Memory Map =========================
    ${mmap.str}
  """)
  mmap.write_to_file(s"${cfg.outdir}/FPGATop.mmap")
}
