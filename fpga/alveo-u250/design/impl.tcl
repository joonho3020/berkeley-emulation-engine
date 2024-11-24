source ../../target.tcl
source $TOP_DIR/au250.tcl

open_checkpoint $PROJECT_DIR/synth/latest/${TOP_MODULE}.dcp

read_xdc $PROJECT_DIR/synth/latest/${TOP_MODULE}.xdc
if {[string trim ${CONSTRAINTS}] ne ""} {
  read_xdc ${CONSTRAINTS}
}

# Create a pblock for the XDMA module
create_pblock pblock_xdma_0
resize_pblock pblock_xdma_0 -add {SLICE_X176Y0:SLICE_X232Y239 BUFG_GT_X1Y0:BUFG_GT_X1Y95 BUFG_GT_SYNC_X1Y0:BUFG_GT_SYNC_X1Y59 DSP48E2_X24Y0:DSP48E2_X31Y95 RAMB18_X11Y0:RAMB18_X13Y95 RAMB36_X11Y0:RAMB36_X13Y47 URAM288_X4Y0:URAM288_X4Y63} -locs keep_all
add_cells_to_pblock pblock_xdma_0 [get_cells -hierarchical -filter {NAME = ~ "*/xdma_0"}] -clear_locs
set_property IS_SOFT true [get_pblocks pblock_xdma]

# Run PnR
opt_design

place_design

write_checkpoint -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_placed.dcp

phys_opt_design -directive Default

route_design

write_xdc -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_post_route.xdc
write_bitstream -force $PROJECT_DIR/impl/latest/$TOP_MODULE.bit

report_drc -file $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_drc.rpt
report_route_status -file   $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_route_status.rpt
report_utilization  -hierarchical -hierarchical_percentages  -file    $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_utilization_hier.rpt
report_utilization  -file    $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_utilization.rpt
report_timing_summary -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_timing.rpt
