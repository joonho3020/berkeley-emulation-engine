package emulator

import chisel3._
import chisel3.util._
import scala.math.{min, max}
import freechips.rocketchip.util.{DecoupledHelper, ParameterizedBundle, HellaPeekingArbiter}
import freechips.rocketchip.amba.axi4._

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

// def bytesToXSize(bytes: UInt) = MuxLookup(bytes, 7.U(3.W), Array(
// UInt(1) -> 0.U,
// UInt(2) -> UInt(1),
// UInt(4) -> UInt(2),
// UInt(8) -> UInt(3),
// UInt(16) -> UInt(4),
// UInt(32) -> UInt(5),
// UInt(64) -> UInt(6),
// UInt(128) -> UInt(7)))
}

object NastiParameters {
  def apply(params: AXI4BundleParameters): NastiParameters =
    NastiParameters(params.dataBits, params.addrBits, params.idBits)
}

abstract class NastiChannel             (cfg: NastiParameters) extends Bundle
abstract class NastiMasterToSlaveChannel(cfg: NastiParameters) extends NastiChannel(cfg)
abstract class NastiSlaveToMasterChannel(cfg: NastiParameters) extends NastiChannel(cfg)

class NastiReadIO(cfg: NastiParameters) extends Bundle {
  val ar =         Decoupled(new NastiReadAddressChannel(cfg))
  val r  = Flipped(Decoupled(new NastiReadDataChannel   (cfg)))
}

class NastiWriteIO(cfg: NastiParameters) extends Bundle {
  val aw =         Decoupled(new NastiWriteAddressChannel (cfg))
  val w  =         Decoupled(new NastiWriteDataChannel    (cfg))
  val b  = Flipped(Decoupled(new NastiWriteResponseChannel(cfg)))
}

class NastiIO(cfg: NastiParameters) extends Bundle {
  val aw =         Decoupled(new NastiWriteAddressChannel (cfg))
  val w  =         Decoupled(new NastiWriteDataChannel    (cfg))
  val b  = Flipped(Decoupled(new NastiWriteResponseChannel(cfg)))
  val ar =         Decoupled(new NastiReadAddressChannel  (cfg))
  val r  = Flipped(Decoupled(new NastiReadDataChannel     (cfg)))
}

class NastiAddressChannel(cfg: NastiParameters) extends NastiMasterToSlaveChannel(cfg) {
  import cfg._
  val addr   = UInt(width = nastiXAddrBits.W)
  val len    = UInt(width = nastiXLenBits.W)
  val size   = UInt(width = nastiXSizeBits.W)
  val burst  = UInt(width = nastiXBurstBits.W)
  val lock   = Bool()
  val cache  = UInt(width = nastiXCacheBits.W)
  val prot   = UInt(width = nastiXProtBits.W)
  val qos    = UInt(width = nastiXQosBits.W)
  val region = UInt(width = nastiXRegionBits.W)
}

class NastiResponseChannel(cfg: NastiParameters) extends NastiSlaveToMasterChannel(cfg) {
  val resp = UInt(width = cfg.nastiXRespBits.W)
}

class NastiWriteAddressChannel(cfg: NastiParameters) extends NastiAddressChannel(cfg) {
  val id   = UInt(width = cfg.nastiWIdBits.W)
  val user = UInt(width = cfg.nastiAWUserBits.W)
}

class NastiWriteDataChannel(cfg: NastiParameters) extends NastiMasterToSlaveChannel(cfg) {
  val data = UInt(width = cfg.nastiXDataBits.W)
  val last = Bool()
  val id   = UInt(width = cfg.nastiWIdBits.W)
  val strb = UInt(width = cfg.nastiWStrobeBits.W)
  val user = UInt(width = cfg.nastiWUserBits.W)
}

class NastiWriteResponseChannel(cfg: NastiParameters) extends NastiResponseChannel(cfg) {
  val id   = UInt(width = cfg.nastiWIdBits.W)
  val user = UInt(width = cfg.nastiBUserBits.W)
}

class NastiReadAddressChannel(cfg: NastiParameters) extends NastiAddressChannel(cfg) {
  val id   = UInt(width = cfg.nastiRIdBits.W)
  val user = UInt(width = cfg.nastiARUserBits.W)
}

