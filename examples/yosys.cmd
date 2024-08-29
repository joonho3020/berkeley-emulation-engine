read_verilog Tile.sv
hierarchy -check -top Tile
proc; opt -nodffe -nosdff; memory; opt -nodffe -nosdff; fsm; opt -nodffe -nosdff; techmap; opt -nodffe -nosdff;
async2sync;
dffunmap; opt -nodffe -nosdff
flatten
opt_clean -purge
abc -fast -lut 3
opt_clean -purge
opt -nodffe -nosdff;
write_blif -gates Tile.lut.blif
