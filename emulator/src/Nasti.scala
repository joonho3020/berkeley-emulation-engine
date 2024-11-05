package emulator

import chisel3._
import chisel3.util._
import scala.math.{min, max}
import freechips.rocketchip.amba.axi4._
import freechips.rocketchip.util.{DecoupledHelper, ParameterizedBundle, HellaPeekingArbiter}
import org.chipsalliance.cde.config.{Parameters, Field}

case class NastiParameters(dataBits: Int, addrBits: Int, idBits: Int) {
  val nastiXDataBits   = dataBits
  val nastiWStrobeBits = nastiXDataBits / 8
  val nastiXAddrBits   = addrBits
  val nastiWIdBits     = idBits
  val nastiRIdBits     = idBits
  val nastiXIdBits     = max(nastiWIdBits, nastiRIdBits)
  val nastiXUserBits   = 1
  val nastiAWUserBits  = nastiXUserBits
  val nastiWUserBits   = nastiXUserBits
  val nastiBUserBits   = nastiXUserBits
  val nastiARUserBits  = nastiXUserBits
  val nastiRUserBits   = nastiXUserBits
  val nastiXLenBits    = 8
  val nastiXSizeBits   = 3
  val nastiXBurstBits  = 2
  val nastiXCacheBits  = 4
  val nastiXProtBits   = 3
  val nastiXQosBits    = 4
  val nastiXRegionBits = 4
  val nastiXRespBits   = 2

  def bytesToXSize(bytes: UInt) = MuxLookup(bytes, UInt("b111"), Array(
    UInt(1) -> UInt(0),
    UInt(2) -> UInt(1),
    UInt(4) -> UInt(2),
    UInt(8) -> UInt(3),
    UInt(16) -> UInt(4),
    UInt(32) -> UInt(5),
    UInt(64) -> UInt(6),
    UInt(128) -> UInt(7)))
}

object NastiParameters {
  def apply(params: AXI4BundleParameters): NastiParameters =
    NastiParameters(params.dataBits, params.addrBits, params.idBits)
}

abstract class NastiModule(val cfg: NastiParameters) extends Module
abstract class NastiBundle(val cfg: NastiParameters) extends Bundle

abstract class NastiChannel             (val cfg: NastiParameters) extends NastiBundle (cfg)
abstract class NastiMasterToSlaveChannel(val cfg: NastiParameters) extends NastiChannel(cfg)
abstract class NastiSlaveToMasterChannel(val cfg: NastiParameters) extends NastiChannel(cfg)

class NastiReadIO(val cfg: NastiParameters) extends NastiBundle(cfg) {
  val ar =         Decoupled(new NastiReadAddressChannel(cfg))
  val r  = Flipped(Decoupled(new NastiReadDataChannel   (cfg)))
}

class NastiWriteIO(val cfg: NastiParameters) extends NastiBundle(cfg) {
  val aw =         Decoupled(new NastiWriteAddressChannel (cfg))
  val w  =         Decoupled(new NastiWriteDataChannel    (cfg))
  val b  = Flipped(Decoupled(new NastiWriteResponseChannel(cfg)))
}

class NastiIO(val cfg: NastiParameters) extends NastiBundle(cfg) {
  val aw =         Decoupled(new NastiWriteAddressChannel (cfg))
  val w  =         Decoupled(new NastiWriteDataChannel    (cfg))
  val b  = Flipped(Decoupled(new NastiWriteResponseChannel(cfg)))
  val ar =         Decoupled(new NastiReadAddressChannel  (cfg))
  val r  = Flipped(Decoupled(new NastiReadDataChannel     (cfg)))
}

class NastiAddressChannel(val cfg: NastiParameters) extends NastiMasterToSlaveChannel(cfg) {
  import cfg._
  val addr   = UInt(width = nastiXAddrBits)
  val len    = UInt(width = nastiXLenBits)
  val size   = UInt(width = nastiXSizeBits)
  val burst  = UInt(width = nastiXBurstBits)
  val lock   = Bool()
  val cache  = UInt(width = nastiXCacheBits)
  val prot   = UInt(width = nastiXProtBits)
  val qos    = UInt(width = nastiXQosBits)
  val region = UInt(width = nastiXRegionBits)
}

class NastiResponseChannel(val cfg: NastiParameters) extends NastiSlaveToMasterChannel(cfg) {
  val resp = UInt(width = nastiXRespBits)
}