class NastiReadDataChannel(cfg: NastiParameters) extends NastiResponseChannel(cfg) {
  val data = UInt(width = cfg.nastiXDataBits.W)
  val last = Bool()
  val id   = UInt(width = cfg.nastiRIdBits.W)
  val user = UInt(width = cfg.nastiRUserBits.W)
}

object NastiConstants {
  val BURST_FIXED = 0.U(2.W)
  val BURST_INCR  = 1.U(2.W)
  val BURST_WRAP  = 2.U(2.W)

  val RESP_OKAY   = 0.U(2.W)
  val RESP_EXOKAY = 1.U(2.W)
  val RESP_SLVERR = 2.U(2.W)
  val RESP_DECERR = 3.U(2.W)

  val CACHE_DEVICE_NOBUF         = 0.U(4.W)
  val CACHE_DEVICE_BUF           = 1.U(4.W)
  val CACHE_NORMAL_NOCACHE_NOBUF = 2.U(4.W)
  val CACHE_NORMAL_NOCACHE_BUF   = 3.U(4.W)

  def AXPROT(instruction: Bool, nonsecure: Bool, privileged: Bool): UInt =
    Cat(instruction, nonsecure, privileged)

  def AXPROT(instruction: Boolean, nonsecure: Boolean, privileged: Boolean): UInt =
    AXPROT(instruction.B, nonsecure.B, privileged.B)
}

import NastiConstants._

object NastiWriteAddressChannel {
  def apply(id: UInt, addr: UInt, size: UInt,
      len: UInt = 0.U, burst: UInt = BURST_INCR)
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
    aw.qos := 0.U(4.W)
    aw.region := 0.U(4.W)
    aw.user := 0.U
    aw
  }
}

object NastiReadAddressChannel {
  def apply(id: UInt, addr: UInt, size: UInt,
      len: UInt = 0.U, burst: UInt = BURST_INCR)
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
    ar.qos := 0.U
    ar.region := 0.U
    ar.user := 0.U
    ar
  }
}

object NastiWriteDataChannel {
  def apply(data: UInt, strb: Option[UInt] = None,
            last: Bool = true.B, id: UInt = 0.U)
           (cfg: NastiParameters): NastiWriteDataChannel = {
    val w = Wire(new NastiWriteDataChannel(cfg))
    w.strb := strb.getOrElse(Fill(cfg.nastiWStrobeBits, 1.U(1.W)))
    w.data := data
    w.last := last
    w.id   := id
    w.user := 0.U
    w
  }
}

object NastiReadDataChannel {
  def apply(id: UInt, data: UInt, last: Bool = true.B, resp: UInt = 0.U)(
      cfg: NastiParameters) = {
    val r = Wire(new NastiReadDataChannel(cfg))
    r.id := id
    r.data := data
    r.last := last
    r.resp := resp
    r.user := 0.U
    r
  }
}

object NastiWriteResponseChannel {
  def apply(id: UInt, resp: UInt = 0.U)(cfg: NastiParameters) = {
    val b = Wire(new NastiWriteResponseChannel(cfg))
    b.id := id
    b.resp := resp
    b.user := 0.U
    b
  }
}

class NastiQueue(depth: Int)(val cfg: NastiParameters) extends Module {
  val io = new Bundle {
    val in  = Flipped(new NastiIO(cfg))
    val out =         new NastiIO(cfg)
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
  val master = Flipped(Vec(arbN, new NastiIO(cfg)))
  val slave  = new NastiIO(cfg)
}

/** Arbitrate among arbN masters requesting to a single slave */
class NastiArbiter(val arbN: Int)(cfg: NastiParameters) extends Module {
  val io = new NastiArbiterIO(arbN)(cfg)

