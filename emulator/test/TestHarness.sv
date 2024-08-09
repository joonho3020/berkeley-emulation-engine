
`timescale 1 ns/10 ps

module testharness;

reg         clock;
reg         reset;
reg  [5:0] io_used_procs;
reg  [15:0] io_host_steps;
wire        io_insns_ready;
reg         io_insns_valid;
reg  [15:0] io_insns_bits_0;
reg  [15:0] io_insns_bits_1;
reg  [15:0] io_insns_bits_2;
wire        io_io_i_ready;
reg         io_io_i_valid;
reg  [15:0] io_io_i_bits_0;
reg  [15:0] io_io_i_bits_1;
reg  [15:0] io_io_i_bits_2;
reg  [15:0] io_io_i_bits_3;
reg         io_io_o_ready;
wire        io_io_o_valid;
wire [15:0] io_io_o_bits_0;
wire [15:0] io_io_o_bits_1;
wire [15:0] io_io_o_bits_2;
wire [15:0] io_io_o_bits_3;

localparam T=20;
always begin
  #(T/2) clock <= ~clock;
end

task enq_instructions;
  input [15:0] bits_2;
  input [15:0] bits_1;
  input [15:0] bits_0;
    begin
      while (io_insns_ready == 0) begin
        @(posedge clock);#(0);
      end
      #1;
      io_insns_valid = 1;
      if (io_insns_ready && io_insns_valid) begin
        io_insns_bits_0 = bits_0;
        io_insns_bits_1 = bits_1;
        io_insns_bits_2 = bits_2;
      end
      @(posedge clock);#(0);
      io_insns_valid = 0;
    end
endtask

task deq_outputs;
    begin
      io_io_o_ready= 1;
      if (io_io_o_ready && io_io_o_valid) begin
        $display($time, " output %x %x %x %x",
          io_io_o_bits_0,
          io_io_o_bits_1,
          io_io_o_bits_2,
          io_io_o_bits_3);
      end
    end
endtask

task enq_inputs;
  input [15:0] bits_3;
  input [15:0] bits_2;
  input [15:0] bits_1;
  input [15:0] bits_0;
  begin
    while (io_io_i_ready == 0) begin
      @(posedge clock);#(0);
      deq_outputs();
    end
    #1;
    io_io_i_valid = 1;
    if (io_io_i_valid && io_io_i_ready) begin
      io_io_i_bits_0 = bits_0;
      io_io_i_bits_1 = bits_1;
      io_io_i_bits_2 = bits_2;
      io_io_i_bits_3 = bits_3;
    end
    @(posedge clock);#(0);
    deq_outputs();
    io_io_i_valid = 0;
  end
endtask

initial begin
  clock  = 1'b1;
  reset = 1'b1;

  #(T*2) reset = 1'b1;
  #(T*2) reset = 1'b0;

  $display($time, " ** Start Simulation **");

      io_host_steps = 16;
      io_used_procs = 9;
      
  io_insns_valid = 0;
  io_insns_bits_0 = 0;
  io_insns_bits_1 = 0;
  io_insns_bits_2 = 0;

  io_io_i_valid = 0;
  io_io_i_bits_0 = 0;
  io_io_i_bits_1 = 0;
  io_io_i_bits_2 = 0;
  io_io_i_bits_3 = 0;

  io_io_o_ready = 0;

  @(posedge clock);#(0);
  @(posedge clock);#(0);
  
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h1
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h8, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h28, 16'h6400, 16'hec3
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h1
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h10, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h18, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h20, 16'h1c, 16'h113
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h28, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h28, 16'h303c, 16'hab
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h2c, 16'h385c, 16'h1a2b
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h28, 16'h86c, 16'h723
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h28, 16'h7014, 16'h6c3
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h1
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h1
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h1
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h6805
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h30, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h38, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h40, 16'h1c, 16'h8b
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'hc00, 16'h1453
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'hc, 16'h113
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h1c00, 16'h1043
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h4, 16'h10, 16'h1813
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h4, 16'h18, 16'h843
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h4, 16'h1802
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h420, 16'h1913
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h42c, 16'h3333
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h440, 16'h1333
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h4, 16'h3438, 16'h813
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h7805
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h4, 16'h5553
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h4, 16'h802
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h8, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h7005
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h4, 16'h5d53
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h4, 16'h1002
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h8, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h6005
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h8, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_instructions(
  16'h0, 16'h0, 16'h0
            );
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h0);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h0);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1e);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'ha);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h18);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h14);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h16);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h0);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h1c);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'h3);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  @(posedge clock);#(0);
  enq_inputs(
  16'h0, 16'h0, 16'h0, 16'hb);
  repeat (16) begin
    @(posedge clock);#(0);
  end
  
  repeat (64) begin
    @(posedge clock);#(0);
  end
  
  $display($time, " ** End Simulation **=");
  $finish;
end

// dump the state of the design
// VCD (Value Change Dump) is a standard dump format defined in Verilog.
initial begin
  $dumpfile("sim.vcd");
  $dumpvars(0, testharness);
end

OpalKellyEmulatorModuleTop dut(
  .clock(clock),
  .reset(reset),
  .io_host_steps(io_host_steps),
  .io_used_procs(io_used_procs),
  .io_insns_ready(io_insns_ready),
  .io_insns_valid(io_insns_valid),
  .io_insns_bits_0(io_insns_bits_0),
  .io_insns_bits_1(io_insns_bits_1),
  .io_insns_bits_2(io_insns_bits_2),
  .io_io_i_ready(io_io_i_ready),
  .io_io_i_valid(io_io_i_valid),
  .io_io_i_bits_0(io_io_i_bits_0),
  .io_io_i_bits_1(io_io_i_bits_1),
  .io_io_i_bits_2(io_io_i_bits_2),
  .io_io_i_bits_3(io_io_i_bits_3),
  .io_io_o_ready(io_io_o_ready),
  .io_io_o_valid(io_io_o_valid),
  .io_io_o_bits_0(io_io_o_bits_0),
  .io_io_o_bits_1(io_io_o_bits_1),
  .io_io_o_bits_2(io_io_o_bits_2),
  .io_io_o_bits_3(io_io_o_bits_3)
);

endmodule
    