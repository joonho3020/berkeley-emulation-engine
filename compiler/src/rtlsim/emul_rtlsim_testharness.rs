use crate::primitives::*;
use crate::rtlsim::rtlsim_utils::*;
use indexmap::IndexMap;
use itertools::Itertools;
use std::cmp::max;

fn generate_testbench_string(
    input_stimuli: &IndexMap<String, Vec<u64>>,
    circuit: &Circuit,
) -> String {
    let mut testbench = "
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
        $display($time, \" output %x %x %x %x\",
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

  $display($time, \" ** Start Simulation **\");
"
    .to_string();

    testbench.push_str(&format!(
        "
      io_host_steps = {};
      io_used_procs = {};
      ",
        circuit.emulator.host_steps, circuit.emulator.used_procs
    ));

    testbench.push_str(
        "
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
  ",
    );

    // push instructions
    for proc_idx in 0..circuit.emulator.used_procs {
        let proc_insts = circuit
            .emulator
            .instructions
            .get(proc_idx as usize)
            .unwrap();
        for inst_idx in 0..circuit.emulator.host_steps {
            let inst = &proc_insts[inst_idx as usize];
            let bitbuf = inst.to_bytes(&circuit.emulator.cfg);
            let mut bit_u16s = vec![];
            for two_bytes in &bitbuf.bytes.into_iter().chunks(2) {
                let mut bit_u16 = 0;
                for (i, b) in two_bytes.enumerate() {
                    bit_u16 = bit_u16 | (b << (i * 8));
                }
                bit_u16s.push(bit_u16);
            }

            testbench.push_str(
                "
  @(posedge clock);#(0);
  enq_instructions(
  ",
            );
            for (i, b16) in bit_u16s.iter().rev().enumerate() {
                testbench.push_str(&format!("16'h{:x}", b16));
                if i != bit_u16s.len() - 1 {
                    testbench.push_str(", ");
                }
            }
            testbench.push_str(
                "
            );",
            );
        }
    }

    // push inputs
    let cycles = input_stimuli.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        // processor indices that has input bit set has high (1)
        let mut input_bitvec = vec![0, 0, 0, 0];
        for key in input_stimuli.keys() {
            let val = input_stimuli[key].get(cycle);
            match val {
                Some(b) => {
                    if *b == 1 {
                        let nidx = circuit.emulator.signal_map.get(key).unwrap().idx;
                        let ninfo = circuit.graph.node_weight(nidx).unwrap().get_info();
                        let procid = ninfo.proc;
                        let idx = procid / 16;
                        let offset = procid % 16;
                        input_bitvec[idx as usize] |= 1 << offset;
                    }
                }
                None => {}
            }
        }
        testbench.push_str(
            "
  @(posedge clock);#(0);
  enq_inputs(
  ",
        );
        for (i, b16) in input_bitvec.iter().rev().enumerate() {
            testbench.push_str(&format!("16h{:x}", b16));
            if i != input_bitvec.len() - 1 {
                testbench.push_str(", ");
            }
            testbench.push_str(");");
        }

        testbench.push_str(&format!(
            "
  repeat ({}) begin
    @(posedge clock);#(0);
  end
  ",
            circuit.emulator.host_steps
        ));
    }

    // Extra simulation time
    testbench.push_str(&format!(
        "
  repeat ({}) begin
    @(posedge clock);#(0);
  end
  ",
        4 * circuit.emulator.host_steps
    ));

    testbench.push_str(
        "
  $display($time, \" ** End Simulation **=\");
  $finish;
end

// dump the state of the design
// VCD (Value Change Dump) is a standard dump format defined in Verilog.
initial begin
  $dumpfile(\"sim.vcd\");
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
    ",
    );

    return testbench;
}

pub fn generate_emulator_testbench(input_stimuli_path: &str, circuit: &Circuit) -> String {
    let input_stimuli = get_input_stimuli(input_stimuli_path);
    return generate_testbench_string(&input_stimuli, circuit);
}