  if (arbN > 1) {
    val arbIdBits = log2Up(arbN)

    val ar_arb = Module(new RRArbiter(new NastiReadAddressChannel(cfg), arbN))
    val aw_arb = Module(new RRArbiter(new NastiWriteAddressChannel(cfg), arbN))

    val w_chosen = Reg(UInt(width = arbIdBits.W))
    val w_done = RegInit(true.B)

    when (aw_arb.io.out.fire) {
      w_chosen := aw_arb.io.chosen
      w_done := false.B
    }

    when (io.slave.w.fire && io.slave.w.bits.last) {
      w_done := true.B
    }

    val queueSize = min((1 << cfg.nastiXIdBits) * arbN, 64)

    val rroq = Module(new ReorderQueue(
      UInt(width = arbIdBits.W), cfg.nastiXIdBits, Some(queueSize)))

    val wroq = Module(new ReorderQueue(
      UInt(width = arbIdBits.W), cfg.nastiXIdBits, Some(queueSize)))

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

      m_r.valid := io.slave.r.valid && rroq.io.deq.head.data === i.U
      m_r.bits := io.slave.r.bits

      m_b.valid := io.slave.b.valid && wroq.io.deq.head.data === i.U
      m_b.bits := io.slave.b.bits

      m_w.ready := io.slave.w.ready && w_chosen === i.U && !w_done
    }

    io.slave.r.ready := io.master(rroq.io.deq.head.data).r.ready
    io.slave.b.ready := io.master(wroq.io.deq.head.data).b.ready

    rroq.io.deq.head.tag   := io.slave.r.bits.id
    rroq.io.deq.head.valid := io.slave.r.fire && io.slave.r.bits.last
    wroq.io.deq.head.tag   := io.slave.b.bits.id
    wroq.io.deq.head.valid := io.slave.b.fire

    assert(!rroq.io.deq.head.valid || rroq.io.deq.head.matches, "NastiArbiter: read  response mismatch")
    assert(!wroq.io.deq.head.valid || wroq.io.deq.head.matches, "NastiArbiter: write response mismatch")

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
class NastiErrorSlave(cfg: NastiParameters) extends Module {
  val io = IO(Flipped(new NastiIO(cfg)))

  when (io.ar.fire) { printf("Invalid read address %x\n", io.ar.bits.addr) }
  when (io.aw.fire) { printf("Invalid write address %x\n", io.aw.bits.addr) }

  val r_queue = Module(new Queue(new NastiReadAddressChannel(cfg), 1))
  r_queue.io.enq <> io.ar

  val responding = RegInit(false.B)
  val beats_left = RegInit(0.U(cfg.nastiXLenBits.W))

  when (!responding && r_queue.io.deq.valid) {
    responding := true.B
    beats_left := r_queue.io.deq.bits.len
  }

  io.r.valid := r_queue.io.deq.valid && responding
  io.r.bits.id := r_queue.io.deq.bits.id
  io.r.bits.data := 0.U
  io.r.bits.resp := RESP_DECERR
  io.r.bits.last := beats_left === 0.U
  io.r.bits.user := 0.U

  r_queue.io.deq.ready := io.r.fire && io.r.bits.last

  when (io.r.fire) {
    when (beats_left === 0.U) {
      responding := false.B
    } .otherwise {
      beats_left := beats_left - 1.U
    }
  }

  val draining = RegInit(false.B)
  io.w.ready := draining

  when (io.aw.fire) { draining := true.B }
  when (io.w.fire && io.w.bits.last) { draining := false.B }

