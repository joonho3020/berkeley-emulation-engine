`timescale 1 ns/10 ps

module testharness;

reg clock, reset;
reg [1:0] io_value1;
reg [1:0] io_value2;
reg io_loadingValues;
reg [1:0] io_outputGCD;
reg io_outputValid;

integer i;

localparam T=20;
always begin
  #(T/2) clock = ~clock;
end


initial begin
  clock  = 1'b0;
  reset = 1'b0;

  #(T*2) reset = 1'b1;
  #(T*2) reset = 1'b0;

  $display($time, " ** Start Simulation **");

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 0;
  io_value2 = 0;
  io_loadingValues = 0;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 0;
  io_value2 = 0;
  io_loadingValues = 0;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 3;
  io_value2 = 1;
  io_loadingValues = 1;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 3;
  io_value2 = 1;
  io_loadingValues = 0;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 3;
  io_value2 = 1;
  io_loadingValues = 0;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 3;
  io_value2 = 1;
  io_loadingValues = 0;
  #(T);

  $display($time, " io_outputValid %x io_outputGCD %x", gcd.io_outputValid, gcd.io_outputGCD);
  io_value1 = 3;
  io_value2 = 1;
  io_loadingValues = 0;
  #(T);

  $display($time, " ** End Simulation **");
  $finish;
end

// dump the state of the design
// VCD (Value Change Dump) is a standard dump format defined in Verilog.
initial begin
  $dumpfile("sim.vcd");
  $dumpvars(0, testharness);
end

GCD gcd(
  .clock(clock),
  .reset(reset),

  .io_value1(io_value1),
  .io_value2(io_value2),
  .io_loadingValues(io_loadingValues),

  .io_outputGCD(io_outputGCD),
  .io_outputValid(io_outputValid)
);

endmodule
