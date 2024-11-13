source ../../target.tcl
source $TOP_DIR/au250.tcl

if {[string trim ${VLOG_SOURCES}] ne ""} {
  read_verilog -v ${VLOG_SOURCES}
}
if {[string trim ${SVLOG_SOURCES}] ne ""} {
  read_verilog -v ${SVLOG_SOURCES}
}
if {[string trim ${CONSTRAINTS}] ne ""} {
  read_xdc ${CONSTRAINTS}
}
if {[string trim ${IP}] ne ""} {
  read_ip ${IP}
  synth_ip [get_ips]
}

synth_design -top $TOP_MODULE -part $part

write_checkpoint -force $PROJECT_DIR/synth/latest/${TOP_MODULE}.dcp

report_drc            -file $PROJECT_DIR/synth/$BUILD_SUFFIX/post_synth_drc.rpt
report_utilization    -file $PROJECT_DIR/synth/$BUILD_SUFFIX/synth_utilization.rpt
report_timing_summary -file $PROJECT_DIR/synth/$BUILD_SUFFIX/synth_timing.rpt

write_xdc -force $PROJECT_DIR/synth/latest/${TOP_MODULE}.xdc
