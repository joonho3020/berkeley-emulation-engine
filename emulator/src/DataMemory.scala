package emulator

import chisel3._
import chisel3.util._

class DataMemoryReadPort(cfg: EmulatorConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Output(UInt(cfg.num_bits.W))
}

class DataMemoryWritePort(cfg: EmulatorConfig) extends Bundle {
  val idx = Input(UInt(cfg.index_bits.W))
  val bit = Input(UInt(cfg.num_bits.W))
  val en  = Input(Bool())
}

class AbstractDataMemory(cfg: EmulatorConfig) extends Module {
  import cfg._
  val io = IO(new Bundle {
    val rd = Vec(lut_inputs, new DataMemoryReadPort(cfg))
    val wr = new DataMemoryWritePort(cfg)
  })
}

class ChiselDataMemory(cfg: EmulatorConfig) extends AbstractDataMemory(cfg) {
  val mem = Reg(Vec(cfg.max_steps, UInt(cfg.num_bits.W)))

  when (io.wr.en) {
    mem(io.wr.idx) := io.wr.bit
  }

  for (i <- 0 until cfg.lut_inputs) {
    io.rd(i).bit := mem(io.rd(i).idx)
  }
}

class DataMemoryBlackBox(depth: Int, width: Int) extends BlackBox(Map(
  "DEPTH" -> depth,
  "WIDTH" -> width
)) with HasBlackBoxInline {
  val io = IO(new Bundle {
    val clk         = Input(Clock())
    val writeEnable = Input(Bool())
    val writeData   = Input(UInt(width.W))
    val writeAddr   = Input(UInt(log2Ceil(depth).W))
    val readAddr1   = Input(UInt(log2Ceil(depth).W))
    val readAddr2   = Input(UInt(log2Ceil(depth).W))
    val readAddr3   = Input(UInt(log2Ceil(depth).W))
    val readData1   = Output(UInt(width.W))
    val readData2   = Output(UInt(width.W))
    val readData3   = Output(UInt(width.W))
  })

  setInline("DataMemoryBlackBox.v",
    """
    |module DataMemoryBlackBox #(
    |    parameter DEPTH = 256,
    |    parameter WIDTH = 32
    |) (
    |    input                      clk,
    |    input                      writeEnable,
    |    input  [WIDTH-1:0]         writeData,
    |    input  [$clog2(DEPTH)-1:0] writeAddr,
    |    input  [$clog2(DEPTH)-1:0] readAddr1,
    |    input  [$clog2(DEPTH)-1:0] readAddr2,
    |    input  [$clog2(DEPTH)-1:0] readAddr3,
    |    output [WIDTH-1:0]         readData1,
    |    output [WIDTH-1:0]         readData2,
    |    output [WIDTH-1:0]         readData3
    |);
    |    (* ram_style = "distributed" *) reg [WIDTH-1:0] mem [0:DEPTH-1];
    |
    |    always @(posedge clk) begin
    |        if (writeEnable) begin
    |            mem[writeAddr] <= writeData;
    |        end
    |    end
    |
    |    assign readData1 = mem[readAddr1];
    |    assign readData2 = mem[readAddr2];
    |    assign readData3 = mem[readAddr3];
    |endmodule
    """.stripMargin)
}

class BlackBoxDataMemory(cfg: EmulatorConfig) extends AbstractDataMemory(cfg) {
  val memory = Module(new DataMemoryBlackBox(cfg.max_steps, cfg.num_bits))
  require(cfg.lut_inputs == 3)


  memory.io.clk         := clock
  memory.io.writeEnable := io.wr.en
  memory.io.writeData   := io.wr.bit
  memory.io.writeAddr   := io.wr.idx
  memory.io.readAddr1   := io.rd(0).idx
  memory.io.readAddr2   := io.rd(1).idx
  memory.io.readAddr3   := io.rd(2).idx
  io.rd(0).bit          := memory.io.readData1
  io.rd(1).bit          := memory.io.readData2
  io.rd(2).bit          := memory.io.readData3
}


class DataMemory(cfg: EmulatorConfig) extends AbstractDataMemory(cfg) {
  val mem = if (cfg.blackbox_dmem) {
    Module(new BlackBoxDataMemory(cfg))
  } else {
    Module(new ChiselDataMemory(cfg))
  }
  io <> mem.io
}
