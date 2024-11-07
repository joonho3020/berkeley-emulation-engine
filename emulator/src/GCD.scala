package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental._
import freechips.rocketchip.amba.axi4._

class GCD extends Module {
  val io = IO(new Bundle {
    val a = Input(UInt(32.W))
    val b = Input(UInt(32.W))
    val start = Input(Bool())
    val done = Output(Bool())
    val result = Output(UInt(32.W))
  })

  // Registers to hold intermediate values
  val x = RegInit(0.U(32.W))
  val y = RegInit(0.U(32.W))
  val busy = RegInit(false.B)

  // Output done signal
  io.done := !busy && io.start

  when(io.start && !busy) {
    x := io.a
    y := io.b
    busy := true.B
  }

  when(busy) {
    when(x > y) {
      x := x - y
    }.elsewhen(y > x) {
      y := y - x
    }.otherwise {
      busy := false.B
    }
  }

  io.result := x
}

class GCDWithAXI(cfg: AXI4BundleParameters) extends Module {
  val io = IO(new Bundle {
    val axi = Flipped(new AXI4Bundle(cfg)) // AXI4Lite Interface with 32-bit address and data width
  })

  // Instantiate the GCD core
  val gcd = Module(new GCD)
  gcd.io.start := false.B
  gcd.io.a := 0.U
  gcd.io.b := 0.U

  // Define AXI4-Lite register addresses
  val addr_a = 0x00.U
  val addr_b = 0x04.U
  val addr_start = 0x08.U
  val addr_result = 0x0C.U

  // Registers to store the inputs and outputs
  val regA = RegInit(0.U(32.W))
  val regB = RegInit(0.U(32.W))
  val regStart = RegInit(false.B)
  val regResult = RegInit(0.U(32.W))
  val busy = RegInit(false.B)

  // AXI4-Lite read and write response
  val awValid = io.axi.aw.valid
  val wValid  = io.axi.w.valid
  val arValid = io.axi.ar.valid

  // AXI write handshake
  io.axi.aw.ready := !busy
  io.axi.w.ready  := !busy

  // Capture write address and data
  when(awValid && io.axi.aw.ready) {
    switch(io.axi.aw.bits.addr) {
      is(addr_a) { regA := io.axi.w.bits.data }
      is(addr_b) { regB := io.axi.w.bits.data }
      is(addr_start) { regStart := true.B }
    }
  }

  // Connect inputs to GCD
  gcd.io.a := regA
  gcd.io.b := regB
  gcd.io.start := regStart

  // Start computation if `start` is high
  when(regStart && gcd.io.done) {
    regResult := gcd.io.result
    regStart := false.B
  }

  // AXI4-Lite read handshake
  io.axi.ar.ready := !busy
  io.axi.r.valid := arValid && io.axi.ar.ready
  io.axi.r.bits.resp := 0.U

  // Return the result on AXI read
  io.axi.r.bits.data := MuxLookup(io.axi.ar.bits.addr, 0.U)(Seq(
    addr_a -> regA,
    addr_b -> regB,
    addr_result -> regResult
  ))

  // Write response always OK
  io.axi.b.valid := io.axi.aw.valid && io.axi.w.valid
  io.axi.b.bits.resp := 0.U // OKAY response


  io.axi.b.bits.id   := DontCare
  io.axi.r.bits.id   := DontCare
  io.axi.r.bits.last := DontCare
}