class NastiWriteAddressChannel(val cfg: NastiParameters) extends NastiAddressChannel(cfg) {
  val id   = UInt(width = nastiWIdBits)
  val user = UInt(width = nastiAWUserBits)
}

class NastiWriteDataChannel(val cfg: NastiParameters) extends NastiMasterToSlaveChannel(cfg) {
  val data = UInt(width = nastiXDataBits)
  val last = Bool()
  val id   = UInt(width = nastiWIdBits)
  val strb = UInt(width = nastiWStrobeBits)
  val user = UInt(width = nastiWUserBits)
}

class NastiWriteResponseChannel(val cfg: NastiParameters) extends NastiResponseChannel(cfg) {
  val id   = UInt(width = nastiWIdBits)
  val user = UInt(width = nastiBUserBits)
}

class NastiReadAddressChannel(val cfg: NastiParameters) extends NastiAddressChannel(cfg) {
  val id   = UInt(width = nastiRIdBits)
  val user = UInt(width = nastiARUserBits)
}

class NastiReadDataChannel(val cfg: NastiParameters) extends NastiResponseChannel(cfg) {
  val data = UInt(width = nastiXDataBits)
  val last = Bool()
  val id   = UInt(width = nastiRIdBits)
  val user = UInt(width = nastiRUserBits)
}

object NastiConstants {
  val BURST_FIXED = UInt("b00")
  val BURST_INCR  = UInt("b01")
  val BURST_WRAP  = UInt("b10")

  val RESP_OKAY = UInt("b00")
  val RESP_EXOKAY = UInt("b01")
  val RESP_SLVERR = UInt("b10")
  val RESP_DECERR = UInt("b11")

  val CACHE_DEVICE_NOBUF = UInt("b0000")
  val CACHE_DEVICE_BUF   = UInt("b0001")
  val CACHE_NORMAL_NOCACHE_NOBUF = UInt("b0010")
  val CACHE_NORMAL_NOCACHE_BUF   = UInt("b0011")

  def AXPROT(instruction: Bool, nonsecure: Bool, privileged: Bool): UInt =
    Cat(instruction, nonsecure, privileged)

  def AXPROT(instruction: Boolean, nonsecure: Boolean, privileged: Boolean): UInt =
    AXPROT(Bool(instruction), Bool(nonsecure), Bool(privileged))
}

import NastiConstants._

object NastiWriteAddressChannel {
  def apply(id: UInt, addr: UInt, size: UInt,
      len: UInt = UInt(0), burst: UInt = BURST_INCR)
      (cfg: NastiParameters) = {
    val aw = Wire(new NastiWriteAddressChannel(cfg))
    aw.id := id
    aw.addr := addr
    aw.len := len
    aw.size := size
    aw.burst := burst
    aw.lock := false.B
    aw.cache := CACHE_DEVICE_NOBUF
    aw.prot := AXPROT(false, false, false)
    aw.qos := UInt("b0000")
    aw.region := UInt("b0000")
    aw.user := UInt(0)
    aw
  }
}

object NastiReadAddressChannel {
  def apply(id: UInt, addr: UInt, size: UInt,
      len: UInt = UInt(0), burst: UInt = BURST_INCR)
      (cfg: NastiParameters) = {
    val ar = Wire(new NastiReadAddressChannel(cfg))
    ar.id := id
    ar.addr := addr
    ar.len := len
    ar.size := size
    ar.burst := burst
    ar.lock := false.B
    ar.cache := CACHE_DEVICE_NOBUF
    ar.prot := AXPROT(false, false, false)
    ar.qos := UInt(0)
    ar.region := UInt(0)
    ar.user := UInt(0)
    ar
  }
}

object NastiWriteDataChannel {
  def apply(data: UInt, strb: Option[UInt] = None,
            last: Bool = true.B, id: UInt = UInt(0))
           (cfg: NastiParameters): NastiWriteDataChannel = {
    val w = Wire(new NastiWriteDataChannel(cfg))
    w.strb := strb.getOrElse(Fill(w.nastiWStrobeBits, UInt(1, 1)))
    w.data := data
    w.last := last
    w.id   := id
    w.user := UInt(0)
    w
  }
}