  val b_queue = Module(new Queue(UInt(cfg.nastiWIdBits.W), 1))
  b_queue.io.enq.valid := io.aw.valid && !draining
  b_queue.io.enq.bits := io.aw.bits.id
  io.aw.ready := b_queue.io.enq.ready && !draining
  io.b.valid := b_queue.io.deq.valid && !draining
  io.b.bits.id := b_queue.io.deq.bits
  io.b.bits.resp := RESP_DECERR
  io.b.bits.user := 0.U
  b_queue.io.deq.ready := io.b.ready && !draining
}

class NastiRouterIO(nSlaves: Int)(val cfg: NastiParameters) extends Bundle {
  val master = Flipped(new NastiIO(cfg))
  val slave = Vec(nSlaves, new NastiIO(cfg))
}

/** Take a single Nasti master and route its requests to various slaves
 *  @param nSlaves the number of slaves
 *  @param routeSel a function which takes an address and produces
 *  a one-hot encoded selection of the slave to write to */
class NastiRouter(nSlaves: Int, routeSel: UInt => UInt)(cfg: NastiParameters)
    extends Module {

  val io = IO(new NastiRouterIO(nSlaves)(cfg))

  val ar_route = routeSel(io.master.ar.bits.addr)
  val aw_route = routeSel(io.master.aw.bits.addr)

  val ar_ready = WireInit(false.B)
  val aw_ready = WireInit(false.B)
  val w_ready  = WireInit(false.B)

  val queueSize = min((1 << cfg.nastiXIdBits) * nSlaves, 64)

  // These reorder queues remember which slave ports requests were sent on
  // so that the responses can be sent back in-order on the master
  val ar_queue = Module(new ReorderQueue(
    UInt(log2Up(nSlaves + 1).W), cfg.nastiXIdBits,
    Some(queueSize), nSlaves + 1))
  val aw_queue = Module(new ReorderQueue(
    UInt(log2Up(nSlaves + 1).W), cfg.nastiXIdBits,
    Some(queueSize), nSlaves + 1))
  // This queue holds the accepted aw_routes so that we know how to route the
  val w_queue = Module(new Queue(aw_route.cloneType, nSlaves))

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

  def routeEncode(oh: UInt): UInt = Mux(oh.orR, OHToUInt(oh), nSlaves.U)

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

  val err_slave = Module(new NastiErrorSlave(cfg))
  err_slave.io.ar.valid := ar_valid && ar_noroute
  err_slave.io.ar.bits := io.master.ar.bits
  err_slave.io.aw.valid := aw_valid && aw_noroute
  err_slave.io.aw.bits := io.master.aw.bits
  err_slave.io.w.valid := w_valid && w_noroute
  err_slave.io.w.bits := io.master.w.bits

  when (ar_noroute) { ar_ready := err_slave.io.ar.ready }
  when (aw_noroute) { aw_ready := err_slave.io.aw.ready }
  when (w_noroute)  { w_ready  := err_slave.io.w.ready }

  val b_arb = Module(new RRArbiter(new NastiWriteResponseChannel(cfg), nSlaves + 1))
  val r_arb = Module(new HellaPeekingArbiter(
    new NastiReadDataChannel(cfg), nSlaves + 1,
    // we can unlock if it's the last beat
    (r: NastiReadDataChannel) => r.last, rr = true))

  val all_slaves = io.slave :+ err_slave.io

  println(s"nSlaves ${nSlaves} allSlaves: ${all_slaves}")

  for (i <- 0 to nSlaves) {
    println(s"i ${i} b_arb.io.in ${b_arb.io.in}")
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
                   (cfg: NastiParameters) extends Module {
  val io = IO(new Bundle {
    val masters = Flipped(Vec(nMasters, new NastiIO(cfg)))
    val slaves = Vec(nSlaves, new NastiIO(cfg))
  })

  if (nMasters == 1) {
    val router = Module(new NastiRouter(nSlaves, routeSel)(cfg))
    router.io.master <> io.masters.head
    io.slaves <> router.io.slave
  } else {
    val routers = Seq.fill(nMasters) { Module(new NastiRouter(nSlaves, routeSel)(cfg)).io }
    val arbiters = Seq.fill(nSlaves) { Module(new NastiArbiter(nMasters)(cfg)).io }

    for (i <- 0 until nMasters) {
      routers(i).master <> io.masters(i)
    }

    for (i <- 0 until nSlaves) {
      for (j <- 0 until nMasters) {
        arbiters(i).master(j) <> routers(j).slave(i)
      }
      io.slaves(i) <> arbiters(i).slave
    }
  }
}

class NastiInterconnectIO(val nMasters: Int, val nSlaves: Int)
                         (val cfg: NastiParameters) extends Bundle {
  /* This is a bit confusing. The interconnect is a slave to the masters and
   * a master to the slaves. Hence why the declarations seem to be backwards. */
  val masters = Flipped(Vec(nMasters, new NastiIO(cfg)))
  val slaves = Vec(nSlaves, new NastiIO(cfg))
}

abstract class NastiInterconnect(cfg: NastiParameters) extends Module {
  val nMasters: Int
  val nSlaves: Int

  lazy val io = new NastiInterconnectIO(nMasters, nSlaves)(cfg)
}

object AXI4NastiAssigner {
  def toNasti(nasti: NastiIO, axi4: AXI4Bundle): Unit = {
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
