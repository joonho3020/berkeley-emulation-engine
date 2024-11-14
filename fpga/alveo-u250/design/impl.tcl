source ../../target.tcl
source $TOP_DIR/au250.tcl

open_checkpoint $PROJECT_DIR/synth/latest/${TOP_MODULE}.dcp

read_xdc $PROJECT_DIR/synth/latest/${TOP_MODULE}.xdc
if {[string trim ${CONSTRAINTS}] ne ""} {
  read_xdc ${CONSTRAINTS}
}

opt_design

place_design

write_checkpoint -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_placed.dcp

phys_opt_design -directive Default

route_design

write_xdc -force $PROJECT_DIR/impl/latest/${TOP_MODULE}_post_route.xdc
write_bitstream -force $PROJECT_DIR/impl/latest/$TOP_MODULE.bit

report_drc -file $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_drc.rpt
report_route_status -file   $PROJECT_DIR/impl/$BUILD_SUFFIX/post_route_route_status.rpt
report_utilization -file    $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_utilization.rpt
report_timing_summary -file $PROJECT_DIR/impl/$BUILD_SUFFIX/impl_timing.rpt