object NastiReadDataChannel {
  def apply(id: UInt, data: UInt, last: Bool = true.B, resp: UInt = UInt(0))(
      cfg: NastiParameters) = {
    val r = Wire(new NastiReadDataChannel(cfg))
    r.id := id
    r.data := data
    r.last := last
    r.resp := resp
    r.user := UInt(0)
    r
  }
}

object NastiWriteResponseChannel {
  def apply(id: UInt, resp: UInt = UInt(0))(cfg: NastiParameters) = {
    val b = Wire(new NastiWriteResponseChannel(cfg))
    b.id := id
    b.resp := resp
    b.user := UInt(0)
    b
  }
}

class NastiQueue(depth: Int)(val cfg: NastiParameters) extends Module {
  val io = new Bundle {
    val in  = Flipped(new NastiIO)
    val out =         new NastiIO
  }
  io.out.ar <> Queue(io.in.ar, depth)
  io.out.aw <> Queue(io.in.aw, depth)
  io.out.w  <> Queue(io.in.w,  depth)
  io.in.r   <> Queue(io.out.r, depth)
  io.in.b   <> Queue(io.out.b, depth)
}

object NastiQueue {
  def apply(in: NastiIO, depth: Int = 2)(cfg: NastiParameters): NastiIO = {
    val queue = Module(new NastiQueue(depth)(cfg))
    queue.io.in <> in
    queue.io.out
  }
}

class NastiArbiterIO(arbN: Int)(val cfg: NastiParameters) extends Bundle {
  val master = Flipped(Vec(arbN, new NastiIO))
  val slave  = new NastiIO
}

/** Arbitrate among arbN masters requesting to a single slave */
class NastiArbiter(val arbN: Int)(val cfg: NastiParameters) extends NastiModule {
  val io = new NastiArbiterIO(arbN)

  if (arbN > 1) {
    val arbIdBits = log2Up(arbN)

    val ar_arb = Module(new RRArbiter(new NastiReadAddressChannel, arbN))
    val aw_arb = Module(new RRArbiter(new NastiWriteAddressChannel, arbN))

    val w_chosen = Reg(UInt(width = arbIdBits))
    val w_done = Reg(init = true.B)

    when (aw_arb.io.out.fire) {
      w_chosen := aw_arb.io.chosen
      w_done := false.B
    }

    when (io.slave.w.fire && io.slave.w.bits.last) {
      w_done := true.B
    }

    val queueSize = min((1 << nastiXIdBits) * arbN, 64)

    val rroq = Module(new ReorderQueue(
      UInt(width = arbIdBits), nastiXIdBits, Some(queueSize)))

    val wroq = Module(new ReorderQueue(
      UInt(width = arbIdBits), nastiXIdBits, Some(queueSize)))

    for (i <- 0 until arbN) {
      val m_ar = io.master(i).ar
      val m_aw = io.master(i).aw
      val m_r = io.master(i).r
      val m_b = io.master(i).b
      val a_ar = ar_arb.io.in(i)
      val a_aw = aw_arb.io.in(i)
      val m_w = io.master(i).w

      a_ar <> m_ar
      a_aw <> m_aw

      m_r.valid := io.slave.r.valid && rroq.io.deq.head.data === UInt(i)
      m_r.bits := io.slave.r.bits

      m_b.valid := io.slave.b.valid && wroq.io.deq.head.data === UInt(i)
      m_b.bits := io.slave.b.bits

      m_w.ready := io.slave.w.ready && w_chosen === UInt(i) && !w_done
    }

    io.slave.r.ready := io.master(rroq.io.deq.head.data).r.ready
    io.slave.b.ready := io.master(wroq.io.deq.head.data).b.ready

    rroq.io.deq.head.tag := io.slave.r.bits.id
    rroq.io.deq.head.valid := io.slave.r.fire && io.slave.r.bits.last
    wroq.io.deq.head.tag := io.slave.b.bits.id
    wroq.io.deq.head.valid := io.slave.b.fire

    assert(!rroq.io.deq.head.valid || rroq.io.deq.head.matches,
      "NastiArbiter: read response mismatch")
    assert(!wroq.io.deq.head.valid || wroq.io.deq.head.matches,
      "NastiArbiter: write response mismatch")

    io.slave.w.bits := io.master(w_chosen).w.bits
    io.slave.w.valid := io.master(w_chosen).w.valid && !w_done

    val ar_helper = DecoupledHelper(
      ar_arb.io.out.valid,
      io.slave.ar.ready,
      rroq.io.enq.ready)

    io.slave.ar.valid := ar_helper.fire(io.slave.ar.ready)
    io.slave.ar.bits := ar_arb.io.out.bits
    ar_arb.io.out.ready := ar_helper.fire(ar_arb.io.out.valid)
    rroq.io.enq.valid := ar_helper.fire(rroq.io.enq.ready)
    rroq.io.enq.bits.tag := ar_arb.io.out.bits.id
    rroq.io.enq.bits.data := ar_arb.io.chosen

    val aw_helper = DecoupledHelper(
      aw_arb.io.out.valid,
      io.slave.aw.ready,
      wroq.io.enq.ready)

    io.slave.aw.bits <> aw_arb.io.out.bits
    io.slave.aw.valid := aw_helper.fire(io.slave.aw.ready, w_done)
    aw_arb.io.out.ready := aw_helper.fire(aw_arb.io.out.valid, w_done)
    wroq.io.enq.valid := aw_helper.fire(wroq.io.enq.ready, w_done)
    wroq.io.enq.bits.tag := aw_arb.io.out.bits.id
    wroq.io.enq.bits.data := aw_arb.io.chosen

  } else { io.slave <> io.master.head }
}

