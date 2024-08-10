//------------------------------------------------------------------------
// Counters.v
//
// HDL for the counters sample.  This HDL describes two counters operating
// on different board clocks and with slightly different functionality.
// The counter controls and counter values are connected to endpoints so
// that FrontPanel may control and observe them.
//
// Copyright (c) 2005-2011
// Opal Kelly Incorporated
//------------------------------------------------------------------------

`default_nettype none
module FPGATop(
	input  wire [4:0]   okUH,
	output wire [2:0]   okHU,
	inout  wire [31:0]  okUHU,
	inout  wire         okAA,

	input  wire         sys_clkp,
	input  wire         sys_clkn,
	
	output wire [3:0]   led,
	inout  wire [3:0]   VREF
	);

// VREF pins MUST be set to Hi-Z if not used for Vref.
// See the XEM7350 - DDR3 Memory documentation for more information.
assign VREF = 4'bZZZZ;

// Clock
wire sys_clk;
IBUFGDS osc_clk(.O(sys_clk), .I(sys_clkp), .IB(sys_clkn));

// Target interface bus:
wire         okClk;
wire [112:0] okHE;
wire [64:0]  okEH;

// Endpoint connections:
wire [31:0] ep_reset;
wire [31:0] ep_host_steps;
wire [31:0] ep_used_procs;
wire [31:0] ep_insns_ready;
wire [31:0] ep_insns_valid;
wire [31:0] ep_insns_bits_0;
wire [31:0] ep_insns_bits_1;
wire [31:0] ep_io_i_ready;
wire [31:0] ep_io_i_valid;
wire [31:0] ep_io_i_bits_0;
wire [31:0] ep_io_o_ready;
wire [31:0] ep_io_o_valid;
wire [31:0] ep_io_o_bits_0;

wire reset;
wire [15:0] io_host_steps;
wire [2:0]  io_used_procs;
wire        io_insns_ready;
wire        io_insns_valid;
wire [15:0] io_insns_bits_0;
wire [15:0] io_insns_bits_1;
wire        io_io_i_ready;
wire        io_io_i_valid;
wire [15:0] io_io_i_bits_0;
wire        io_io_o_ready;
wire        io_io_o_valid;
wire [15:0] io_io_o_bits_0;

assign reset = ep_reset[0];

assign io_host_steps = ep_host_steps[15:0];
assign io_used_procs = ep_used_procs[2:0];

assign ep_insns_ready = {31'h0, io_insns_ready};
assign io_insns_valid = ep_insns_valid[0];
assign io_insns_bits_0 = ep_insns_bits_0[15:0];
assign io_insns_bits_1 = ep_insns_bits_1[15:0];

assign ep_io_i_ready = {31'h0, io_io_i_ready};
assign io_io_i_valid = ep_io_i_valid[0];
assign io_io_i_bits_0 = ep_io_i_bits_0[15:0];

assign ep_io_o_valid = {31'h0, io_io_o_valid};
assign io_io_o_ready = ep_io_o_ready[0];
assign ep_io_o_bits_0 = io_io_o_bits_0[15:0];

assign led[0] = !(io_io_i_valid && io_io_i_ready);
assign led[1] = !(io_io_o_valid && io_io_o_ready);
assign led[2] = !(io_insns_valid && io_insns_ready);
assign led[3] = reset;

// Instantiate the okHost and connect endpoints.
wire [65*4-1:0]  okEHx;
okHost okHI(
	.okUH(okUH),
	.okHU(okHU),
	.okUHU(okUHU),
	.okAA(okAA),
	.okClk(okClk),
	.okHE(okHE), 
	.okEH(okEH)
);

okWireOR # (.N(4)) wireOR (okEH, okEHx);

okWireIn     okin0(.okHE(okHE),                              .ep_addr(8'h00), .ep_dataout(ep_reset));
okWireIn     okin1(.okHE(okHE),                              .ep_addr(8'h01), .ep_dataout(ep_host_steps));
okWireIn     okin2(.okHE(okHE),                              .ep_addr(8'h02), .ep_dataout(ep_used_procs));
okWireIn     okin3(.okHE(okHE),                              .ep_addr(8'h03), .ep_dataout(ep_insns_valid));
okWireIn     okin4(.okHE(okHE),                              .ep_addr(8'h04), .ep_dataout(ep_insns_bits_0));
okWireIn     okin5(.okHE(okHE),                              .ep_addr(8'h05), .ep_dataout(ep_insns_bits_1));
okWireIn     okin6(.okHE(okHE),                              .ep_addr(8'h06), .ep_dataout(ep_io_i_valid));
okWireIn     okin7(.okHE(okHE),                              .ep_addr(8'h07), .ep_dataout(ep_io_i_bits_0));
okWireIn     okin8(.okHE(okHE),                              .ep_addr(8'h08), .ep_dataout(ep_io_o_ready));

okWireOut    okout0(.okHE(okHE), .okEH(okEHx[ 0*65 +: 65 ]), .ep_addr(8'h20), .ep_datain(ep_insns_ready));
okWireOut    okout1(.okHE(okHE), .okEH(okEHx[ 1*65 +: 65 ]), .ep_addr(8'h21), .ep_datain(ep_io_i_ready));
okWireOut    okout2(.okHE(okHE), .okEH(okEHx[ 2*65 +: 65 ]), .ep_addr(8'h22), .ep_datain(ep_io_o_valid));
okWireOut    okout3(.okHE(okHE), .okEH(okEHx[ 3*65 +: 65 ]), .ep_addr(8'h23), .ep_datain(ep_io_o_bits_0));

OpalKellyEmulatorModuleTop dut(
  .clock(sys_clk),
  .reset(reset),
  .io_host_steps(io_host_steps),
  .io_used_procs(io_used_procs),
  .io_insns_ready(io_insns_ready),
  .io_insns_valid(io_insns_valid),
  .io_insns_bits_0(io_insns_bits_0),
  .io_insns_bits_1(io_insns_bits_1),
  .io_io_i_ready(io_io_i_ready),
  .io_io_i_valid(io_io_i_valid),
  .io_io_i_bits_0(io_io_i_bits_0),
  .io_io_o_ready(io_io_o_ready),
  .io_io_o_valid(io_io_o_valid),
  .io_io_o_bits_0(io_io_o_bits_0)
);

endmodule
`default_nettype wire
