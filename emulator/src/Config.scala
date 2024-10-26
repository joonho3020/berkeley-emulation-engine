package emulator

import scala.collection.mutable.Map
import chisel3._
import chisel3.util._

case class Coordinate(mod: Int, proc: Int)

case class EmulatorConfig(
  max_steps:   Int = 128,        // Maximum host steps that can be run
  num_bits:    Int = 1,           // Width of the datapath
  num_procs:   Int = 8,          // Number of processor in a module
  num_mods:    Int = 9,          // Number of modules in a board
  imem_lat:    Int = 1,           // Instruction memory read latency
  dmem_rd_lat: Int = 0,           // Data memory read latency
  num_prims:   Int = 9,           // Number of primitives
  inter_proc_nw_lat: Int = 0,     // NW latency within a module
  inter_mod_nw_lat:  Int = 0,     // NW latency across modules
  lut_inputs: Int = 3,            // Number of lut inputs
  ireg_skip:  Int = 4,            // Insert queues for instruction scan chain every ireg_skip processors
  debug:      Boolean = false     // Insert debug bundles
) {
  // TODO: there requirements exists because I didn't write RTL for it
  // The compiler supports changing these flags
  require(dmem_rd_lat == 0)
  require(dmem_rd_lat <= 1)
  require(imem_lat <= 1)
  require(inter_proc_nw_lat == 0)
  require(inter_mod_nw_lat  == 0)

  val index_bits = log2Ceil(max_steps)
  val switch_bits = log2Ceil(num_procs)
  val opcode_bits = log2Ceil(num_prims)
  val lut_bits    = 1 << lut_inputs
  val dmem_bits   = max_steps * num_bits
  val fetch_decode_lat = imem_lat + dmem_rd_lat

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
