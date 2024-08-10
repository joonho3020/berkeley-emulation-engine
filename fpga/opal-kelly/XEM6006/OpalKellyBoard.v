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

module OpalKellyBoard(
	input  wire [7:0]  hi_in,
	output wire [1:0]  hi_out,
	inout  wire [15:0] hi_inout,
	inout  wire        hi_aa,

	input  wire        sys_clk,
	
	output wire [3:0]  led
	);

// Target interface bus:
wire        ti_clk;
wire [30:0] ok1;
wire [16:0] ok2;

// Endpoint connections:
wire [15:0] ep_reset;
wire reset;
assign reset = ep_reset[0];

wire [15:0] ep_host_steps;
wire [15:0] ep_used_procs;
wire [15:0] ep_insns_ready;
wire [15:0] ep_insns_valid;
wire [15:0] ep_insns_bits_0;
wire [15:0] ep_insns_bits_1;
wire [15:0] ep_io_i_ready;
wire [15:0] ep_io_i_valid;
wire [15:0] ep_io_i_bits_0;
wire [15:0] ep_io_o_ready;
wire [15:0] ep_io_o_valid;
wire [15:0] ep_io_o_bits_0;

wire [15:0] io_host_steps;
wire [1:0]  io_used_procs;
wire        io_insns_ready;
wire        io_insns_valid;
wire [15:0] io_insns_bits_0;
wire [15:0] io_insns_bits_1;
wire        io_io_i_ready;
wire        io_io_i_valid;
wire [15:0] io_io_i_bits_0;
wire [15:0] io_io_i_bits_1;
wire [15:0] io_io_i_bits_2;
wire [15:0] io_io_i_bits_3;
wire        io_io_o_ready;
wire        io_io_o_valid;
wire [15:0] io_io_o_bits_0;
wire [15:0] io_io_o_bits_1;
wire [15:0] io_io_o_bits_2;
wire [15:0] io_io_o_bits_3;

assign io_host_steps = ep_host_steps;
assign io_used_procs = ep_used_procs[1:0];

assign ep_insns_ready = {15'h0, io_insns_ready};
assign io_insns_valid = ep_insns_valid[0];
assign io_insns_bits_0 = ep_insns_bits_0;
assign io_insns_bits_1 = ep_insns_bits_1;

assign ep_io_i_ready = {15'h0, io_io_i_ready};
assign io_io_i_valid = ep_io_i_valid[0];
assign io_io_i_bits_0 = ep_io_i_bits_0;

assign ep_io_o_valid = {15'h0, io_io_o_valid};
assign io_io_o_ready = ep_io_o_ready[0];
assign ep_io_o_bits_0 = io_io_o_bits_0;

assign led[0] = !(io_io_i_valid && io_io_i_ready);
assign led[1] = !(io_io_o_valid && io_io_o_ready);
assign led[2] = !(io_insns_valid && io_insns_ready);
assign led[3] = reset;

// Instantiate the okHost and connect endpoints.
wire [17*4-1:0]  ok2x;
okHost okHI(
  .hi_in(hi_in), .hi_out(hi_out), .hi_inout(hi_inout), .hi_aa(hi_aa), .ti_clk(ti_clk),
  .ok1(ok1), .ok2(ok2));

okWireOR # (.N(4)) wireOR (ok2, ok2x);

okWireIn     okin0 (.ok1(ok1),                         .ep_addr(8'h00), .ep_dataout(ep_reset));
okWireIn     okin1 (.ok1(ok1),                         .ep_addr(8'h01), .ep_dataout(ep_host_steps));
okWireIn     okin2 (.ok1(ok1),                         .ep_addr(8'h02), .ep_dataout(ep_used_procs));
okWireIn     okin3 (.ok1(ok1),                         .ep_addr(8'h03), .ep_dataout(ep_insns_valid));
okWireIn     okin4 (.ok1(ok1),                         .ep_addr(8'h04), .ep_dataout(ep_insns_bits_0));
okWireIn     okin5 (.ok1(ok1),                         .ep_addr(8'h05), .ep_dataout(ep_insns_bits_1));
okWireIn     okin7 (.ok1(ok1),                         .ep_addr(8'h07), .ep_dataout(ep_io_i_valid));
okWireIn     okin8 (.ok1(ok1),                         .ep_addr(8'h08), .ep_dataout(ep_io_i_bits_0));
okWireIn     okin12(.ok1(ok1),                         .ep_addr(8'h0C), .ep_dataout(ep_io_o_ready));

okWireOut    okout0(.ok1(ok1), .ok2(ok2x[ 0*17 +: 17 ]), .ep_addr(8'h20), .ep_datain(ep_insns_ready));
okWireOut    okout1(.ok1(ok1), .ok2(ok2x[ 1*17 +: 17 ]), .ep_addr(8'h21), .ep_datain(ep_io_i_ready));
okWireOut    okout2(.ok1(ok1), .ok2(ok2x[ 2*17 +: 17 ]), .ep_addr(8'h22), .ep_datain(ep_io_o_valid));
okWireOut    okout3(.ok1(ok1), .ok2(ok2x[ 3*17 +: 17 ]), .ep_addr(8'h23), .ep_datain(ep_io_o_bits_0));


OpalKellyEmulatorModuleTop tester(
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