/** A slave that send decode error for every request it receives */
class NastiErrorSlave(val cfg: NastiParameters) extends NastiModule {
  val io = Flipped(new NastiIO)

  when (io.ar.fire) { printf("Invalid read address %x\n", io.ar.bits.addr) }
  when (io.aw.fire) { printf("Invalid write address %x\n", io.aw.bits.addr) }

  val r_queue = Module(new Queue(new NastiReadAddressChannel, 1))
  r_queue.io.enq <> io.ar

  val responding = Reg(init = false.B)
  val beats_left = Reg(init = UInt(0, nastiXLenBits))

  when (!responding && r_queue.io.deq.valid) {
    responding := true.B
    beats_left := r_queue.io.deq.bits.len
  }

  io.r.valid := r_queue.io.deq.valid && responding
  io.r.bits.id := r_queue.io.deq.bits.id
  io.r.bits.data := UInt(0)
  io.r.bits.resp := RESP_DECERR
  io.r.bits.last := beats_left === UInt(0)

  r_queue.io.deq.ready := io.r.fire && io.r.bits.last

  when (io.r.fire) {
    when (beats_left === UInt(0)) {
      responding := false.B
    } .otherwise {
      beats_left := beats_left - UInt(1)
    }
  }

  val draining = Reg(init = false.B)
  io.w.ready := draining

  when (io.aw.fire) { draining := true.B }
  when (io.w.fire && io.w.bits.last) { draining := false.B }

  val b_queue = Module(new Queue(UInt(width = nastiWIdBits), 1))
  b_queue.io.enq.valid := io.aw.valid && !draining
  b_queue.io.enq.bits := io.aw.bits.id
  io.aw.ready := b_queue.io.enq.ready && !draining
  io.b.valid := b_queue.io.deq.valid && !draining
  io.b.bits.id := b_queue.io.deq.bits
  io.b.bits.resp := RESP_DECERR
  b_queue.io.deq.ready := io.b.ready && !draining
}

class NastiRouterIO(nSlaves: Int)(val cfg: NastiParameters) extends Bundle {
  val master = Flipped(new NastiIO)
  val slave = Vec(nSlaves, new NastiIO)
}

/** Take a single Nasti master and route its requests to various slaves
 *  @param nSlaves the number of slaves
 *  @param routeSel a function which takes an address and produces
 *  a one-hot encoded selection of the slave to write to */
