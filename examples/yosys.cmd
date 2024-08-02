read_verilog MyQueue.sv
hierarchy -check -top MyQueue
proc; opt -nodffe -nosdff; memory; opt -nodffe -nosdff; fsm; opt -nodffe -nosdff; techmap; opt -nodffe -nosdff;
async2sync;
dffunmap; opt -nodffe -nosdff
flatten
abc -lut 3
opt -nodffe -nosdff;
write_blif -gates MyQueue.lut.blif
