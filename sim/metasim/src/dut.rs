#[repr(C)]
pub struct VFPGATop {
    _private: [u8; 0], // Opaque type for FFI
}

#[repr(C)]
pub struct VerilatedVcdC {
    _private: [u8; 0], // Opaque type for FFI
}

extern "C" {

    pub fn FPGATop_new() -> *mut VFPGATop;
    pub fn FPGATop_eval(dut: *mut VFPGATop);
    pub fn FPGATop_delete(dut: *mut VFPGATop);
    pub fn enable_trace(dut: *mut VFPGATop) -> *mut VerilatedVcdC;
    pub fn close_trace(tfp: *mut VerilatedVcdC);
    pub fn dump_vcd(tfp: *mut VerilatedVcdC, timestep: u32);
    pub fn poke_clock (dut: *mut VFPGATop, clock: u64);
    pub fn poke_io_clkwiz_ctrl_axi_aclk (dut: *mut VFPGATop, io_clkwiz_ctrl_axi_aclk: u64);
    pub fn poke_reset (dut: *mut VFPGATop, reset: u64);
    pub fn peek_io_dma_axi4_master_aw_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_dma_axi4_master_aw_valid (dut: *mut VFPGATop, io_dma_axi4_master_aw_valid: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_id (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_id: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_len (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_len: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_size (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_size: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_burst (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_burst: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_lock (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_lock: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_cache (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_cache: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_prot (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_prot: u64);
    pub fn poke_io_dma_axi4_master_aw_bits_qos (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_qos: u64);
    pub fn peek_io_dma_axi4_master_w_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_dma_axi4_master_w_valid (dut: *mut VFPGATop, io_dma_axi4_master_w_valid: u64);
    pub fn poke_io_dma_axi4_master_w_bits_last (dut: *mut VFPGATop, io_dma_axi4_master_w_bits_last: u64);
    pub fn poke_io_dma_axi4_master_b_ready (dut: *mut VFPGATop, io_dma_axi4_master_b_ready: u64);
    pub fn peek_io_dma_axi4_master_b_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_b_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_b_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_ar_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_dma_axi4_master_ar_valid (dut: *mut VFPGATop, io_dma_axi4_master_ar_valid: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_id (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_id: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_len (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_len: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_size (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_size: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_burst (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_burst: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_lock (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_lock: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_cache (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_cache: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_prot (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_prot: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_qos (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_qos: u64);
    pub fn poke_io_dma_axi4_master_r_ready (dut: *mut VFPGATop, io_dma_axi4_master_r_ready: u64);
    pub fn peek_io_dma_axi4_master_r_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_r_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_r_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_dma_axi4_master_r_bits_last (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_mmio_axi4_master_aw_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_mmio_axi4_master_aw_valid (dut: *mut VFPGATop, io_mmio_axi4_master_aw_valid: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_len (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_len: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_size (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_size: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_burst (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_burst: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_lock (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_lock: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_cache (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_cache: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_prot (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_prot: u64);
    pub fn poke_io_mmio_axi4_master_aw_bits_qos (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_qos: u64);
    pub fn peek_io_mmio_axi4_master_w_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_mmio_axi4_master_w_valid (dut: *mut VFPGATop, io_mmio_axi4_master_w_valid: u64);
    pub fn poke_io_mmio_axi4_master_w_bits_strb (dut: *mut VFPGATop, io_mmio_axi4_master_w_bits_strb: u64);
    pub fn poke_io_mmio_axi4_master_w_bits_last (dut: *mut VFPGATop, io_mmio_axi4_master_w_bits_last: u64);
    pub fn poke_io_mmio_axi4_master_b_ready (dut: *mut VFPGATop, io_mmio_axi4_master_b_ready: u64);
    pub fn peek_io_mmio_axi4_master_b_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_mmio_axi4_master_b_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_mmio_axi4_master_ar_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_mmio_axi4_master_ar_valid (dut: *mut VFPGATop, io_mmio_axi4_master_ar_valid: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_len (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_len: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_size (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_size: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_burst (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_burst: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_lock (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_lock: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_cache (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_cache: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_prot (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_prot: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_qos (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_qos: u64);
    pub fn poke_io_mmio_axi4_master_r_ready (dut: *mut VFPGATop, io_mmio_axi4_master_r_ready: u64);
    pub fn peek_io_mmio_axi4_master_r_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_mmio_axi4_master_r_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_mmio_axi4_master_r_bits_last (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_axi_aresetn (dut: *mut VFPGATop, io_clkwiz_ctrl_axi_aresetn: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_aw_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_valid (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_valid: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_len (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_len: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_size (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_size: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_burst (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_burst: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_lock (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_lock: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_cache (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_cache: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_prot (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_prot: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_qos (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_qos: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_w_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_w_valid (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_w_valid: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_w_bits_strb (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_w_bits_strb: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_w_bits_last (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_w_bits_last: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_b_ready (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_b_ready: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_b_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_b_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_ar_ready (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_valid (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_valid: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_len (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_len: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_size (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_size: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_burst (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_burst: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_lock (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_lock: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_cache (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_cache: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_prot (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_prot: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_qos (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_qos: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_r_ready (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_r_ready: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_r_valid (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_r_bits_resp (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_r_bits_last (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_clk_wiz_locked (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_clk_wiz_locked: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_clk_wiz_reset (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_clkwiz_ctrl_ctrl_fpga_top_ctrl_resetn (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_debug_tot_pushed (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_debug_proc_0_init_vec (dut: *mut VFPGATop) -> u64;
    pub fn peek_io_debug_proc_n_init_vec (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_mmio_axi4_master_aw_bits_id (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_id: u64);
    pub fn peek_io_mmio_axi4_master_b_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_mmio_axi4_master_ar_bits_id (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_id: u64);
    pub fn peek_io_mmio_axi4_master_r_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_id (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_id: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_b_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_id (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_id: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_r_bits_id (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_dma_axi4_master_w_bits_data (dut: *mut VFPGATop, io_dma_axi4_master_w_bits_data: *const u32);
    pub fn peek_io_dma_axi4_master_r_bits_data (dut: *mut VFPGATop, io_dma_axi4_master_r_bits_data: *mut u32);
    pub fn poke_io_mmio_axi4_master_aw_bits_addr (dut: *mut VFPGATop, io_mmio_axi4_master_aw_bits_addr: u64);
    pub fn poke_io_mmio_axi4_master_w_bits_data (dut: *mut VFPGATop, io_mmio_axi4_master_w_bits_data: u64);
    pub fn poke_io_mmio_axi4_master_ar_bits_addr (dut: *mut VFPGATop, io_mmio_axi4_master_ar_bits_addr: u64);
    pub fn peek_io_mmio_axi4_master_r_bits_data (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_aw_bits_addr (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_aw_bits_addr: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_w_bits_data (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_w_bits_data: u64);
    pub fn poke_io_clkwiz_ctrl_ctrl_axil_ar_bits_addr (dut: *mut VFPGATop, io_clkwiz_ctrl_ctrl_axil_ar_bits_addr: u64);
    pub fn peek_io_clkwiz_ctrl_ctrl_axil_r_bits_data (dut: *mut VFPGATop) -> u64;
    pub fn poke_io_dma_axi4_master_aw_bits_addr (dut: *mut VFPGATop, io_dma_axi4_master_aw_bits_addr: u64);
    pub fn poke_io_dma_axi4_master_w_bits_strb (dut: *mut VFPGATop, io_dma_axi4_master_w_bits_strb: u64);
    pub fn poke_io_dma_axi4_master_ar_bits_addr (dut: *mut VFPGATop, io_dma_axi4_master_ar_bits_addr: u64);
} // extern "C"

