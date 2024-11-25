source ../../target.tcl
source $TOP_DIR/au250.tcl

open_checkpoint $PROJECT_DIR/synth/latest/${TOP_MODULE}.dcp

read_xdc $PROJECT_DIR/synth/latest/${TOP_MODULE}.xdc
if {[string trim ${CONSTRAINTS}] ne ""} {
  read_xdc ${CONSTRAINTS}
}

# Create a pblock for the XDMA module
create_pblock pblock_xdma_0
resize_pblock pblock_xdma_0 -add {SLICE_X152Y240:SLICE_X232Y479 BUFG_GT_X1Y96:BUFG_GT_X1Y191 BUFG_GT_SYNC_X1Y60:BUFG_GT_SYNC_X1Y119 DSP48E2_X21Y96:DSP48E2_X31Y191 RAMB18_X11Y96:RAMB18_X13Y191 RAMB36_X11Y48:RAMB36_X13Y95 URAM288_X3Y64:URAM288_X4Y127} -locs keep_all
add_cells_to_pblock pblock_xdma_0 [get_cells -hierarchical -filter {NAME =~ "xdma_0"}] -clear_locs

# Run PnR
opt_design -directive Explore

place_design

report_timing_summary -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_post_place_timing.rpt

write_checkpoint -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_placed.dcp

phys_opt_design -directive AggressiveExplore

report_timing_summary -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_post_opt_timing.rpt

route_design -directive AggressiveExplore

write_checkpoint -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_routed.dcp
write_xdc -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_post_route.xdc
write_bitstream -force $PROJECT_DIR/impl/latest/$TOP_MODULE.bit

report_drc -file $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_drc.rpt
report_route_status -file   $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_route_status.rpt
report_utilization  -hierarchical -hierarchical_percentages  -file    $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_utilization_hier.rpt
report_utilization  -file    $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_utilization.rpt
report_timing_summary -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_timing.rpt
report_cdc -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_cdc.rpt
