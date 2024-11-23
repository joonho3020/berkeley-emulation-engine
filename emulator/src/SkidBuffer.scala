package emulator

import chisel3._
import chisel3.util._

class SkidBuffer[T <: Data](gen: T) extends Module {
  val io = IO(new Bundle {
    val enq  = Flipped(Decoupled(gen))
    val deq = Decoupled(gen)
  })

  val skidData   = Reg(gen)
  val skidValid  = RegInit(false.B)

  val pipeData   = Reg(gen)
  val pipeValid  = RegInit(false.B)

  io.deq.bits  := pipeData
  io.deq.valid := pipeValid
  io.enq.ready  := !skidValid

  when(io.enq.valid && io.enq.ready && pipeValid) {
    skidData  := io.enq.bits
    skidValid := true.B
  }

  when(!pipeValid || io.deq.valid && io.deq.ready) {
    pipeData  := Mux(skidValid, skidData, io.enq.bits)
    pipeValid := skidValid || io.enq.valid
    skidValid := false.B
  }
}

class SkidBufferChain[T <: Data](gen: T, depth: Int) extends Module {
  val io = IO(new Bundle {
    val enq  = Flipped(Decoupled(gen))
    val deq = Decoupled(gen)
  })

  val sbs = Seq.fill(depth)(Module(new SkidBuffer(gen)))
  sbs(0).io.enq <> io.enq
  for (i <- 0 until depth - 1) {
    sbs(i+1).io.enq <> sbs(i).io.deq
  }
  io.deq <> sbs(depth-1).io.deq
}
