create_project -force -part $part $project_name $project_dir 
source au250.tcl

set xdma_ip_path "./ip/xdma_0/xdma_0.xci"
add_files -norecurse $xdma_ip_path
set_property -dict [list \
   CONFIG.axilite_master_en {true} \
   CONFIG.axilite_master_size {32} \
   CONFIG.en_gt_selection {true} \
   CONFIG.mode_selection {Advanced} \
   CONFIG.pl_link_cap_max_link_speed {8.0_GT/s} \
   CONFIG.pl_link_cap_max_link_width {X16} \
   CONFIG.xdma_axi_intf_mm {AXI_Memory_Mapped} \
   CONFIG.xdma_rnum_chnl {4} \
   CONFIG.xdma_wnum_chnl {4} \
   CONFIG.pciebar2axibar_axist_bypass {0x0000000000000000} \
   CONFIG.pf0_msix_cap_pba_bir {BAR_1} \
   CONFIG.pf0_msix_cap_table_bir {BAR_1} \
] [get_ips xdma_0]

set axi_32_path "./ip/axi_lite_bram_ctrl_0_32/axi_lite_bram_ctrl_0_32.xci"
add_files -norecurse $axi_32_path
set_property -dict [list \
  CONFIG.DATA_WIDTH {32} \
  CONFIG.PROTOCOL {AXI4LITE} \
  CONFIG.MEM_DEPTH {1024} \
] [get_ips axi_lite_bram_ctrl_0_32]

set axi_512_path "./ip/axi_bram_ctrl_0_512/axi_bram_ctrl_0_512.xci"
add_files -norecurse $axi_512_path
set_property -dict [list \
  CONFIG.DATA_WIDTH {512} \
  CONFIG.MEM_DEPTH {1024} \
] [get_ips axi_bram_ctrl_0_512]

set bram_0_512 "./ip/bram_0_512/bram_0_512.xci"
add_files -norecurse $bram_0_512
set_property -dict [list \
  CONFIG.Memory_Type {True_Dual_Port_RAM} \
  CONFIG.Write_Width_A {512} \
  CONFIG.Write_Width_B {512} \
  CONFIG.Write_Depth_A {1024} \
] [get_ips bram_0_512]

set bram_0_32 "./ip/bram_0_32/bram_0_32.xci"
add_files -norecurse $bram_0_32
set_property -dict [list \
  CONFIG.Memory_Type {True_Dual_Port_RAM} \
  CONFIG.Write_Width_A {32} \
  CONFIG.Write_Width_B {32} \
  CONFIG.Write_Depth_A {1024} \
] [get_ips bram_0_32]

set ila_0 "./ip/ila_0/ila_0.xci"
add_files -norecurse $ila_0
set_property -dict [list \
  CONFIG.C_DATA_DEPTH {1024} \
  CONFIG.C_MONITOR_TYPE {AXI} \
] [get_ips ila_0]

generate_target all [get_ips]
close_project