class NastiRouter(nSlaves: Int, routeSel: UInt => UInt)(val cfg: NastiParameters)
    extends NastiModule {

  val io = new NastiRouterIO(nSlaves)

  val ar_route = routeSel(io.master.ar.bits.addr)
  val aw_route = routeSel(io.master.aw.bits.addr)

  val ar_ready = Wire(init = false.B)
  val aw_ready = Wire(init = false.B)
  val w_ready = Wire(init = false.B)

  val queueSize = min((1 << nastiXIdBits) * nSlaves, 64)

  // These reorder queues remember which slave ports requests were sent on
  // so that the responses can be sent back in-order on the master
  val ar_queue = Module(new ReorderQueue(
    UInt(width = log2Up(nSlaves + 1)), nastiXIdBits,
    Some(queueSize), nSlaves + 1))
  val aw_queue = Module(new ReorderQueue(
    UInt(width = log2Up(nSlaves + 1)), nastiXIdBits,
    Some(queueSize), nSlaves + 1))
  // This queue holds the accepted aw_routes so that we know how to route the
  val w_queue = Module(new Queue(aw_route, nSlaves))

  val ar_helper = DecoupledHelper(
    io.master.ar.valid,
    ar_queue.io.enq.ready,
    ar_ready)

  val aw_helper = DecoupledHelper(
    io.master.aw.valid,
    w_queue.io.enq.ready,
    aw_queue.io.enq.ready,
    aw_ready)

  val w_helper = DecoupledHelper(
    io.master.w.valid,
    w_queue.io.deq.valid,
    w_ready)

  def routeEncode(oh: UInt): UInt = Mux(oh.orR, OHToUInt(oh), UInt(nSlaves))

  ar_queue.io.enq.valid := ar_helper.fire(ar_queue.io.enq.ready)
  ar_queue.io.enq.bits.tag := io.master.ar.bits.id
  ar_queue.io.enq.bits.data := routeEncode(ar_route)

  aw_queue.io.enq.valid := aw_helper.fire(aw_queue.io.enq.ready)
  aw_queue.io.enq.bits.tag := io.master.aw.bits.id
  aw_queue.io.enq.bits.data := routeEncode(aw_route)

  w_queue.io.enq.valid := aw_helper.fire(w_queue.io.enq.ready)
  w_queue.io.enq.bits := aw_route
  w_queue.io.deq.ready := w_helper.fire(w_queue.io.deq.valid, io.master.w.bits.last)

  io.master.ar.ready := ar_helper.fire(io.master.ar.valid)
  io.master.aw.ready := aw_helper.fire(io.master.aw.valid)
  io.master.w.ready := w_helper.fire(io.master.w.valid)

  val ar_valid = ar_helper.fire(ar_ready)
  val aw_valid = aw_helper.fire(aw_ready)
  val w_valid = w_helper.fire(w_ready)
  val w_route = w_queue.io.deq.bits

  io.slave.zipWithIndex.foreach { case (s, i) =>
    s.ar.valid := ar_valid && ar_route(i)
    s.ar.bits := io.master.ar.bits
    when (ar_route(i)) { ar_ready := s.ar.ready }

    s.aw.valid := aw_valid && aw_route(i)
    s.aw.bits := io.master.aw.bits
    when (aw_route(i)) { aw_ready := s.aw.ready }

    s.w.valid := w_valid && w_route(i)
    s.w.bits := io.master.w.bits
    when (w_route(i)) { w_ready := s.w.ready }
  }

  val ar_noroute = !ar_route.orR
  val aw_noroute = !aw_route.orR
  val w_noroute  = !w_route.orR

  val err_slave = Module(new NastiErrorSlave)
  err_slave.io.ar.valid := ar_valid && ar_noroute
  err_slave.io.ar.bits := io.master.ar.bits
  err_slave.io.aw.valid := aw_valid && aw_noroute
  err_slave.io.aw.bits := io.master.aw.bits
  err_slave.io.w.valid := w_valid && w_noroute
  err_slave.io.w.bits := io.master.w.bits

  when (ar_noroute) { ar_ready := err_slave.io.ar.ready }
  when (aw_noroute) { aw_ready := err_slave.io.aw.ready }
  when (w_noroute)  { w_ready  := err_slave.io.w.ready }

  val b_arb = Module(new RRArbiter(new NastiWriteResponseChannel, nSlaves + 1))
  val r_arb = Module(new HellaPeekingArbiter(
    new NastiReadDataChannel, nSlaves + 1,
    // we can unlock if it's the last beat
    (r: NastiReadDataChannel) => r.last, rr = true))

  val all_slaves = io.slave :+ err_slave.io

  for (i <- 0 to nSlaves) {
    b_arb.io.in(i) <> all_slaves(i).b
    aw_queue.io.deq(i).valid := all_slaves(i).b.fire
    aw_queue.io.deq(i).tag := all_slaves(i).b.bits.id

    r_arb.io.in(i) <> all_slaves(i).r
    ar_queue.io.deq(i).valid := all_slaves(i).r.fire && all_slaves(i).r.bits.last
    ar_queue.io.deq(i).tag := all_slaves(i).r.bits.id

    assert(!aw_queue.io.deq(i).valid || aw_queue.io.deq(i).matches,
      s"aw_queue $i tried to dequeue untracked transaction")
    assert(!ar_queue.io.deq(i).valid || ar_queue.io.deq(i).matches,
      s"ar_queue $i tried to dequeue untracked transaction")
  }

  io.master.b <> b_arb.io.out
  io.master.r <> r_arb.io.out
}

