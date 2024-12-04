package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental._
import freechips.rocketchip.amba.axi4._
import freechips.rocketchip.util.DecoupledHelper

class StreamAdapterIO(val w: Int) extends Bundle {
  val in  = Flipped(Decoupled(UInt(w.W)))
  val out = Decoupled(UInt(w.W))

  def flipConnect(other: StreamAdapterIO): Unit = {
    in       <> other.out
    other.in <> out
  }
}

class StreamWidthAdapter(narrowW: Int, wideW: Int) extends Module {
  require(wideW % narrowW == 0)
  val io = IO(new Bundle {
    val narrow = new StreamAdapterIO(narrowW)
    val wide   = new StreamAdapterIO(wideW)
  })

  if (wideW <= narrowW) {
    io.narrow.out <> io.wide.in
    io.wide.out   <> io.narrow.in
  } else {
    val beats = wideW / narrowW

    val narrow_beats     = RegInit(0.U(log2Ceil(beats).W))
    val narrow_last_beat = narrow_beats === (beats - 1).U
    val narrow_data      = Reg(Vec(beats - 1, UInt(narrowW.W)))

    val wide_beats     = RegInit(0.U(log2Ceil(beats).W))
    val wide_last_beat = wide_beats === (beats - 1).U

    io.narrow.in.ready := Mux(narrow_last_beat, io.wide.out.ready, true.B)
    when(io.narrow.in.fire) {
      narrow_beats := Mux(narrow_last_beat, 0.U, narrow_beats + 1.U)
      when(!narrow_last_beat) { narrow_data(narrow_beats) := io.narrow.in.bits }
    }
    io.wide.out.valid  := narrow_last_beat && io.narrow.in.valid
    io.wide.out.bits   := Cat(io.narrow.in.bits, narrow_data.asUInt)

    io.narrow.out.valid := io.wide.in.valid
    io.narrow.out.bits  := io.wide.in.bits.asTypeOf(Vec(beats, UInt(narrowW.W)))(wide_beats)
    when(io.narrow.out.fire) {
      wide_beats := Mux(wide_last_beat, 0.U, wide_beats + 1.U)
    }
    io.wide.in.ready    := wide_last_beat && io.narrow.out.ready
  }
}

case class StreamParam(
  widthBits: Int,
  bufferDepth: Int
)

class Stream(val sp: StreamParam, val axiDataBits: Int) extends Bundle {
  val widthBytes   = axiDataBits / 8
  val maxBytes     = sp.bufferDepth * widthBytes
  val capacityBits = log2Ceil(maxBytes + 1)
  require(capacityBits <= 32)

  val enq = Flipped(Decoupled(UInt(sp.widthBits.W)))
  val filled_bytes = UInt(capacityBits.W)
  val deq = Decoupled(UInt(sp.widthBits.W))
  val empty_bytes = UInt(capacityBits.W)
}

