//------------------------------------------------------------------------
// First_tf.v
//
// A simple text fixture example for getting started with FrontPanel 3.x
// simulation.  This sample connects the top-level signals from First.v
// to a call system that, when integrated with Opal Kelly simulation
// libraries, mimics the functionality of FrontPanel.  Listed below are
// the tasks and functions that can be called.  They are designed to
// replicate calls made from the PC via FrontPanel API, Python, Java, DLL,
// etc.
//
//------------------------------------------------------------------------
// Copyright (c) 2005-2023 Opal Kelly Incorporated
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// 
//------------------------------------------------------------------------
`timescale 1ns / 1ps
`default_nettype none

module FPGATopTestHarness;

wire  [4:0]   okUH;
wire  [2:0]   okHU;
wire  [31:0]  okUHU;
wire  [3:0]   led;

reg sys_clkp;
reg sys_clkn;

// FIXME: sys_clkp has a period of 5ns (200MHz). Is it okay to use T/2 for
// that or is that a bit sus?
localparam real T=10;
assign sys_clkn = ~sys_clkp;
always begin
  #(T/2) sys_clkp <= ~sys_clkp;
end

FPGATop dut (
	.okUH(okUH),
	.okHU(okHU),
	.okUHU(okUHU),
  .sys_clkp(sys_clkp),
  .sys_clkn(sys_clkn),
	.led(led)
	);

//------------------------------------------------------------------------
// Begin okHostInterface simulation user configurable  global data
//------------------------------------------------------------------------
parameter BlockDelayStates = 5;   // REQUIRED: # of clocks between blocks of pipe data
parameter ReadyCheckDelay = 5;    // REQUIRED: # of clocks before block transfer before
                                  //           host interface checks for ready (0-255)
parameter PostReadyDelay = 5;     // REQUIRED: # of clocks after ready is asserted and
                                  //           check that the block transfer begins (0-255)
parameter pipeInSize = 1024;      // REQUIRED: byte (must be even) length of default
                                  //           PipeIn; Integer 0-2^32
parameter pipeOutSize = 1024;     // REQUIRED: byte (must be even) length of default
                                  //           PipeOut; Integer 0-2^32

integer k;
reg  [7:0]  pipeIn [0:(pipeInSize-1)];
initial for (k=0; k<pipeInSize; k=k+1) pipeIn[k] = 8'h00;

reg  [7:0]  pipeOut [0:(pipeOutSize-1)];
initial for (k=0; k<pipeOutSize; k=k+1) pipeOut[k] = 8'h00;

wire [31:0] u32Address [0:31];
reg  [31:0] u32Data [0:31];
wire [31:0] u32Count;
wire [31:0] ReadRegisterData;

//------------------------------------------------------------------------
//  Available User Task and Function Calls:
//    FrontPanelReset;                  // Always start routine with FrontPanelReset;
//    SetWireInValue(ep, val, mask);
//    UpdateWireIns;
//    UpdateWireOuts;
//    GetWireOutValue(ep);
//    ActivateTriggerIn(ep, bit);       // bit is an integer 0-15
//    UpdateTriggerOuts;
//    IsTriggered(ep, mask);            // Returns a 1 or 0
//    WriteToPipeIn(ep, length);        // passes pipeIn array data
//    ReadFromPipeOut(ep, length);      // passes data to pipeOut array
//    WriteToBlockPipeIn(ep, blockSize, length);    // pass pipeIn array data; blockSize and length are integers
//    ReadFromBlockPipeOut(ep, blockSize, length);  // pass data to pipeOut array; blockSize and length are integers
//
//    *Pipes operate by passing arrays of data back and forth to the user's
//    design.  If you need multiple arrays, you can create a new procedure
//    above and connect it to a differnet array.  More information is
//    available in Opal Kelly documentation and online support tutorial.
//------------------------------------------------------------------------

// User configurable block of called FrontPanel operations.
reg [31:0] insns_ready, i_ready, o_valid, o_bits_0;
initial insns_ready = 0;
initial i_ready = 0;
initial o_valid = 0;
initial o_bits_0 = 0;



// task enq_instruction;
//   input [31:0] bits_1;
//   input [31:0] bits_0;
//   begin
//     while (insns_ready == 0) begin
//       UpdateWireOuts;
//       insns_ready = GetWireOutValue(8'h20);
//       @(posedge okUH[0]);
//     end
//     SetWireInValue(8'h04, bits_0, 32'hffff_ffff); // bits0
//     UpdateWireIns;
//     SetWireInValue(8'h05, bits_1, 32'hffff_ffff); // bits1
//     UpdateWireIns;
//     SetWireInValue(8'h03, 32'h01, 32'hffff_ffff); // valid
//     UpdateWireIns;
//     SetWireInValue(8'h03, 32'h00, 32'hffff_ffff); // valid
//     UpdateWireIns;
//   end
// endtask
// 
// task enq_inputs;
//   input [31:0] bits_0;
//   begin
//     while (i_ready == 0) begin
//       UpdateWireOuts;
//       i_ready = GetWireOutValue(8'h21);
//       @(posedge okUH[0]);
//     end
//     SetWireInValue(8'h07, bits_0, 32'hffff_ffff); // bits0
//     UpdateWireIns;
//     SetWireInValue(8'h06, 32'h01, 32'hffff_ffff); // valid
//     UpdateWireIns;
//     SetWireInValue(8'h03, 32'h00, 32'hffff_ffff); // valid
//     UpdateWireIns;
//   end
// endtask

initial begin
  sys_clkp = 0;

  FrontPanelReset;                      // Start routine with FrontPanelReset;

  // host steps
  SetWireInValue(8'h00, 32'h1, 32'hffff_ffff);
  UpdateWireIns;
  // host steps
  SetWireInValue(8'h00, 32'h0, 32'hffff_ffff);
  UpdateWireIns;

  // host steps
  SetWireInValue(8'h01, 32'h6, 32'hffff_ffff);
  UpdateWireIns;

  // used procs
  SetWireInValue(8'h02, 32'h6, 32'hffff_ffff);
  UpdateWireIns;

  // enq_instruction(32'h80, 32'h01);

  // enq_inputs(32'h4);
  // 


  // UpdateWireOuts;
  // o_valid = GetWireOutValue(8'h22);
  // o_bits_0  = GetWireOutValue(8'h23);
  // SetWireInValue(8'h08, 32'h1, 32'hffff_ffff);
  // UpdateWireIns;
  // SetWireInValue(8'h08, 32'h0, 32'hffff_ffff);
  // UpdateWireIns;

  // enq_inputs(32'h8);
  // enq_inputs(32'h9);

  // instruction
  SetWireInValue(8'h04, 32'h01, 32'hffff_ffff);
  UpdateWireIns;
  SetWireInValue(8'h05, 32'h80, 32'hffff_ffff);
  UpdateWireIns;
  SetWireInValue(8'h03, 32'h01, 32'hffff_ffff);
  UpdateWireIns;
  SetWireInValue(8'h03, 32'h00, 32'hffff_ffff);
  UpdateWireIns;

  // ready insns_ready
  UpdateWireOuts;
  insns_ready = GetWireOutValue(8'h20);

end

`include "./oksim/okHostCalls.vh"   // Do not remove!  The tasks, functions, and data stored
                                   // in okHostCalls.vh must be included here.

endmodule
`default_nettype wire
