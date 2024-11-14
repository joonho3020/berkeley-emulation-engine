module XilinxU250Board (
    input pcie_mgt_clkn,
    input pcie_mgt_clkp,
    input pcie_perstn_rst,
    input  [15:0] pci_exp_rxn,
    input  [15:0] pci_exp_rxp,
    output [15:0] pci_exp_txn,
    output [15:0] pci_exp_txp
);

wire sys_clk;
wire sys_clk_gt;
wire axi_aclk;
wire axi_aresetn;

wire [3:0]  m_axi_awid;
wire [63:0] m_axi_awaddr;
wire [7:0] m_axi_awlen;
wire [2:0] m_axi_awsize;
wire [1:0] m_axi_awburst;
wire m_axi_awlock;
wire [3:0] m_axi_awcache;
wire [2:0] m_axi_awprot;
wire m_axi_awvalid;
wire m_axi_awready;

wire [511:0] m_axi_wdata;
wire [63:0] m_axi_wstrb;
wire m_axi_wlast;
wire m_axi_wvalid;
wire m_axi_wready;

wire [3:0] m_axi_bid;
wire [1:0] m_axi_bresp;
wire m_axi_bvalid;
wire m_axi_bready;

wire [3:0] m_axi_arid;
wire [63:0] m_axi_araddr;
wire [7:0] m_axi_arlen;
wire [2:0] m_axi_arsize;
wire [1:0] m_axi_arburst;
wire m_axi_arlock;
wire [3:0] m_axi_arcache;
wire [2:0] m_axi_arprot;
wire m_axi_arvalid;
wire m_axi_arready;

wire [3:0] m_axi_rid;
wire [511:0] m_axi_rdata;
wire [1:0] m_axi_rresp;
wire m_axi_rlast;
wire m_axi_rvalid;
wire m_axi_rready;

wire [31:0] m_axil_awaddr;
wire [2:0] m_axil_awprot;
wire m_axil_awvalid;
wire m_axil_awready;
wire [31:0] m_axil_wdata;
wire [3:0] m_axil_wstrb;
wire m_axil_wvalid;
wire m_axil_wready;
wire m_axil_bvalid;
wire [1:0] m_axil_bresp;
wire m_axil_bready;
wire [31:0] m_axil_araddr;
wire [2:0] m_axil_arprot;
wire m_axil_arvalid;
wire m_axil_arready;
wire [31:0] m_axil_rdata;
wire [1:0] m_axil_rresp;
wire m_axil_rvalid;
wire m_axil_rready;

