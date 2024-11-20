package emulator

import scala.collection.mutable.Map
import chisel3._
import chisel3.util._

case class Coordinate(mod: Int, proc: Int)

case class EmulatorConfig(
  max_steps:   Int = 128,        // Maximum host steps that can be run
  num_bits:    Int = 1,          // Width of the datapath
  num_procs:   Int = 8,          // Number of processor in a module
  num_mods:    Int = 9,          // Number of modules in a board
  imem_lat:    Int = 1,          // Instruction memory read latency
  dmem_rd_lat: Int = 0,          // Data memory read latency
  num_prims:   Int = 9,          // Number of primitives
  inter_proc_nw_lat: Int = 0,    // NW latency within a module
  inter_mod_nw_lat:  Int = 0,    // NW latency across modules
  lut_inputs: Int = 3,           // Number of lut inputs
  ireg_skip:  Int = 4,           // Insert queues for instruction scan chain every ireg_skip processors
  sram_width: Int = 16,          // SRAM width in bits
  sram_entries: Int = 16,        // Number of SRAM entries
  sram_wr_lat: Int  = 1,         // Number of cycles to perform SRAM write
  sram_rd_lat: Int  = 1,         // Number of cycles to perform SRAM reads
  blackbox_dmem: Boolean = false, // Use blackbox datamemory (for FPGA lutram mapping)
  debug:      Boolean = false    // Insert debug bundles
) {
  require(num_bits  == 1)

  // TODO: these requirements exists because I didn't write RTL for it
  // The compiler supports changing these flags
  require(dmem_rd_lat == 0)
  require(dmem_rd_lat <= 1)

  // These requirements are HW limitations
  require(imem_lat <= 1)
  require(sram_wr_lat == 1)
  require(sram_rd_lat == 1)
  require((sram_width & (sram_width - 1)) == 0)

  // Processor related parameters
  val index_bits = log2Ceil(max_steps)
  val switch_bits = log2Ceil(num_procs)
  val opcode_bits = log2Ceil(num_prims)
  val lut_bits    = 1 << lut_inputs
  val dmem_bits   = max_steps * num_bits
  val fetch_decode_lat = imem_lat + dmem_rd_lat

  // ...
  val insts_per_mod = num_procs * max_steps


  // SRAM processor related parameters
  val sram_addr_bits = log2Ceil(sram_entries)
  val sram_width_bits = log2Ceil(sram_width)

  val sram_rd_en_offset     = 0
  val sram_wr_en_offset     = sram_rd_en_offset + 1
  val sram_rd_addr_offset   = sram_wr_en_offset + 1
  val sram_wr_addr_offset   = sram_rd_addr_offset + sram_entries
  val sram_wr_data_offset   = sram_wr_addr_offset + sram_entries
  val sram_wr_mask_offset   = sram_wr_data_offset + sram_width
  val sram_rdwr_en_offset   = sram_wr_mask_offset + sram_width
  val sram_rdwr_mode_offset = sram_rdwr_en_offset + 1
  val sram_rdwr_addr_offset = sram_rdwr_mode_offset + 1
  val sram_other_offset     = sram_rdwr_addr_offset + sram_entries

  val sram_offset_decode_bits = sram_addr_bits.max(sram_width_bits)
  val sram_addr_width_max = sram_addr_bits.max(sram_width)

  val sram_unique_indices = 1 + sram_other_offset
  val sram_unique_indices_bits = log2Ceil(sram_unique_indices)

  def global_network_topology: Map[Coordinate, Coordinate] = {
    var ret: Map[Coordinate, Coordinate] = Map()
    if (num_mods == 1) {
      return ret
    }
    val num_mods_1 = num_mods - 1;
    val grp_sz = num_procs / num_mods_1

    require((num_mods_1 & (num_mods_1 - 1)) == 0)
    require((num_procs  & (num_procs  - 1)) == 0)
    require( num_procs  >= num_mods_1)

    for (m <- 0 until num_mods_1) {
      for (p <- 0 until num_procs) {
        val r = p % grp_sz;
        val q = (p - r) / grp_sz;
        val src = Coordinate(m, p)
        val dst = if (q == m) {
          Coordinate(num_mods_1, p)
        } else {
          Coordinate(q, m * grp_sz + r)
        }
        ret.put(src, dst)
        ret.put(dst, src)
      }
    }
    return ret
  }
}
