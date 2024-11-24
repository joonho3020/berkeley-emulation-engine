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

set fpga_freq_mhz 100

create_ip -name xdma                 -vendor xilinx.com -library ip -version 4.1 -module_name xdma_0                   -dir $ip_directory
create_ip -name ila                  -vendor xilinx.com -library ip -version 6.2 -module_name ila_0                    -dir $ip_directory
create_ip -name axi_clock_converter  -vendor xilinx.com -library ip -version 2.1 -module_name axi_cdc                  -dir $ip_directory
create_ip -name axi_clock_converter  -vendor xilinx.com -library ip -version 2.1 -module_name axi_lite_cdc             -dir $ip_directory
create_ip -name clk_wiz              -vendor xilinx.com -library ip -version 6.0 -module_name clk_wiz_0                -dir $ip_directory
create_ip -name proc_sys_reset       -vendor xilinx.com -library ip -version 5.0 -module_name proc_sys_reset_0         -dir $ip_directory

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
  CONFIG.C_DATA_DEPTH {1024} \
  CONFIG.C_MONITOR_TYPE {AXI} \
] [get_ips ila_0]

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

set clk_wiz_0 "./ip/clk_wiz_0/clk_wiz_0.xci"
add_files -norecurse $clk_wiz_0
set_property -dict [list \
   CONFIG.CLKOUT1_REQUESTED_OUT_FREQ $fpga_freq_mhz \
   CONFIG.USE_LOCKED {false} \
] [get_ips clk_wiz_0]


set proc_sys_reset_0 "./ip/proc_sys_reset_0/proc_sys_reset_0.xci"
add_files -norecurse $proc_sys_reset_0

generate_target all [get_ips]
close_project
