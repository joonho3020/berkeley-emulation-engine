package emulator

import chisel3._
import chisel3.util._

class Processor extends Module {
  val io = IO(new Bundle {
    val a = Input(UInt(3.W))
    val b = Output(UInt(3.W))
  })
  io.b := io.a
}