class AXI4DecoupledConverter(
  axiParams: AXI4BundleParameters,
  streamParams: Seq[StreamParam],
  addressSpaceBits: Int
) extends Module {

  val num_stream = streamParams.length

  val io = IO(new Bundle {
    val axi = Flipped(AXI4Bundle(axiParams))
    val streams = MixedVec(streamParams.map(new Stream(_, axiParams.dataBits)))
  })

  val axiBeatBytes = axiParams.dataBits / 8

  // FromHostCPU streams are implemented using the AW, W, B channels, which
  // write into large BRAM FIFOs for each stream.
  assert(!io.axi.aw.valid || io.axi.aw.bits.size === log2Ceil(axiBeatBytes).U)
  assert(!io.axi. w.valid || io.axi. w.bits.strb === ~0.U(axiBeatBytes.W))

  io.axi.b.bits.resp := 0.U(2.W)
  io.axi.b.bits.id   := io.axi.aw.bits.id
  io.axi.b.bits.user := io.axi.aw.bits.user

  // This will be set by the channel given the grant using last connect semantics
  io.axi.b.valid     := false.B
  io.axi.aw.ready    := false.B
  io.axi.w.ready     := false.B

  def connect_axiw(
    deq: DecoupledIO[UInt],
    empty_bytes: UInt,
    sp: StreamParam,
    idx: Int
  ) = {
    val serdes_deq = Module(new StreamWidthAdapter(axiParams.dataBits, sp.widthBits))
    serdes_deq.io.wide.in.bits     := 0.U
    serdes_deq.io.wide.in.valid    := false.B
    serdes_deq.io.narrow.out.ready := false.B

    val incomingQueueIO = Module(new Queue(UInt(sp.widthBits.W), sp.bufferDepth)).io
    deq <> incomingQueueIO.deq
    incomingQueueIO.enq <> serdes_deq.io.wide.out

    val grant = (io.axi.aw.bits.addr >> addressSpaceBits) === idx.U

    val axiWriteDataQueueIO = Module(new Queue(UInt(axiParams.dataBits.W), sp.bufferDepth)).io

    val writeHelper = DecoupledHelper(
      io.axi.aw.valid,
      io.axi.w.valid,
      io.axi.b.ready,
      axiWriteDataQueueIO.enq.ready,
      grant
    )

    val writeBeatCounter = RegInit(0.U(9.W))
    val lastWriteBeat    = writeBeatCounter === io.axi.aw.bits.len
    when(io.axi.w.fire && grant) {
      writeBeatCounter := Mux(lastWriteBeat, 0.U, writeBeatCounter + 1.U)
    }

    when (grant) {
      io.axi.w.ready  := writeHelper.fire(io.axi.w.valid)
      io.axi.aw.ready := writeHelper.fire(io.axi.aw.valid, lastWriteBeat)
      io.axi.b.valid  := writeHelper.fire(io.axi.b.ready, lastWriteBeat)
    }
    serdes_deq.io.narrow.in <> axiWriteDataQueueIO.deq

    axiWriteDataQueueIO.enq.valid := writeHelper.fire(axiWriteDataQueueIO.enq.ready, grant)
    axiWriteDataQueueIO.enq.bits := io.axi.w.bits.data

    // check to see if axi4 is ready to accept data instead of forcing writes
    empty_bytes := (sp.bufferDepth.U - axiWriteDataQueueIO.count) * axiBeatBytes.U
  }

  io.streams.zipWithIndex.foreach({ case (s, i) => {
    connect_axiw(s.deq, s.empty_bytes, s.sp, i)
  }})

  /////////////////////////////////////////////////////////////////////////////

  assert(!io.axi.ar.valid || io.axi.ar.bits.size === log2Ceil(axiBeatBytes).U)
  io.axi.r.bits.resp := 0.U(2.W)
  io.axi.r.bits.id   := io.axi.ar.bits.id
  io.axi.r.bits.user := io.axi.ar.bits.user
  io.axi.r.valid     := false.B
  io.axi.ar.ready    := false.B
  io.axi.r.bits.data := DontCare
  io.axi.r.bits.last := DontCare

  def connect_axir(
    enq: DecoupledIO[UInt],
    filled_bytes: UInt,
    sp: StreamParam,
    idx: Int
  ) = {
    val serdes_enq = Module(new StreamWidthAdapter(axiParams.dataBits, sp.widthBits))
    // unused
    serdes_enq.io.narrow.in.bits  := 0.U
    serdes_enq.io.narrow.in.valid := false.B
    serdes_enq.io.wide.out.ready  := false.B

    val outgoingQueueIO = Module(new Queue(UInt(sp.widthBits.W), sp.bufferDepth)).io

    outgoingQueueIO.enq <> enq
    serdes_enq.io.wide.in <> outgoingQueueIO.deq

    val grant = (io.axi.ar.bits.addr >> addressSpaceBits) === idx.U

    val axiReadDataQueueIO = Module(new Queue(UInt(axiParams.dataBits.W), sp.bufferDepth)).io

    val readHelper = DecoupledHelper(
      io.axi.ar.valid,
      io.axi.r.ready,
      axiReadDataQueueIO.deq.valid,
    )

    val readBeatCounter = RegInit(0.U(9.W))
    val lastReadBeat    = readBeatCounter === io.axi.ar.bits.len
    when(io.axi.r.fire && grant) {
      readBeatCounter := Mux(lastReadBeat, 0.U, readBeatCounter + 1.U)
    }

    axiReadDataQueueIO.enq <> serdes_enq.io.narrow.out
    axiReadDataQueueIO.deq.ready := readHelper.fire(axiReadDataQueueIO.deq.valid, grant)

    when (grant) {
      io.axi.r.valid     := readHelper.fire(io.axi.r.ready)
      io.axi.r.bits.data := axiReadDataQueueIO.deq.bits
      io.axi.r.bits.last := lastReadBeat
      io.axi.ar.ready    := readHelper.fire(io.axi.ar.valid, lastReadBeat)
    }

    // check to see if io.axi has valid output instead of waiting for timeouts
    filled_bytes := axiReadDataQueueIO.count * axiBeatBytes.U
  }

  io.streams.zipWithIndex.foreach({ case (s, i) => {
    connect_axir(s.enq, s.filled_bytes, s.sp, i)
  }})
}
