
read_verilog GCD.sv
proc; opt; memory; opt; fsm; opt; techmap; opt;
async2sync;
abc -lut 3
opt;
write_blif GCD.lut.blif