/** Crossbar between multiple Nasti masters and slaves
 *  @param nMasters the number of Nasti masters
 *  @param nSlaves the number of Nasti slaves
 *  @param routeSel a function selecting the slave to route an address to */
class NastiCrossbar(nMasters: Int, nSlaves: Int,
                    routeSel: UInt => UInt)
                   (val cfg: NastiParameters) extends NastiModule {
  val io = new Bundle {
    val masters = Flipped(Vec(nMasters, new NastiIO))
    val slaves = Vec(nSlaves, new NastiIO)
  }

  if (nMasters == 1) {
    val router = Module(new NastiRouter(nSlaves, routeSel))
    router.io.master <> io.masters.head
    io.slaves <> router.io.slave
  } else {
    val routers = Vec.fill(nMasters) { Module(new NastiRouter(nSlaves, routeSel)).io }
    val arbiters = Vec.fill(nSlaves) { Module(new NastiArbiter(nMasters)).io }

    for (i <- 0 until nMasters) {
      routers(i).master <> io.masters(i)
    }

    for (i <- 0 until nSlaves) {
      arbiters(i).master <> Vec(routers.map(r => r.slave(i)))
      io.slaves(i) <> arbiters(i).slave
    }
  }
}

class NastiInterconnectIO(val nMasters: Int, val nSlaves: Int)
                         (val cfg: NastiParameters) extends Bundle {
  /* This is a bit confusing. The interconnect is a slave to the masters and
   * a master to the slaves. Hence why the declarations seem to be backwards. */
  val masters = Flipped(Vec(nMasters, new NastiIO))
  val slaves = Vec(nSlaves, new NastiIO)
}

abstract class NastiInterconnect(val cfg: NastiParameters) extends NastiModule(cfg) {
  val nMasters: Int
  val nSlaves: Int

  lazy val io = new NastiInterconnectIO(nMasters, nSlaves)
}

class NastiRecursiveInterconnect(
    val nMasters: Int, addrMap: AddrMap)
    (val cfg: NastiParameters) extends NastiInterconnect(cfg) {
  def port(name: String) = io.slaves(addrMap.port(name))
  val nSlaves = addrMap.numSlaves
  val routeSel = (addr: UInt) =>
    Cat(addrMap.entries.map(e => addrMap(e.name).containsAddress(addr)).reverse)

  val xbar = Module(new NastiCrossbar(nMasters, addrMap.length, routeSel))
  xbar.io.masters <> io.masters

  io.slaves <> addrMap.entries.zip(xbar.io.slaves).flatMap {
    case (entry, xbarSlave) => {
      entry.region match {
        case submap: AddrMap if submap.entries.isEmpty =>
          val err_slave = Module(new NastiErrorSlave)
          err_slave.io <> xbarSlave
          None
        case submap: AddrMap =>
          val ic = Module(new NastiRecursiveInterconnect(1, submap))
          ic.io.masters.head <> xbarSlave
          ic.io.slaves
        case r: MemRange =>
          Some(xbarSlave)
      }
    }
  }
}

