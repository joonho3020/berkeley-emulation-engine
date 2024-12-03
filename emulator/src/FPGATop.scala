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
        address = Seq(AddressSet(0, (BigInt(1) << 16) - 1)),
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
    val tot_pushed      = Output(UInt(log2Ceil(cfg.emul.insts_per_mod * cfg.emul.num_mods + 1).W))
    val proc_0_init_vec = Output(UInt(cfg.emul.num_mods.W))
    val proc_n_init_vec = Output(UInt(cfg.emul.num_mods.W))
  })

  dontTouch(io_dma_axi4_master)
  dontTouch(io_dma_axi4_slave)

  val total_procs = cfg.emul.num_procs * cfg.emul.num_mods
  val dataBits = cfg.axi.axi4BundleParams.dataBits
  val io_stream_width = (((total_procs + dataBits - 1) / dataBits) * dataBits).toInt
  val dbg_stream_width = (((total_procs * 2 + dataBits - 1) / dataBits) * dataBits).toInt
  println(s"io_stream_width: ${io_stream_width}")
  println(s"dbg_stream_width: ${dbg_stream_width}")
  println(s"total_procs: ${total_procs}")

  // TODO : Change streamParams to Map for better indexing?
  val stream_converter = Module(new AXI4DecoupledConverter(
    axiParams = cfg.axi.axi4BundleParams,
    streamParams = Seq(
      StreamParam(io_stream_width, io_stream_width / dataBits * 2),
      StreamParam(cfg.axi.axi4BundleParams.dataBits, 128),
      StreamParam(io_stream_width, io_stream_width / dataBits * 2),
      StreamParam(dbg_stream_width, 2 * cfg.emul.max_steps)
    ),
    addressSpaceBits = 12))

  stream_converter.io.axi <> io_dma_axi4_slave

  stream_converter.io.streams(1).enq.valid := false.B
  stream_converter.io.streams(1).enq.bits  := 0.U
  stream_converter.io.streams(1).deq.ready := false.B

  stream_converter.io.streams(3).enq.valid := false.B
  stream_converter.io.streams(3).enq.bits  := 0.U
  stream_converter.io.streams(3).deq.ready := false.B

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

  val max_mmio_regs = 4 * cfg.emul.num_mods + 26

  val mmio = Module(new AXI4MMIOModule(max_mmio_regs, cfg.axil.axi4BundleParams))
  AXI4MMIOModule.tieoff(mmio)
  dontTouch(mmio.io.axi)

  mmio.io.axi <> mmio_axi4_slave

  val custom_resetn = RegInit(false.B)
  mmap.ctrl.add_reg(new MMIOIf(
    AXI4MMIOModule.bind_writeonly_reg(custom_resetn, mmio) << 2,
    false,
    true,
    "custom_resetn"))

  withReset (!custom_resetn.asBool) {
    val num_mods_log2 = log2Ceil(cfg.emul.num_mods + 1)

    require(cfg.axil.dataBits >= cfg.emul.sram_width_bits)
    val single_port_ram = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(cfg.axil.dataBits.W)))
    val wmask_bits      = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(cfg.axil.dataBits.W)))
    val width_bits      = Seq.fill(cfg.emul.num_mods)(RegInit(0.U(cfg.axil.dataBits.W)))

    val ptype_idxs = AXI4MMIOModule.bind_readwrite_reg_array(single_port_ram, mmio)
    val mask_idxs  = AXI4MMIOModule.bind_readwrite_reg_array(wmask_bits,      mmio)
    val width_idxs = AXI4MMIOModule.bind_readwrite_reg_array(width_bits,      mmio)

    ptype_idxs.zip(mask_idxs).zip(width_idxs).foreach({ case((p, m), w) => {
      mmap.ctrl.add_sram(SRAMConfigAddr(p << 2, m << 2, w << 2))
    }})

    val fingerprint_reg = RegInit(BigInt("F00DCAFE", 16).U(32.W))
    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readwrite_reg(fingerprint_reg, mmio) << 2,
      true,
      true,
      "fingerprint"))

    val host_steps = RegInit(0.U(cfg.emul.index_bits.W))
    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readwrite_reg(host_steps, mmio) << 2,
      true,
      true,
      "host_steps"))

    val host_steps_prv = RegNext(host_steps)
    val host_steps_prv_q = Module(new Queue(UInt(cfg.emul.index_bits.W), 4))
    val host_steps_cur_q  = Module(new Queue(UInt(cfg.emul.index_bits.W), 4))

    host_steps_prv_q.io.enq.valid := host_steps_prv =/= host_steps
    host_steps_prv_q.io.enq.bits  := host_steps_prv

    host_steps_cur_q.io.enq.valid := host_steps_prv =/= host_steps
    host_steps_cur_q.io.enq.bits  := host_steps

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_decoupled_read(host_steps_prv_q.io.deq, mmio) << 2,
      true,
      false,
      "host_steps_prv_deq"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readonly_reg(host_steps_prv_q.io.count, mmio) << 2,
      true,
      false,
      "host_steps_prv_cnt"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_decoupled_read(host_steps_cur_q.io.deq, mmio) << 2,
      true,
      false,
      "host_steps_cur_deq"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readonly_reg(host_steps_cur_q.io.count, mmio) << 2,
      true,
      false,
      "host_steps_cur_cnt"))

    ////////////////////////////////////////////////////////////////////////////

    val board = Module(new Board(cfg.emul))

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

    val tot_insts_pushed = RegInit(0.U(log2Ceil(cfg.emul.insts_per_mod * cfg.emul.num_mods + 1).W))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readwrite_reg(tot_insts_pushed, mmio) << 2,
      true,
      false,
      "tot_insts_pushed"))

    val pcs_are_zero = RegNext(board.io.dbg_pcs_are_zero_vec)
    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readonly_reg(pcs_are_zero, mmio) << 2,
      true,
      false,
      "pcs_are_zero"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readwrite_reg(RegNext(board.io.dbg_proc_0_init), mmio) << 2,
      true,
      false,
      "dbg_proc_0_init"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readwrite_reg(RegNext(board.io.dbg_proc_n_init), mmio) << 2,
      true,
      false,
      "dbg_proc_n_init"))

    io_debug.tot_pushed      := tot_insts_pushed
    io_debug.proc_0_init_vec := board.io.dbg_proc_0_init
    io_debug.proc_n_init_vec := board.io.dbg_proc_n_init

    // TODO: make this into parallel streams to make the loading faster(?)
    board.io.inst.bits  := stream_converter.io.streams(1).deq.bits.asTypeOf(new BoardInstInitBundle(cfg.emul))
    board.io.inst.valid := stream_converter.io.streams(1).deq.valid
    stream_converter.io.streams(1).deq.ready := board.io.inst.ready

    val expect_midx = RegInit(0.U(log2Ceil(cfg.emul.num_mods).W))
    val expect_pidx = RegInit(0.U(log2Ceil(cfg.emul.num_procs).W))
    val inst_cntr   = RegInit(0.U(cfg.emul.index_bits.W))

    when (stream_converter.io.streams(1).deq.fire) {
      tot_insts_pushed := tot_insts_pushed + 1.U

      when (inst_cntr === host_steps - 1.U) {
        inst_cntr := 0.U
        when (expect_pidx === (cfg.emul.num_procs - 1).U) {
          expect_pidx := 0.U
          expect_midx := expect_midx + 1.U
          } .otherwise {
            expect_pidx := expect_pidx + 1.U
          }
          } .otherwise {
            inst_cntr := inst_cntr + 1.U
          }
    }

    val midx_mismatch_q = Module(new Queue(UInt(log2Ceil(cfg.emul.num_mods).W), 4))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_decoupled_read(midx_mismatch_q.io.deq, mmio) << 2,
      true,
      false,
      "midx_mismatch_deq"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readonly_reg(midx_mismatch_q.io.count, mmio) << 2,
      true,
      false,
      "midx_mismatch_cnt"))

    midx_mismatch_q.io.enq.valid := (expect_midx =/= board.io.inst.bits.midx) &&
    board.io.inst.fire
    midx_mismatch_q.io.enq.bits := board.io.inst.bits.midx

    val pidx_mismatch_q = Module(new Queue(UInt(log2Ceil(cfg.emul.num_procs).W), 4))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_decoupled_read(pidx_mismatch_q.io.deq, mmio) << 2,
      true,
      false,
      "pidx_mismatch_deq"))

    mmap.ctrl.add_reg(new MMIOIf(
      AXI4MMIOModule.bind_readonly_reg(pidx_mismatch_q.io.count, mmio) << 2,
      true,
      false,
      "pidx_mismatch_cnt"))

    pidx_mismatch_q.io.enq.valid := (expect_pidx =/= board.io.inst.bits.inst.pidx) &&
    board.io.inst.fire
    pidx_mismatch_q.io.enq.bits := board.io.inst.bits.inst.pidx

    val dbg_proc_init_cnt = board.io.dbg_proc_init_cnt.map(dpic => {
      RegNext(dpic)
    })

    val dbg_proc_init_idx = AXI4MMIOModule.bind_readonly_reg_array(dbg_proc_init_cnt, mmio)
    dbg_proc_init_idx.foreach(dpi_idx => {
      mmap.ctrl.add_dbg_mmio(dpi_idx << 2)
    })

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

    mmap.dmas.append(new DMAIf(
      0x1000,
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(1).filled_bytes, mmio) << 2),
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(1) .empty_bytes, mmio) << 2),
      "inst_bridge"))

    mmap.dmas.append(new DMAIf(
      0x2000,
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(2).filled_bytes, mmio) << 2),
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(2) .empty_bytes, mmio) << 2),
      "dma_bridge"))

    val dma_test_q = Module(new Queue(UInt(io_stream_width.W), 4))
    dma_test_q.io.enq <> stream_converter.io.streams(2).deq
    stream_converter.io.streams(2).enq <> dma_test_q.io.deq

    mmap.dmas.append(new DMAIf(
      0x3000,
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(3).filled_bytes, mmio) << 2),
      Some(AXI4MMIOModule.bind_readonly_reg(stream_converter.io.streams(3) .empty_bytes, mmio) << 2),
      "dbg_bridge"))

    board.io.dbg.map(x => {
      require(cfg.emul.num_bits == 1)
      println("Connecting dbg_bridge")

      val ldm_state_at_step = Wire(Vec(cfg.emul.num_mods * cfg.emul.num_procs, Bool()))
      val sdm_state_at_step = Wire(Vec(cfg.emul.num_mods * cfg.emul.num_procs, Bool()))
      for (i <- 0 until cfg.emul.num_mods) {
        for (j <- 0 until cfg.emul.num_procs) {
          ldm_state_at_step(cfg.emul.num_procs * i + j) := x.bdbg(i).pdbg(j).ldm
          sdm_state_at_step(cfg.emul.num_procs * i + j) := x.bdbg(i).pdbg(j).sdm
        }
      }

      stream_converter.io.streams(3).enq.valid := board.io.run
      stream_converter.io.streams(3).enq.bits  := Cat(ldm_state_at_step.zip(sdm_state_at_step).map({ case (l, s) => Cat(s, l) }).reverse)
      assert(stream_converter.io.streams(3).enq.ready === true.B)
    })
  }

  val io_clkwiz_ctrl = IO(new Bundle {
    val axi_aclk    = Input(Bool())
    val axi_aresetn = Input(Bool())
    val ctrl        = new ClockWizardControllerBundle(cfg)
  })

  withClockAndReset(io_clkwiz_ctrl.axi_aclk.asClock, !io_clkwiz_ctrl.axi_aresetn) {
    val clkwiz_ctrl = Module(new ClockWizardController(cfg))
    clkwiz_ctrl.io <> io_clkwiz_ctrl.ctrl
  }

  println(s"""=================== Simulator Memory Map =========================
    ${mmap.str}
  """)
  mmap.write_to_file(s"${cfg.outdir}/FPGATop.mmap")

}
