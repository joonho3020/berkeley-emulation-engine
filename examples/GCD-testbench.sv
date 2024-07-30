`timescale 1 ns/10 ps

module testharness;

reg clock, reset;
reg [1:0] io_value1;
reg [1:0] io_value2;
reg io_loadingValues;
reg [1:0] io_outputGCD;
reg io_outputValid;

integer i;

reg [1:0] io_value1_data [3:0];
reg [1:0] io_value2_data [3:0];
reg       io_loadingValues_data [3:0];

localparam T=20;
always begin
  #(T/2) clock = ~clock;
end


initial begin
  $readmemh("io_value1.txt",        io_value1_data);
  $readmemh("io_value2.txt",        io_value2_data);
  $readmemh("io_loadingValues.txt", io_loadingValues_data);

  clock  = 1'b0;
  reset = 1'b0;

  $display($time, " ** Start Simulation **");

  $monitor($time, " io_value1 : %x", gcd.io_value1);
  $monitor($time, " io_value2 : %x", gcd.io_value2);
  $monitor($time, " io_loadingValues : %x", gcd.io_loadingValues);
  $monitor($time, " io_outputGCD: %x", gcd.io_outputGCD);
  $monitor($time, " io_outputValid: %x", gcd.io_outputValid);

  #(T*2) reset = 1'b1;
  #(T*2) reset = 1'b0;

  for (i = 0; i < 16; i = i + 1)
  begin
    io_value1 = io_value1_data[i];
    io_value2 = io_value2_data[i];
    io_loadingValues = io_loadingValues_data[i];
    #(T);
  end

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
