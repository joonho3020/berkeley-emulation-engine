read_verilog GCD.sv
proc; opt; memory; opt; fsm; opt; techmap; opt;
abc -lut 3
write_blif GCD.lut.blif
