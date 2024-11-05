package emulator

import chisel3._
import chisel3.util._

class ReorderQueueWrite[T <: Data](dType: T, tagWidth: Int) extends Bundle {
  val data = dType.cloneType
  val tag = UInt(tagWidth.W)
}

class ReorderEnqueueIO[T <: Data](dType: T, tagWidth: Int)
  extends DecoupledIO(new ReorderQueueWrite(dType, tagWidth)) {
}

class ReorderDequeueIO[T <: Data](dType: T, tagWidth: Int) extends Bundle {
  val valid = Input(Bool())
  val tag = Input(UInt(tagWidth.W))
  val data = Output(dType.cloneType)
  val matches = Output(Bool())
}

class ReorderQueue[T <: Data](dType: T, tagWidth: Int,
    size: Option[Int] = None, nDeq: Int = 1)
    extends Module {
  val io = new Bundle {
    val enq = Flipped(new ReorderEnqueueIO(dType, tagWidth))
    val deq = Vec(nDeq, new ReorderDequeueIO(dType, tagWidth))
  }

  val tagSpaceSize = 1 << tagWidth
  val actualSize = size.getOrElse(tagSpaceSize)

  if (tagSpaceSize > actualSize) {
    require(tagSpaceSize % actualSize == 0)

    val smallTagSize = log2Ceil(actualSize)

    val roq_data = Reg(Vec(actualSize, dType))
    val roq_tags = Reg(Vec(actualSize, UInt((tagWidth - smallTagSize).W)))
    val roq_free = VecInit(Seq.fill(actualSize)(RegInit(true.B)))
    val roq_enq_addr = io.enq.bits.tag(smallTagSize-1, 0)

    io.enq.ready := roq_free(roq_enq_addr)

    when (io.enq.valid && io.enq.ready) {
      roq_data(roq_enq_addr) := io.enq.bits.data
      roq_tags(roq_enq_addr) := io.enq.bits.tag >> smallTagSize.U
      roq_free(roq_enq_addr) := false.B
    }

    io.deq.foreach { deq =>
      val roq_deq_addr = deq.tag(smallTagSize-1, 0)

      deq.data := roq_data(roq_deq_addr)
      deq.matches := !roq_free(roq_deq_addr) && roq_tags(roq_deq_addr) === (deq.tag >> smallTagSize.U)

      when (deq.valid) {
        roq_free(roq_deq_addr) := true.B
      }
    }
  } else if (tagSpaceSize == actualSize) {
    val roq_data = Mem(tagSpaceSize, dType)
    val roq_free = VecInit(Seq.fill(tagSpaceSize)(RegInit(true.B)))

    io.enq.ready := roq_free(io.enq.bits.tag)

    when (io.enq.valid && io.enq.ready) {
      roq_data(io.enq.bits.tag) := io.enq.bits.data
      roq_free(io.enq.bits.tag) := false.B
    }

    io.deq.foreach { deq =>
      deq.data := roq_data(deq.tag)
      deq.matches := !roq_free(deq.tag)

      when (deq.valid) {
        roq_free(deq.tag) := true.B
      }
    }
  } else {
    require(actualSize % tagSpaceSize == 0)

    val qDepth = actualSize / tagSpaceSize
    val queues = Seq.fill(tagSpaceSize) {
      Module(new Queue(dType, qDepth))
    }

    io.enq.ready := false.B
    io.deq.foreach(_.matches := false.B)
    io.deq.foreach(_.data := 0.U.asTypeOf(dType))

    for ((q, i) <- queues.zipWithIndex) {
      when (io.enq.bits.tag === i.U) { io.enq.ready := q.io.enq.ready }
      q.io.enq.valid := io.enq.valid && io.enq.bits.tag === i.U
      q.io.enq.bits  := io.enq.bits.data

      val deqReadys = Wire(Vec(nDeq, Bool()))
      io.deq.zip(deqReadys).foreach { case (deq, rdy) =>
        when (deq.tag === i.U) {
          deq.matches := q.io.deq.valid
          deq.data := q.io.deq.bits
        }
        rdy := deq.valid && deq.tag === i.U
      }
      q.io.deq.ready := deqReadys.reduce(_ || _)
    }
  }
}
