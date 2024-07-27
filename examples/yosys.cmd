read_verilog GCD.sv
hierarchy -check -top GCD
proc; opt -nodffe -nosdff; memory; opt -nodffe -nosdff; fsm; opt -nodffe -nosdff; techmap; opt -nodffe -nosdff;
async2sync;
dffunmap; opt -nodffe -nosdff
abc -lut 3
flatten
opt -nodffe -nosdff;
write_blif -gates GCD.lut.blif
