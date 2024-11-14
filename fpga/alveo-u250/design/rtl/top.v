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

assign io_dma_axi4_master_awid = m_axi_awid;
assign io_dma_axi4_master_awaddr = m_axi_awaddr;
assign io_dma_axi4_master_awlen = m_axi_awlen;
assign io_dma_axi4_master_awsize = m_axi_awsize;
assign io_dma_axi4_master_awvalid = m_axi_awvalid;
assign m_axi_awready = io_dma_axi4_master_awready;

assign io_dma_axi4_master_wvalid = m_axi_wvalid;
assign io_dma_axi4_master_wdata = m_axi_wdata;
assign io_dma_axi4_master_wstrb = m_axi_wstrb;
assign io_dma_axi4_master_wlast = m_axi_wlast;
assign m_axi_wready = io_dma_axi4_master_wready;

assign io_dma_axi4_master_bready = m_axi_bready;
assign m_axi_bvalid = io_dma_axi4_master_bvalid;
assign m_axi_bid = io_dma_axi4_master_bid;
assign m_axi_bresp = io_dma_axi4_master_bresp;

assign io_dma_axi4_master_arid = m_axi_arid;
assign io_dma_axi4_master_araddr = m_axi_araddr;
assign io_dma_axi4_master_arlen = m_axi_arlen;
assign io_dma_axi4_master_arsize = m_axi_arsize;
assign io_dma_axi4_master_arvalid = m_axi_arvalid;
assign m_axi_arready = io_dma_axi4_master_arready;

assign io_dma_axi4_master_rready = m_axi_rready;
assign m_axi_rvalid = io_dma_axi4_master_rvalid;
assign m_axi_rid = io_dma_axi4_master_rid;
assign m_axi_rdata = io_dma_axi4_master_rdata;
assign m_axi_rresp = io_dma_axi4_master_rresp;
assign m_axi_rlast = io_dma_axi4_master_rlast;


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


assign io_mmio_axi4_master_awaddr = m_axil_awaddr;
assign io_mmio_axi4_master_awvalid = m_axil_awvalid;
assign m_axil_awready = io_mmio_axi4_master_awready;

assign io_mmio_axi4_master_wstrb = m_axil_wstrb;
assign io_mmio_axi4_master_wdata = m_axil_wdata;
assign io_mmio_axi4_master_wvalid = m_axil_wvalid;
assign m_axil_wready = io_mmio_axi4_master_wready;

assign m_axil_bresp = io_mmio_axi4_master_bresp;
assign m_axil_bvalid = io_mmio_axi4_master_bvalid;
assign io_mmio_axi4_master_bready = m_axil_bready;


assign io_mmio_axi4_master_araddr = m_axil_araddr;
assign io_mmio_axi4_master_arvalid = m_axil_arvalid;
assign m_axil_arready = io_mmio_axi4_master_arready;

assign m_axil_rdata = io_mmio_axi4_master_rdata;
assign m_axil_rresp = io_mmio_axi4_master_rresp;
assign m_axil_rvalid = io_mmio_axi4_master_rvalid;
assign io_mmio_axi4_master_rready = m_axil_rready;

FPGATop fpgatop(
  .clock(axi_aclk),
  .reset(!axi_aresetn),

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