object AXI4NastiAssigner {
  def toNasti(nasti: NastiIO, axi4: AXI4Bundle)(cfg: NastiParameters): Unit = {
    // HACK: Nasti and Diplomatic have diverged to the point where it's no
    // longer safe to emit a partial connect leaf fields. Onus is on the
    // invoker to check widths.
    nasti.aw.valid  := axi4.aw.valid
    nasti.aw.bits.id     := axi4.aw.bits.id
    nasti.aw.bits.addr   := axi4.aw.bits.addr
    nasti.aw.bits.len    := axi4.aw.bits.len
    nasti.aw.bits.size   := axi4.aw.bits.size
    nasti.aw.bits.burst  := axi4.aw.bits.burst
    nasti.aw.bits.lock   := axi4.aw.bits.lock
    nasti.aw.bits.cache  := axi4.aw.bits.cache
    nasti.aw.bits.prot   := axi4.aw.bits.prot
    nasti.aw.bits.qos    := axi4.aw.bits.qos
    nasti.aw.bits.user   := chisel3.DontCare
    nasti.aw.bits.region := 0.U
    axi4.aw.ready := nasti.aw.ready

    nasti.ar.valid  := axi4.ar.valid
    nasti.ar.bits.id     := axi4.ar.bits.id
    nasti.ar.bits.addr   := axi4.ar.bits.addr
    nasti.ar.bits.len    := axi4.ar.bits.len
    nasti.ar.bits.size   := axi4.ar.bits.size
    nasti.ar.bits.burst  := axi4.ar.bits.burst
    nasti.ar.bits.lock   := axi4.ar.bits.lock
    nasti.ar.bits.cache  := axi4.ar.bits.cache
    nasti.ar.bits.prot   := axi4.ar.bits.prot
    nasti.ar.bits.qos    := axi4.ar.bits.qos
    nasti.ar.bits.user   := chisel3.DontCare
    nasti.ar.bits.region := 0.U
    axi4.ar.ready := nasti.ar.ready

    nasti.w.valid  := axi4.w.valid
    nasti.w.bits.data  := axi4.w.bits.data
    nasti.w.bits.strb  := axi4.w.bits.strb
    nasti.w.bits.last  := axi4.w.bits.last
    nasti.w.bits.user  := chisel3.DontCare
    nasti.w.bits.id    := chisel3.DontCare // We only use AXI4, not AXI3
    axi4.w.ready := nasti.w.ready

    axi4.r.valid     := nasti.r.valid
    axi4.r.bits.id   := nasti.r.bits.id
    axi4.r.bits.data := nasti.r.bits.data
    axi4.r.bits.resp := nasti.r.bits.resp
    axi4.r.bits.last := nasti.r.bits.last
    axi4.r.bits.user := chisel3.DontCare
    // Echo is not a AXI4 standard signal.
    axi4.r.bits.echo := chisel3.DontCare
    nasti.r.ready := axi4.r.ready

    axi4.b.valid     := nasti.b.valid
    axi4.b.bits.id   := nasti.b.bits.id
    axi4.b.bits.resp := nasti.b.bits.resp
    axi4.b.bits.user := chisel3.DontCare
    // Echo is not a AXI4 standard signal.
    axi4.b.bits.echo := chisel3.DontCare
    nasti.b.ready := axi4.b.ready
  }

  def toAXI4Slave(axi4: AXI4Bundle, nasti: NastiIO): Unit = {
    // HACK: Nasti and Diplomatic have diverged to the point where it's no
    // longer safe to emit a partial connect leaf fields. Onus is on the
    // invoker to check widths.
    axi4.aw.valid       := nasti.aw.valid
    axi4.aw.bits.id     := nasti.aw.bits.id
    axi4.aw.bits.addr   := nasti.aw.bits.addr
    axi4.aw.bits.len    := nasti.aw.bits.len
    axi4.aw.bits.size   := nasti.aw.bits.size
    axi4.aw.bits.burst  := nasti.aw.bits.burst
    axi4.aw.bits.lock   := nasti.aw.bits.lock
    axi4.aw.bits.cache  := nasti.aw.bits.cache
    axi4.aw.bits.prot   := nasti.aw.bits.prot
    axi4.aw.bits.qos    := nasti.aw.bits.qos
    axi4.aw.bits.user   := chisel3.DontCare
    axi4.aw.bits.echo   := chisel3.DontCare
    nasti.aw.ready := axi4.aw.ready

    axi4.ar.valid       := nasti.ar.valid
    axi4.ar.bits.id     := nasti.ar.bits.id
    axi4.ar.bits.addr   := nasti.ar.bits.addr
    axi4.ar.bits.len    := nasti.ar.bits.len
    axi4.ar.bits.size   := nasti.ar.bits.size
    axi4.ar.bits.burst  := nasti.ar.bits.burst
    axi4.ar.bits.lock   := nasti.ar.bits.lock
    axi4.ar.bits.cache  := nasti.ar.bits.cache
    axi4.ar.bits.prot   := nasti.ar.bits.prot
    axi4.ar.bits.qos    := nasti.ar.bits.qos
    axi4.ar.bits.user   := chisel3.DontCare
    axi4.ar.bits.echo   := chisel3.DontCare
    nasti.ar.ready := axi4.ar.ready

    axi4.w.valid      := nasti.w.valid
    axi4.w.bits.data  := nasti.w.bits.data
    axi4.w.bits.strb  := nasti.w.bits.strb
    axi4.w.bits.last  := nasti.w.bits.last
    axi4.w.bits.user  := chisel3.DontCare
    nasti.w.ready := axi4.w.ready

    nasti.r.valid     := axi4.r.valid
    nasti.r.bits.id   := axi4.r.bits.id
    nasti.r.bits.data := axi4.r.bits.data
    nasti.r.bits.resp := axi4.r.bits.resp
    nasti.r.bits.last := axi4.r.bits.last
    nasti.r.bits.user := chisel3.DontCare
    axi4.r.ready := nasti.r.ready

    nasti.b.valid     := axi4.b.valid
    nasti.b.bits.id   := axi4.b.bits.id
    nasti.b.bits.resp := axi4.b.bits.resp
    nasti.b.bits.user := chisel3.DontCare
    // Echo is not a AXI4 standard signal.
    axi4.b.ready := nasti.b.ready
  }

