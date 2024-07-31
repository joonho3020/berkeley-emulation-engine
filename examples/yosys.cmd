read_verilog Fir.sv
hierarchy -check -top Fir
proc; opt -nodffe -nosdff; memory; opt -nodffe -nosdff; fsm; opt -nodffe -nosdff; techmap; opt -nodffe -nosdff;
async2sync;
dffunmap; opt -nodffe -nosdff
abc -lut 3
flatten
opt -nodffe -nosdff;
write_blif -gates Fir.lut.blif
