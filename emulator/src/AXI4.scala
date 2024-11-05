package emulator

import chisel3._
import chisel3.util._
import chisel3.experimental.hierarchy.{Definition, Instance}

object AXI4Parameters
{
  // These are all fixed by the AXI4 standard:
  val lenBits   = 8
  val sizeBits  = 3
  val burstBits = 2
  val lockBits  = 1
  val cacheBits = 4
  val protBits  = 3
  val qosBits   = 4
  val respBits  = 2

  def CACHE_RALLOCATE  = 8.U(cacheBits.W)
  def CACHE_WALLOCATE  = 4.U(cacheBits.W)
  def CACHE_MODIFIABLE = 2.U(cacheBits.W)
  def CACHE_BUFFERABLE = 1.U(cacheBits.W)

  def PROT_PRIVILEGED = 1.U(protBits.W)
  def PROT_INSECURE    = 2.U(protBits.W)
  def PROT_INSTRUCTION = 4.U(protBits.W)

  def BURST_FIXED = 0.U(burstBits.W)
  def BURST_INCR  = 1.U(burstBits.W)
  def BURST_WRAP  = 2.U(burstBits.W)

  def RESP_OKAY   = 0.U(respBits.W)
  def RESP_EXOKAY = 1.U(respBits.W)
  def RESP_SLVERR = 2.U(respBits.W)
  def RESP_DECERR = 3.U(respBits.W)
}

case class AXI4BundleParameters(addrBits: Int, dataBits: Int, idBits:   Int)
{
  require (dataBits >= 8, s"AXI4 data bits must be >= 8 (got $dataBits)")
  require (addrBits >= 1, s"AXI4 addr bits must be >= 1 (got $addrBits)")
  require (idBits >= 1, s"AXI4 id bits must be >= 1 (got $idBits)")
  require (isPow2(dataBits), s"AXI4 data bits must be pow2 (got $dataBits)")

  // Bring the globals into scope
  val lenBits   = AXI4Parameters.lenBits
  val sizeBits  = AXI4Parameters.sizeBits
  val burstBits = AXI4Parameters.burstBits
  val lockBits  = AXI4Parameters.lockBits
  val cacheBits = AXI4Parameters.cacheBits
  val protBits  = AXI4Parameters.protBits
  val qosBits   = AXI4Parameters.qosBits
  val respBits  = AXI4Parameters.respBits
  val userBits  = 0
  val echoBits  = 0
}

abstract class AXI4BundleBase(val params: AXI4BundleParameters) extends Bundle

/**
  * Common signals of AW and AR channels of AXI4 protocol
  */
abstract class AXI4BundleA(params: AXI4BundleParameters) extends AXI4BundleBase(params)
{
  val id     = UInt(params.idBits.W)
  val addr   = UInt(params.addrBits.W)
  val len    = UInt(params.lenBits.W)  // number of beats - 1
  val size   = UInt(params.sizeBits.W) // bytes in beat = 2^size
  val burst  = UInt(params.burstBits.W)
  val lock   = UInt(params.lockBits.W)
  val cache  = UInt(params.cacheBits.W)
  val prot   = UInt(params.protBits.W)
  val qos    = UInt(params.qosBits.W)  // 0=no QoS, bigger = higher priority
  val user   = UInt(params.userBits.W)
  val echo   = UInt(params.echoBits.W)
  // val region = UInt(4.W) // optional

  // Number of bytes-1 in this operation
  def bytes1(x:Int=0) = {
    val maxShift = 1 << params.sizeBits
    val tail = ((BigInt(1) << maxShift) - 1).U
    (Cat(len, tail) << size) >> maxShift
  }
}

/**
  * A non-standard bundle that can be both AR and AW
  */
class AXI4BundleARW(params: AXI4BundleParameters) extends AXI4BundleA(params)
{
  val wen = Bool()
}

/**
  * AW channel of AXI4 protocol
  */
class AXI4BundleAW(params: AXI4BundleParameters) extends AXI4BundleA(params)

/**
  * AR channel of AXI4 protocol
  */
class AXI4BundleAR(params: AXI4BundleParameters) extends AXI4BundleA(params)

/**
  * W channel of AXI4 protocol
  */
class AXI4BundleW(params: AXI4BundleParameters) extends AXI4BundleBase(params)
{
  // id ... removed in AXI4
  val data = UInt(params.dataBits.W)
  val strb = UInt((params.dataBits/8).W)
  val last = Bool()
  val user = UInt(params.userBits.W)
}

/**
  * R channel of AXI4 protocol
  */
class AXI4BundleR(params: AXI4BundleParameters) extends AXI4BundleBase(params)
{
  val id   = UInt(params.idBits.W)
  val data = UInt(params.dataBits.W)
  val resp = UInt(params.respBits.W)
  val user = UInt(params.userBits.W) // control and data
  val echo = UInt(params.echoBits.W)
  val last = Bool()
}

/**
  * B channel of AXI4 protocol
  */
class AXI4BundleB(params: AXI4BundleParameters) extends AXI4BundleBase(params)
{
  val id   = UInt(params.idBits.W)
  val resp = UInt(params.respBits.W)
  val user = UInt(params.userBits.W) // control and data
  val echo = UInt(params.echoBits.W)
}

/**
  * AXI4 protocol bundle
  */
class AXI4Bundle(params: AXI4BundleParameters) extends AXI4BundleBase(params)
{
  val aw = Irrevocable(new AXI4BundleAW(params))
  val w  = Irrevocable(new AXI4BundleW (params))
  val b  = Flipped(Irrevocable(new AXI4BundleB (params)))
  val ar = Irrevocable(new AXI4BundleAR(params))
  val r  = Flipped(Irrevocable(new AXI4BundleR (params)))
}

object AXI4Bundle
{
  def apply(params: AXI4BundleParameters) = new AXI4Bundle(params)
}