  def toAXI4Master(axi4: AXI4Bundle, nasti: NastiIO): Unit = {
    // HACK: Nasti and Diplomatic have diverged to the point where it's no
    // longer safe to emit a partial connect leaf fields. Onus is on the
    // invoker to check widths.
    nasti.aw.valid      := axi4.aw.valid
    nasti.aw.bits.id    := axi4.aw.bits.id
    nasti.aw.bits.addr  := axi4.aw.bits.addr
    nasti.aw.bits.len   := axi4.aw.bits.len
    nasti.aw.bits.size  := axi4.aw.bits.size
    nasti.aw.bits.burst := axi4.aw.bits.burst
    nasti.aw.bits.lock  := axi4.aw.bits.lock
    nasti.aw.bits.cache := axi4.aw.bits.cache
    nasti.aw.bits.prot  := axi4.aw.bits.prot
    nasti.aw.bits.qos   := axi4.aw.bits.qos
    nasti.aw.bits.user  := chisel3.DontCare
    nasti.aw.bits.region := chisel3.DontCare
    //nasti.aw.bits.echo  := chisel3.DontCare
    axi4.aw.ready       := nasti.aw.ready

    nasti.ar.valid      := axi4.ar.valid
    nasti.ar.bits.id    := axi4.ar.bits.id
    nasti.ar.bits.addr  := axi4.ar.bits.addr
    nasti.ar.bits.len   := axi4.ar.bits.len
    nasti.ar.bits.size  := axi4.ar.bits.size
    nasti.ar.bits.burst := axi4.ar.bits.burst
    nasti.ar.bits.lock  := axi4.ar.bits.lock
    nasti.ar.bits.cache := axi4.ar.bits.cache
    nasti.ar.bits.prot  := axi4.ar.bits.prot
    nasti.ar.bits.qos   := axi4.ar.bits.qos
    nasti.ar.bits.user  := chisel3.DontCare
    nasti.ar.bits.region := chisel3.DontCare
    //nasti.ar.bits.echo  := chisel3.DontCare
    axi4.ar.ready       := nasti.ar.ready

    nasti.w.valid     := axi4.w.valid
    nasti.w.bits.data := axi4.w.bits.data
    nasti.w.bits.strb := axi4.w.bits.strb
    nasti.w.bits.last := axi4.w.bits.last
    nasti.w.bits.user := chisel3.DontCare
    nasti.w.bits.id   := chisel3.DontCare
    axi4.w.ready      := nasti.w.ready

    axi4.r.valid      := nasti.r.valid
    axi4.r.bits.id    := nasti.r.bits.id
    axi4.r.bits.data  := nasti.r.bits.data
    axi4.r.bits.resp  := nasti.r.bits.resp
    axi4.r.bits.last  := nasti.r.bits.last
    nasti.r.bits.user := chisel3.DontCare
    nasti.r.ready     := axi4.r.ready

    axi4.b.valid     := nasti.b.valid
    axi4.b.bits.id   := nasti.b.bits.id
    axi4.b.bits.resp := nasti.b.bits.resp
    nasti.b.bits.user := chisel3.DontCare
    // Echo is not a AXI4 standard signal.
    nasti.b.ready := axi4.b.ready
  }
}
