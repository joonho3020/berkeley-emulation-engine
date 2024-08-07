
`timescale 1 ns/10 ps

module testharness;

reg clock;
reg reset;
wire io_in_ready;
reg io_in_valid;
reg [2:0] io_in_bits_a;
reg [1:0] io_in_bits_b;
reg io_out_ready;
wire io_out_valid;
wire [2:0] io_out_bits_a;
wire [1:0] io_out_bits_b;

localparam T=20;
always begin
  #(T/2) clock <= ~clock;
end

task deq_from_queue;
    begin
      io_out_ready = 1;
      if (io_out_ready && io_out_valid) begin
        $display($time, " deq io_out_bits_a: %x io_out_bits_b: %x ", io_out_bits_a, io_out_bits_b);
      end
    end
endtask

task enq_to_queue;
    begin
      while (io_in_ready == 0) begin
        @(posedge clock);#(0);
        deq_from_queue();
      end
      #1;
      io_in_valid = 1;
      if (io_in_ready && io_in_valid) begin
        $display($time, " enq io_in_bits_a: %x io_in_bits_b: %x ", io_in_bits_a, io_in_bits_b);
      end
      deq_from_queue();
    end
endtask

initial begin
  clock  = 1'b1;
  reset = 1'b1;

  #(T*2) reset = 1'b1;
  #(T*2) reset = 1'b0;

  $display($time, " ** Start Simulation **");

  @(posedge clock);#(0);
  $display($time, " io_in_ready %x io_out_valid %x io_out_bits_a %x io_out_bits_b %x ", top.io_in_ready, top.io_out_valid, top.io_out_bits_a, top.io_out_bits_b);
  io_in_bits_a = 1;
  io_in_bits_b = 1;
  enq_to_queue();

  @(posedge clock);#(0);
  $display($time, " io_in_ready %x io_out_valid %x io_out_bits_a %x io_out_bits_b %x ", top.io_in_ready, top.io_out_valid, top.io_out_bits_a, top.io_out_bits_b);
  io_in_bits_a = 2;
  io_in_bits_b = 2;
  enq_to_queue();


  @(posedge clock);#(0);
  $display($time, " io_in_ready %x io_out_valid %x io_out_bits_a %x io_out_bits_b %x ", top.io_in_ready, top.io_out_valid, top.io_out_bits_a, top.io_out_bits_b);
  reset = 0;
  io_in_bits_a = 3;
  io_in_bits_b = 3;
  enq_to_queue();
  @(posedge clock);#(0);
  @(posedge clock);#(0);
  @(posedge clock);#(0);

  $display($time, " ** End Simulation **=");
  $finish;
end

// dump the state of the design
// VCD (Value Change Dump) is a standard dump format defined in Verilog.
initial begin
  $dumpfile("sim.vcd");
  $dumpvars(0, testharness);
end

MyQueue top(
  .clock(clock),
  .reset(reset),
  .io_in_ready(io_in_ready),
  .io_in_valid(io_in_valid),
  .io_in_bits_a(io_in_bits_a),
  .io_in_bits_b(io_in_bits_b),
  .io_out_ready(io_out_ready),
  .io_out_valid(io_out_valid),
  .io_out_bits_a(io_out_bits_a),
  .io_out_bits_b(io_out_bits_b)
);

endmodule