IBUFDS_GTE4 #(
  .REFCLK_HROW_CK_SEL(2'b00)
)
IBUFDS_inst (
   .O(sys_clk_gt),         // 1-bit output: Refer to Transceiver User Guide.
   .I (pcie_mgt_clkp),     // 1-bit input: Refer to Transceiver User Guide.
   .IB(pcie_mgt_clkn),      // 1-bit input: Refer to Transceiver User Guide.
   .CEB(1'b0),
   .ODIV2(sys_clk)
);

xdma_0 xdma_0 (
  .sys_clk(sys_clk),                                    // input wire sys_clk
  .sys_clk_gt(sys_clk_gt),                                 // input wire sys_clk_gt
  .sys_rst_n(pcie_perstn_rst),                          // input wire sys_rst_n
  .user_lnk_up(),                                       // output wire user_lnk_up
  .pci_exp_txp(pci_exp_txp),                            // output wire [15 : 0] pci_exp_txp
  .pci_exp_txn(pci_exp_txn),                            // output wire [15 : 0] pci_exp_txn
  .pci_exp_rxp(pci_exp_rxp),                            // input wire [15 : 0] pci_exp_rxp
  .pci_exp_rxn(pci_exp_rxn),                            // input wire [15 : 0] pci_exp_rxn
  .axi_aclk(axi_aclk),                                  // output wire axi_aclk
  .axi_aresetn(axi_aresetn),                            // output wire axi_aresetn
  .usr_irq_req(1'b0),                                   // input wire [0 : 0] usr_irq_req
  .usr_irq_ack(),                                       // output wire [0 : 0] usr_irq_ack
  .msi_enable(),                                        // output wire msi_enable
  .msi_vector_width(),                                  // output wire [2 : 0] msi_vector_width

  .m_axi_awid(m_axi_awid),                               // output wire [3 : 0] m_axi_awid
  .m_axi_awaddr(m_axi_awaddr),                          // output wire [63 : 0] m_axi_awaddr
  .m_axi_awlen(m_axi_awlen),                            // output wire [7 : 0] m_axi_awlen
  .m_axi_awsize(m_axi_awsize),                          // output wire [2 : 0] m_axi_awsize
  .m_axi_awburst(m_axi_awburst),                        // output wire [1 : 0] m_axi_awburst
  .m_axi_awlock(m_axi_awlock),                          // output wire m_axi_awlock
  .m_axi_awcache(m_axi_awcache),                        // output wire [3 : 0] m_axi_awcache
  .m_axi_awprot(m_axi_awprot),                          // output wire [2 : 0] m_axi_awprot
  .m_axi_awvalid(m_axi_awvalid),                        // output wire m_axi_awvalid
  .m_axi_awready(m_axi_awready),                        // input wire m_axi_awready

  .m_axi_wdata(m_axi_wdata),                            // output wire [511 : 0] m_axi_wdata
  .m_axi_wstrb(m_axi_wstrb),                            // output wire [63 : 0] m_axi_wstrb
  .m_axi_wlast(m_axi_wlast),                            // output wire m_axi_wlast
  .m_axi_wvalid(m_axi_wvalid),                          // output wire m_axi_wvalid
  .m_axi_wready(m_axi_wready),                          // input wire m_axi_wready

  .m_axi_bid(m_axi_bid),                                // input wire [3 : 0] m_axi_bid
  .m_axi_bresp(m_axi_bresp),                            // input wire [1 : 0] m_axi_bresp
  .m_axi_bvalid(m_axi_bvalid),                          // input wire m_axi_bvalid
  .m_axi_bready(m_axi_bready),                          // output wire m_axi_bready

  .m_axi_arid(m_axi_arid),                              // output wire [3 : 0] m_axi_arid
  .m_axi_araddr(m_axi_araddr),                          // output wire [63 : 0] m_axi_araddr
  .m_axi_arlen(m_axi_arlen),                            // output wire [7 : 0] m_axi_arlen
  .m_axi_arsize(m_axi_arsize),                          // output wire [2 : 0] m_axi_arsize
  .m_axi_arburst(m_axi_arburst),                        // output wire [1 : 0] m_axi_arburst
  .m_axi_arlock(m_axi_arlock),                          // output wire m_axi_arlock
  .m_axi_arcache(m_axi_arcache),                        // output wire [3 : 0] m_axi_arcache
  .m_axi_arprot(m_axi_arprot),                          // output wire [2 : 0] m_axi_arprot
  .m_axi_arvalid(m_axi_arvalid),                        // output wire m_axi_arvalid
  .m_axi_arready(m_axi_arready),                        // input wire m_axi_arready

  .m_axi_rid(m_axi_rid),                                // input wire [3 : 0] m_axi_rid
  .m_axi_rdata(m_axi_rdata),                            // input wire [511 : 0] m_axi_rdata
  .m_axi_rresp(m_axi_rresp),                            // input wire [1 : 0] m_axi_rresp
  .m_axi_rlast(m_axi_rlast),                            // input wire m_axi_rlast
  .m_axi_rvalid(m_axi_rvalid),                          // input wire m_axi_rvalid
  .m_axi_rready(m_axi_rready),                          // output wire m_axi_rready

  .m_axil_awaddr(m_axil_awaddr),                        // output wire [31 : 0] m_axil_awaddr
  .m_axil_awprot(m_axil_awprot),                        // output wire [2 : 0] m_axil_awprot
  .m_axil_awvalid(m_axil_awvalid),                      // output wire m_axil_awvalid
  .m_axil_awready(m_axil_awready),                      // input wire m_axil_awready
  .m_axil_wdata(m_axil_wdata),                          // output wire [31 : 0] m_axil_wdata
  .m_axil_wstrb(m_axil_wstrb),                          // output wire [3 : 0] m_axil_wstrb
  .m_axil_wvalid(m_axil_wvalid),                        // output wire m_axil_wvalid
  .m_axil_wready(m_axil_wready),                        // input wire m_axil_wready
  .m_axil_bvalid(m_axil_bvalid),                        // input wire m_axil_bvalid
  .m_axil_bresp(m_axil_bresp),                          // input wire [1 : 0] m_axil_bresp
  .m_axil_bready(m_axil_bready),                        // output wire m_axil_bready
  .m_axil_araddr(m_axil_araddr),                        // output wire [31 : 0] m_axil_araddr
  .m_axil_arprot(m_axil_arprot),                        // output wire [2 : 0] m_axil_arprot
  .m_axil_arvalid(m_axil_arvalid),                      // output wire m_axil_arvalid
  .m_axil_arready(m_axil_arready),                      // input wire m_axil_arready
  .m_axil_rdata(m_axil_rdata),                          // input wire [31 : 0] m_axil_rdata
  .m_axil_rresp(m_axil_rresp),                          // input wire [1 : 0] m_axil_rresp
  .m_axil_rvalid(m_axil_rvalid),                        // input wire m_axil_rvalid
  .m_axil_rready(m_axil_rready),                        // output wire m_axil_rready

  .cfg_mgmt_addr(),                                     // input wire [18 : 0] cfg_mgmt_addr
  .cfg_mgmt_write(),                                    // input wire cfg_mgmt_write
  .cfg_mgmt_write_data(),                               // input wire [31 : 0] cfg_mgmt_write_data
  .cfg_mgmt_byte_enable(),                              // input wire [3 : 0] cfg_mgmt_byte_enable
  .cfg_mgmt_read(),                                     // input wire cfg_mgmt_read
  .cfg_mgmt_read_data(),                                // output wire [31 : 0] cfg_mgmt_read_data
  .cfg_mgmt_read_write_done()                           // output wire cfg_mgmt_read_write_done
);

wire fpga_top_clock;
wire fpga_top_resetn;

clk_wiz_0 clk_wizard
(
  // Clock out ports
  .clk_out1(fpga_top_clock),     // output clk_out1
    // Status and control signals
    .reset(!axi_aresetn), // input reset
      // Clock in ports
      .clk_in1(axi_aclk)      // input clk_in1
      );


// https://docs.amd.com/v/u/en-US/pg164-proc-sys-reset
proc_sys_reset_0 reset_synchronizer (
  .slowest_sync_clk(fpga_top_clock),          // input wire slowest_sync_clk
  .ext_reset_in(!axi_aresetn),                  // input wire ext_reset_in
  .aux_reset_in(1'b0),                  // input wire aux_reset_in
  .mb_debug_sys_rst(1'b1),          // input wire mb_debug_sys_rst
  .dcm_locked(1'b1),                      // input wire dcm_locked
  .mb_reset(),                          // output wire mb_reset
  .bus_struct_reset(),          // output wire [0 : 0] bus_struct_reset
  .peripheral_reset(),          // output wire [0 : 0] peripheral_reset
  .interconnect_aresetn(fpga_top_resetn),  // output wire [0 : 0] interconnect_aresetn
  .peripheral_aresetn()      // output wire [0 : 0] peripheral_aresetn
);





wire [3 : 0] io_dma_axi4_master_awid;
wire [63 : 0] io_dma_axi4_master_awaddr;
wire [7 : 0] io_dma_axi4_master_awlen;
wire [2 : 0] io_dma_axi4_master_awsize;
wire io_dma_axi4_master_awvalid;
wire io_dma_axi4_master_awready;

wire [511 : 0] io_dma_axi4_master_wdata;
wire [63 : 0] io_dma_axi4_master_wstrb;
wire io_dma_axi4_master_wlast;
wire io_dma_axi4_master_wvalid;
wire io_dma_axi4_master_wready;

wire [3 : 0] io_dma_axi4_master_bid;
wire [1 : 0] io_dma_axi4_master_bresp;
wire io_dma_axi4_master_bvalid;
wire io_dma_axi4_master_bready;

wire [3 : 0] io_dma_axi4_master_arid;
wire [63 : 0] io_dma_axi4_master_araddr;
wire [7 : 0] io_dma_axi4_master_arlen;
wire [2 : 0] io_dma_axi4_master_arsize;
wire io_dma_axi4_master_arvalid;
wire io_dma_axi4_master_arready;

wire [3 : 0] io_dma_axi4_master_rid;
wire [511 : 0] io_dma_axi4_master_rdata;
wire [1 : 0] io_dma_axi4_master_rresp;
wire io_dma_axi4_master_rlast;
wire io_dma_axi4_master_rvalid;
wire io_dma_axi4_master_rready;

axi_cdc axi4_master_cdc (
  .s_axi_aclk(axi_aclk),            // input wire s_axi_aclk
  .s_axi_aresetn(axi_aresetn),      // input wire s_axi_aresetn

  .s_axi_awid(m_axi_awid),          // input wire [3 : 0] s_axi_awid
  .s_axi_awaddr(m_axi_awaddr),      // input wire [63 : 0] s_axi_awaddr
  .s_axi_awlen(m_axi_awlen),        // input wire [7 : 0] s_axi_awlen
  .s_axi_awsize(m_axi_awsize),      // input wire [2 : 0] s_axi_awsize
  .s_axi_awburst(m_axi_awburst),    // input wire [1 : 0] s_axi_awburst
  .s_axi_awlock(m_axi_awlock),      // input wire [0 : 0] s_axi_awlock
  .s_axi_awcache(m_axi_awcache),    // input wire [3 : 0] s_axi_awcache
  .s_axi_awprot(m_axi_awprot),      // input wire [2 : 0] s_axi_awprot
  .s_axi_awregion(4'b0),            // input wire [3 : 0] s_axi_awregion
  .s_axi_awqos(4'b0),               // input wire [3 : 0] s_axi_awqos
  .s_axi_awvalid(m_axi_awvalid),    // input wire s_axi_awvalid
  .s_axi_awready(m_axi_awready),    // output wire s_axi_awready

  .s_axi_wdata(m_axi_wdata),        // input wire [511 : 0] s_axi_wdata
  .s_axi_wstrb(m_axi_wstrb),        // input wire [63 : 0] s_axi_wstrb
  .s_axi_wlast(m_axi_wlast),        // input wire s_axi_wlast
  .s_axi_wvalid(m_axi_wvalid),      // input wire s_axi_wvalid
  .s_axi_wready(m_axi_wready),      // output wire s_axi_wready

  .s_axi_bid(m_axi_bid),            // output wire [3 : 0] s_axi_bid
  .s_axi_bresp(m_axi_bresp),        // output wire [1 : 0] s_axi_bresp
  .s_axi_bvalid(m_axi_bvalid),      // output wire s_axi_bvalid
  .s_axi_bready(m_axi_bready),      // input wire s_axi_bready

  .s_axi_arid(m_axi_arid),          // input wire [3 : 0] s_axi_arid
  .s_axi_araddr(m_axi_araddr),      // input wire [63 : 0] s_axi_araddr
  .s_axi_arlen(m_axi_arlen),        // input wire [7 : 0] s_axi_arlen
  .s_axi_arsize(m_axi_arsize),      // input wire [2 : 0] s_axi_arsize
  .s_axi_arburst(m_axi_arburst),    // input wire [1 : 0] s_axi_arburst
  .s_axi_arlock(m_axi_arlock),      // input wire [0 : 0] s_axi_arlock
  .s_axi_arcache(m_axi_arcache),    // input wire [3 : 0] s_axi_arcache
  .s_axi_arprot(m_axi_arprot),      // input wire [2 : 0] s_axi_arprot
  .s_axi_arregion(4'b0),            // input wire [3 : 0] s_axi_arregion
  .s_axi_arqos(4'b0),               // input wire [3 : 0] s_axi_arqos
  .s_axi_arvalid(m_axi_arvalid),    // input wire s_axi_arvalid
  .s_axi_arready(m_axi_arready),    // output wire s_axi_arready

  .s_axi_rid(m_axi_rid),            // output wire [3 : 0] s_axi_rid
  .s_axi_rdata(m_axi_rdata),        // output wire [511 : 0] s_axi_rdata
  .s_axi_rresp(m_axi_rresp),        // output wire [1 : 0] s_axi_rresp
  .s_axi_rlast(m_axi_rlast),        // output wire s_axi_rlast
  .s_axi_rvalid(m_axi_rvalid),      // output wire s_axi_rvalid
  .s_axi_rready(m_axi_rready),      // input wire s_axi_rready

  .m_axi_aclk(fpga_top_clock),          // input wire m_axi_aclk
  .m_axi_aresetn(fpga_top_resetn),    // input wire m_axi_aresetn

  .m_axi_awid(io_dma_axi4_master_awid),          // output wire [3 : 0] m_axi_awid
  .m_axi_awaddr(io_dma_axi4_master_awaddr),      // output wire [63 : 0] m_axi_awaddr
  .m_axi_awlen(io_dma_axi4_master_awlen),        // output wire [7 : 0] m_axi_awlen
  .m_axi_awsize(io_dma_axi4_master_awsize),      // output wire [2 : 0] m_axi_awsize
  .m_axi_awburst(),                              // output wire [1 : 0] m_axi_awburst
  .m_axi_awlock(),                               // output wire [0 : 0] m_axi_awlock
  .m_axi_awcache(),                              // output wire [3 : 0] m_axi_awcache
  .m_axi_awprot(),                               // output wire [2 : 0] m_axi_awprot
  .m_axi_awregion(),                             // output wire [3 : 0] m_axi_awregion
  .m_axi_awqos(),                                // output wire [3 : 0] m_axi_awqos
  .m_axi_awvalid(io_dma_axi4_master_awvalid),    // output wire m_axi_awvalid
  .m_axi_awready(io_dma_axi4_master_awready),    // input wire m_axi_awready

  .m_axi_wdata(io_dma_axi4_master_wdata),        // output wire [511 : 0] m_axi_wdata
  .m_axi_wstrb(io_dma_axi4_master_wstrb),        // output wire [63 : 0] m_axi_wstrb
  .m_axi_wlast(io_dma_axi4_master_wlast),        // output wire m_axi_wlast
  .m_axi_wvalid(io_dma_axi4_master_wvalid),      // output wire m_axi_wvalid
  .m_axi_wready(io_dma_axi4_master_wready),      // input wire m_axi_wready

  .m_axi_bid(io_dma_axi4_master_bid),            // input wire [3 : 0] m_axi_bid
  .m_axi_bresp(io_dma_axi4_master_bresp),        // input wire [1 : 0] m_axi_bresp
  .m_axi_bvalid(io_dma_axi4_master_bvalid),      // input wire m_axi_bvalid
  .m_axi_bready(io_dma_axi4_master_bready),      // output wire m_axi_bready

  .m_axi_arid(io_dma_axi4_master_arid),          // output wire [3 : 0] m_axi_arid
  .m_axi_araddr(io_dma_axi4_master_araddr),      // output wire [63 : 0] m_axi_araddr
  .m_axi_arlen(io_dma_axi4_master_arlen),        // output wire [7 : 0] m_axi_arlen
  .m_axi_arsize(io_dma_axi4_master_arsize),      // output wire [2 : 0] m_axi_arsize
  .m_axi_arburst(),                              // output wire [1 : 0] m_axi_arburst
  .m_axi_arlock(),                               // output wire [0 : 0] m_axi_arlock
  .m_axi_arcache(),                              // output wire [3 : 0] m_axi_arcache
  .m_axi_arprot(),                               // output wire [2 : 0] m_axi_arprot
  .m_axi_arregion(),                             // output wire [3 : 0] m_axi_arregion
  .m_axi_arqos(),                                // output wire [3 : 0] m_axi_arqos
  .m_axi_arvalid(io_dma_axi4_master_arvalid),    // output wire m_axi_arvalid
  .m_axi_arready(io_dma_axi4_master_arready),    // input wire m_axi_arready

  .m_axi_rid(io_dma_axi4_master_rid),            // input wire [3 : 0] m_axi_rid
  .m_axi_rdata(io_dma_axi4_master_rdata),        // input wire [511 : 0] m_axi_rdata
  .m_axi_rresp(io_dma_axi4_master_rresp),        // input wire [1 : 0] m_axi_rresp
  .m_axi_rlast(io_dma_axi4_master_rlast),        // input wire m_axi_rlast
  .m_axi_rvalid(io_dma_axi4_master_rvalid),      // input wire m_axi_rvalid
  .m_axi_rready(io_dma_axi4_master_rready)       // output wire m_axi_rready
);

wire [31 : 0] io_mmio_axi4_master_awaddr;
wire io_mmio_axi4_master_awvalid;
wire io_mmio_axi4_master_awready;

wire [31 : 0] io_mmio_axi4_master_wdata;
wire [3 : 0] io_mmio_axi4_master_wstrb;
wire io_mmio_axi4_master_wvalid;
wire io_mmio_axi4_master_wready;

wire [1 : 0] io_mmio_axi4_master_bresp;
wire io_mmio_axi4_master_bvalid;
wire io_mmio_axi4_master_bready;

wire [31 : 0] io_mmio_axi4_master_araddr;
wire io_mmio_axi4_master_arvalid;
wire io_mmio_axi4_master_arready;

wire [31 : 0] io_mmio_axi4_master_rdata;
wire [1 : 0] io_mmio_axi4_master_rresp;
wire io_mmio_axi4_master_rvalid;
wire io_mmio_axi4_master_rready;

axi_lite_cdc axi4_lite_master_cdc (
  .s_axi_aclk(axi_aclk),        // input wire s_axi_aclk
  .s_axi_aresetn(axi_aresetn),  // input wire s_axi_aresetn

  .s_axi_awaddr(m_axil_awaddr),    // input wire [31 : 0] s_axi_awaddr
  .s_axi_awprot(m_axil_awprot),    // input wire [2 : 0] s_axi_awprot
  .s_axi_awvalid(m_axil_awvalid),  // input wire s_axi_awvalid
  .s_axi_awready(m_axil_awready),  // output wire s_axi_awready
  .s_axi_wdata(m_axil_wdata),      // input wire [31 : 0] s_axi_wdata
  .s_axi_wstrb(m_axil_wstrb),      // input wire [3 : 0] s_axi_wstrb
  .s_axi_wvalid(m_axil_wvalid),    // input wire s_axi_wvalid
  .s_axi_wready(m_axil_wready),    // output wire s_axi_wready
  .s_axi_bresp(m_axil_bresp),      // output wire [1 : 0] s_axi_bresp
  .s_axi_bvalid(m_axil_bvalid),    // output wire s_axi_bvalid
  .s_axi_bready(m_axil_bready),    // input wire s_axi_bready
  .s_axi_araddr(m_axil_araddr),    // input wire [31 : 0] s_axi_araddr
  .s_axi_arprot(m_axil_arprot),    // input wire [2 : 0] s_axi_arprot
  .s_axi_arvalid(m_axil_arvalid),  // input wire s_axi_arvalid
  .s_axi_arready(m_axil_arready),  // output wire s_axi_arready
  .s_axi_rdata(m_axil_rdata),      // output wire [31 : 0] s_axi_rdata
  .s_axi_rresp(m_axil_rresp),      // output wire [1 : 0] s_axi_rresp
  .s_axi_rvalid(m_axil_rvalid),    // output wire s_axi_rvalid
  .s_axi_rready(m_axil_rready),    // input wire s_axi_rready

  .m_axi_aclk(fpga_top_clock),        // input wire m_axi_aclk
  .m_axi_aresetn(fpga_top_resetn),  // input wire m_axi_aresetn

  .m_axi_awaddr(io_mmio_axi4_master_awaddr),    // output wire [31 : 0] m_axi_awaddr
  .m_axi_awprot(),                              // output wire [2 : 0] m_axi_awprot
  .m_axi_awvalid(io_mmio_axi4_master_awvalid),  // output wire m_axi_awvalid
  .m_axi_awready(io_mmio_axi4_master_awready),  // input wire m_axi_awready

  .m_axi_wdata(io_mmio_axi4_master_wdata),      // output wire [31 : 0] m_axi_wdata
  .m_axi_wstrb(io_mmio_axi4_master_wstrb),      // output wire [3 : 0] m_axi_wstrb
  .m_axi_wvalid(io_mmio_axi4_master_wvalid),    // output wire m_axi_wvalid
  .m_axi_wready(io_mmio_axi4_master_wready),    // input wire m_axi_wready

  .m_axi_bresp(io_mmio_axi4_master_bresp),      // input wire [1 : 0] m_axi_bresp
  .m_axi_bvalid(io_mmio_axi4_master_bvalid),    // input wire m_axi_bvalid
  .m_axi_bready(io_mmio_axi4_master_bready),    // output wire m_axi_bready

  .m_axi_araddr(io_mmio_axi4_master_araddr),    // output wire [31 : 0] m_axi_araddr
  .m_axi_arprot(),    // output wire [2 : 0] m_axi_arprot
  .m_axi_arvalid(io_mmio_axi4_master_arvalid),  // output wire m_axi_arvalid
  .m_axi_arready(io_mmio_axi4_master_arready),  // input wire m_axi_arready

  .m_axi_rdata(io_mmio_axi4_master_rdata),      // input wire [31 : 0] m_axi_rdata
  .m_axi_rresp(io_mmio_axi4_master_rresp),      // input wire [1 : 0] m_axi_rresp
  .m_axi_rvalid(io_mmio_axi4_master_rvalid),    // input wire m_axi_rvalid
  .m_axi_rready(io_mmio_axi4_master_rready)     // output wire m_axi_rready
);

FPGATop fpgatop(
  .clock(fpga_top_clock),
  .reset(!fpga_top_resetn),

  .io_dma_axi4_master_aw_ready(io_dma_axi4_master_awready),
  .io_dma_axi4_master_aw_valid(io_dma_axi4_master_awvalid),
  .io_dma_axi4_master_aw_bits_id(io_dma_axi4_master_awid),
  .io_dma_axi4_master_aw_bits_addr(io_dma_axi4_master_awaddr),
  .io_dma_axi4_master_aw_bits_len(io_dma_axi4_master_awlen),
  .io_dma_axi4_master_aw_bits_size(io_dma_axi4_master_awsize),
  .io_dma_axi4_master_aw_bits_burst(2'h0),
  .io_dma_axi4_master_aw_bits_lock(1'h0),
  .io_dma_axi4_master_aw_bits_cache(4'h0),
  .io_dma_axi4_master_aw_bits_prot(3'h0),
  .io_dma_axi4_master_aw_bits_qos(4'h0),

  .io_dma_axi4_master_w_ready(io_dma_axi4_master_wready),
  .io_dma_axi4_master_w_valid(io_dma_axi4_master_wvalid),
  .io_dma_axi4_master_w_bits_data(io_dma_axi4_master_wdata),
  .io_dma_axi4_master_w_bits_strb(io_dma_axi4_master_wstrb),
  .io_dma_axi4_master_w_bits_last(io_dma_axi4_master_wlast),

  .io_dma_axi4_master_b_ready(io_dma_axi4_master_bready),
  .io_dma_axi4_master_b_valid(io_dma_axi4_master_bvalid),
  .io_dma_axi4_master_b_bits_id(io_dma_axi4_master_bid),
  .io_dma_axi4_master_b_bits_resp(io_dma_axi4_master_bresp),

  .io_dma_axi4_master_ar_ready(io_dma_axi4_master_arready),
  .io_dma_axi4_master_ar_valid(io_dma_axi4_master_arvalid),
  .io_dma_axi4_master_ar_bits_id(io_dma_axi4_master_arid),
  .io_dma_axi4_master_ar_bits_addr(io_dma_axi4_master_araddr),
  .io_dma_axi4_master_ar_bits_len(io_dma_axi4_master_arlen),
  .io_dma_axi4_master_ar_bits_size(io_dma_axi4_master_arsize),
  .io_dma_axi4_master_ar_bits_burst(2'h0),
  .io_dma_axi4_master_ar_bits_lock(1'h0),
  .io_dma_axi4_master_ar_bits_cache(4'h0),
  .io_dma_axi4_master_ar_bits_prot(3'h0),
  .io_dma_axi4_master_ar_bits_qos(4'h0),

  .io_dma_axi4_master_r_ready(io_dma_axi4_master_rready),
  .io_dma_axi4_master_r_valid(io_dma_axi4_master_rvalid),
  .io_dma_axi4_master_r_bits_id(io_dma_axi4_master_rid),
  .io_dma_axi4_master_r_bits_data(io_dma_axi4_master_rdata),
  .io_dma_axi4_master_r_bits_resp(io_dma_axi4_master_rresp),
  .io_dma_axi4_master_r_bits_last(io_dma_axi4_master_rlast),

  .io_mmio_axi4_master_aw_ready(io_mmio_axi4_master_awready),
  .io_mmio_axi4_master_aw_valid(io_mmio_axi4_master_awvalid),
  .io_mmio_axi4_master_aw_bits_id(12'h0),
  .io_mmio_axi4_master_aw_bits_addr(io_mmio_axi4_master_awaddr),
  .io_mmio_axi4_master_aw_bits_len(8'h0),
  .io_mmio_axi4_master_aw_bits_size(3'h2),
  .io_mmio_axi4_master_aw_bits_burst(2'h0),
  .io_mmio_axi4_master_aw_bits_lock(1'h0),
  .io_mmio_axi4_master_aw_bits_cache(4'h0),
  .io_mmio_axi4_master_aw_bits_prot(3'h0),
  .io_mmio_axi4_master_aw_bits_qos(4'h0),

  .io_mmio_axi4_master_w_ready(io_mmio_axi4_master_wready),
  .io_mmio_axi4_master_w_valid(io_mmio_axi4_master_wvalid),
  .io_mmio_axi4_master_w_bits_data(io_mmio_axi4_master_wdata),
  .io_mmio_axi4_master_w_bits_strb(io_mmio_axi4_master_wstrb),
  .io_mmio_axi4_master_w_bits_last(1'h1),

  .io_mmio_axi4_master_b_ready(io_mmio_axi4_master_bready),
  .io_mmio_axi4_master_b_valid(io_mmio_axi4_master_bvalid),
  .io_mmio_axi4_master_b_bits_resp(io_mmio_axi4_master_bresp),
  .io_mmio_axi4_master_b_bits_id(),

  .io_mmio_axi4_master_ar_ready(io_mmio_axi4_master_arready),
  .io_mmio_axi4_master_ar_valid(io_mmio_axi4_master_arvalid),
  .io_mmio_axi4_master_ar_bits_addr(io_mmio_axi4_master_araddr),
  .io_mmio_axi4_master_ar_bits_id(12'h0),
  .io_mmio_axi4_master_ar_bits_len(8'h0),
  .io_mmio_axi4_master_ar_bits_size(3'h2),
  .io_mmio_axi4_master_ar_bits_burst(2'h0),
  .io_mmio_axi4_master_ar_bits_lock(1'h0),
  .io_mmio_axi4_master_ar_bits_cache(4'h0),
  .io_mmio_axi4_master_ar_bits_prot(3'h0),
  .io_mmio_axi4_master_ar_bits_qos(4'h0),

  .io_mmio_axi4_master_r_ready(io_mmio_axi4_master_rready),
  .io_mmio_axi4_master_r_valid(io_mmio_axi4_master_rvalid),
  .io_mmio_axi4_master_r_bits_data(io_mmio_axi4_master_rdata),
  .io_mmio_axi4_master_r_bits_resp(io_mmio_axi4_master_rresp),
  .io_mmio_axi4_master_r_bits_id(),
  .io_mmio_axi4_master_r_bits_last()
);

endmodule
