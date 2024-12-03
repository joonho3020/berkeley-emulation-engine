# Creates an IP project to instantiate IPs and place them in the IP directory
# Other projects can read the XCI files generated from this project
set project_name "ip_project"
set project_dir "./ip_project"

create_project $project_name $project_dir -f -part xcu250-figd2104-2l-e -ip
source au250.tcl

set ip_directory "./ip"
if {![file exists $ip_directory]} {
    file mkdir $ip_directory
    puts "Directory created: $ip_directory"
} else {
    puts "Directory already exists: $ip_directory"
}

set fpga_freq_mhz 80

create_ip -name xdma                 -vendor xilinx.com -library ip -version 4.1 -module_name xdma_0            -dir $ip_directory
create_ip -name ila                  -vendor xilinx.com -library ip -version 6.2 -module_name ila_0             -dir $ip_directory
create_ip -name ila                  -vendor xilinx.com -library ip -version 6.2 -module_name ila_1             -dir $ip_directory
create_ip -name ila                  -vendor xilinx.com -library ip -version 6.2 -module_name ila_2             -dir $ip_directory
create_ip -name ila                  -vendor xilinx.com -library ip -version 6.2 -module_name ila_3             -dir $ip_directory
create_ip -name axi_clock_converter  -vendor xilinx.com -library ip -version 2.1 -module_name axi_cdc           -dir $ip_directory
create_ip -name axi_clock_converter  -vendor xilinx.com -library ip -version 2.1 -module_name axi_lite_cdc      -dir $ip_directory
create_ip -name clk_wiz              -vendor xilinx.com -library ip -version 6.0 -module_name clk_wiz_1         -dir $ip_directory
create_ip -name axi_crossbar         -vendor xilinx.com -library ip -version 2.1 -module_name axi_lite_xbar     -dir $ip_directory

set axi_lite_xbar "./ip/axi_lite_xbar/axi_lite_xbar.xci"
add_files -norecurse $axi_lite_xbar
set_property -dict [list \
  CONFIG.PROTOCOL {AXI4LITE} \
  CONFIG.NUM_MI {2} \
  CONFIG.ADDR_RANGES {1} \
  CONFIG.M00_A00_ADDR_WIDTH {12} \
  CONFIG.M00_A00_BASE_ADDR {0x0000000000000000} \
  CONFIG.M01_A00_ADDR_WIDTH {12} \
  CONFIG.M01_A00_BASE_ADDR {0x0000000000010000} \
] [get_ips axi_lite_xbar]

set xdma_ip_path "./ip/xdma_0/xdma_0.xci"
add_files -norecurse $xdma_ip_path
set_property -dict [list \
   CONFIG.axilite_master_en {true} \
   CONFIG.axilite_master_size {32} \
   CONFIG.pl_link_cap_max_link_speed {8.0_GT/s} \
   CONFIG.pl_link_cap_max_link_width {X16} \
   CONFIG.xdma_axi_intf_mm {AXI_Memory_Mapped} \
   CONFIG.xdma_rnum_chnl {4} \
   CONFIG.xdma_wnum_chnl {4} \
   CONFIG.pciebar2axibar_axist_bypass {0x0000000000000000} \
   CONFIG.pf0_msix_cap_pba_bir {BAR_1} \
   CONFIG.pf0_msix_cap_table_bir {BAR_1} \
] [get_ips xdma_0]

set ila_0 "./ip/ila_0/ila_0.xci"
add_files -norecurse $ila_0
set_property -dict [list \
  CONFIG.C_SLOT_0_AXI_ADDR_WIDTH {64} \
  CONFIG.C_SLOT_0_AXI_DATA_WIDTH {512} \
  CONFIG.C_DATA_DEPTH {1024} \
  CONFIG.C_MONITOR_TYPE {AXI} \
] [get_ips ila_0]

set ila_1 "./ip/ila_1/ila_1.xci"
add_files -norecurse $ila_1
set_property -dict [list \
  CONFIG.C_NUM_OF_PROBES {3} \
  CONFIG.C_PROBE0_WIDTH {32} \
  CONFIG.C_PROBE1_WIDTH {32} \
  CONFIG.C_PROBE2_WIDTH {32} \
] [get_ips ila_1]

set ila_2 "./ip/ila_2/ila_2.xci"
add_files -norecurse $ila_1
set_property -dict [list \
  CONFIG.C_NUM_OF_PROBES {1} \
  CONFIG.C_PROBE0_WIDTH {1} \
] [get_ips ila_2]

set ila_3 "./ip/ila_3/ila_3.xci"
add_files -norecurse $ila_1
set_property -dict [list \
  CONFIG.C_NUM_OF_PROBES {3} \
  CONFIG.C_PROBE0_WIDTH {1} \
  CONFIG.C_PROBE1_WIDTH {1} \
  CONFIG.C_PROBE2_WIDTH {1} \
] [get_ips ila_3]

set axi_cdc "./ip/axi_cdc/axi_cdc.xci"
add_files -norecurse $axi_cdc
set_property -dict [list \
  CONFIG.ADDR_WIDTH {64} \
  CONFIG.DATA_WIDTH {512} \
  CONFIG.ID_WIDTH {4} \
] [get_ips axi_cdc]

set axi_lite_cdc "./ip/axi_lite_cdc/axi_lite_cdc.xci"
add_files -norecurse $axi_lite_cdc
set_property -dict [list \
  CONFIG.ARUSER_WIDTH {0} \
  CONFIG.AWUSER_WIDTH {0} \
  CONFIG.BUSER_WIDTH {0} \
  CONFIG.DATA_WIDTH {32} \
  CONFIG.ID_WIDTH {0} \
  CONFIG.PROTOCOL {AXI4LITE} \
  CONFIG.RUSER_WIDTH {0} \
  CONFIG.WUSER_WIDTH {0} \
] [get_ips axi_lite_cdc]

# For 80MHz
set clk_wiz_1 "./ip/clk_wiz_1/clk_wiz_1.xci"
set_property -dict [list \
  CONFIG.CLKIN1_JITTER_PS {33.330000000000005} \
  CONFIG.CLKOUT1_JITTER {106.018} \
  CONFIG.CLKOUT1_PHASE_ERROR {77.836} \
  CONFIG.CLKOUT1_REQUESTED_OUT_FREQ {80.000} \
  CONFIG.MMCM_CLKFBOUT_MULT_F {4.000} \
  CONFIG.MMCM_CLKIN1_PERIOD {3.333} \
  CONFIG.MMCM_CLKIN2_PERIOD {10.0} \
  CONFIG.MMCM_CLKOUT0_DIVIDE_F {15.000} \
  CONFIG.PRIM_IN_FREQ {300.000} \
] [get_ips clk_wiz_1]

generate_target all [get_ips]
close_project
