import sys
from typing import Dict, Optional
import time

sys.path.append("/Users/joonhohwangbo/Desktop/UCB-BAR/opal-kelly/FrontPanel-Mac/API/Python")
import ok

class Tester:

  def __init__(self):
    self.addr_map = {
        # Signals going into the FPGA
        'reset':        0x00,
        'host_steps':   0x01,
        'used_procs':   0x02,
        'insns_valid':  0x03,
        'insns_bits_0': 0x04,
        'insns_bits_1': 0x05,
        'io_i_valid':   0x06,
        'io_i_bits_0':  0x07,
        'io_o_ready':   0x08,

        # Signals comming out from the FPGA
        'insns_ready': 0x20,
        'io_i_ready':  0x21,
        'io_o_valid':  0x22,
        'io_o_bits_0': 0x23
    }

    self.xem = ok.FrontPanelDevices().Open()
    if not self.xem:
      print ("A device could not be opened.  Is one connected?")
      exit(1)

    self.dev_info = ok.okTDeviceInfo()
    if (self.xem.NoError != self.xem.GetDeviceInfo(self.dev_info)):
        print ("Unable to retrieve device information.")
        exit(1)

    print("         Product: " + self.dev_info.productName)
    print("Firmware version: %d.%d" % (self.dev_info.deviceMajorVersion, self.dev_info.deviceMinorVersion))
    print("   Serial Number: %s" % self.dev_info.serialNumber)
    print("       Device ID: %s" % self.dev_info.deviceID)

    self.xem.LoadDefaultPLLConfiguration()

    if (self.xem.NoError != self.xem.ConfigureFPGA("FPGATop.bit")):
      print ("FPGA configuration failed.")
      exit(1)

    # Check for FrontPanel support in the FPGA configuration.
    if (False == self.xem.IsFrontPanelEnabled()):
      print ("FrontPanel support is not available.")
      exit(1)

  def read(self, addr: int) -> int:
    self.xem.UpdateWireOuts()
    return self.xem.GetWireOutValue(addr)

  def write(self, addr: int, val: int, mask: int = 0xffffffff) -> None:
    self.xem.SetWireInValue(addr, val, mask)
    self.xem.UpdateWireIns()

  def reset_on(self) -> None:
    self.write(self.addr_map['reset'], 0xff)

  def reset_off(self) -> None:
    self.write(self.addr_map['reset'], 0)

  def reset(self) -> None:
    self.reset_on()
    self.reset_off()

  def set_host_steps(self, v: int) -> None:
    self.write(self.addr_map['host_steps'], v)

  def set_used_procs(self, v: int) -> None:
    self.write(self.addr_map['used_procs'], v)

  def try_enq_instruction(self, bits1: int, bits0: int) -> bool:
    ready = self.read(self.addr_map['insns_ready'])
    if ready != 0:
      self.xem.SetWireInValue(self.addr_map['insns_bits_0'], bits0, 0xffffffff)
      self.xem.SetWireInValue(self.addr_map['insns_bits_1'], bits1, 0xffffffff)
      self.xem.UpdateWireIns()
      self.write(self.addr_map['insns_valid'], 1)
      self.write(self.addr_map['insns_valid'], 0)
      return True
    else:
      return False

  def enq_instruction(self, bits1: int, bits0: int):
    while True:
      if self.try_enq_instruction(bits1, bits0):
        break

  def try_enq_inputs(self, bits: int) -> bool:
    ready = self.read(self.addr_map['io_i_ready'])
    if ready != 0:
      self.write(self.addr_map['io_i_bits_0'], bits)
      self.write(self.addr_map['io_i_valid'], 1)
      self.write(self.addr_map['io_i_valid'], 0)
      return True
    else:
      return False

  def enq_inputs(self, bits: int):
    while True:
      if self.try_enq_inputs(bits):
        break

  def deq_outputs(self) -> bool:
    valid = self.read(self.addr_map['io_o_valid'])
    if valid != 0:
      bits = self.read(self.addr_map['io_o_bits_0'])
      self.write(self.addr_map['io_o_ready'], 1)
      self.write(self.addr_map['io_o_ready'], 0)
      print(f'dequeued {bits}')
      return True
    else:
      return False

def main():
  tester = Tester()
  tester.reset()
  tester.set_host_steps(6)
  tester.set_used_procs(6)

  tester.write(tester.addr_map['io_o_ready'], 0)

  print(f'insns_ready', tester.read(tester.addr_map['insns_ready']))
  print(f'io_i_ready',  tester.read(tester.addr_map['io_i_ready']))
  print(f'io_o_valid',  tester.read(tester.addr_map['io_o_valid']))
  print(f'io_o_bits_0',  tester.read(tester.addr_map['io_o_bits_0']))

  tester.enq_instruction(0x80, 0x01)
  tester.enq_instruction(0x100, 0x0)
  tester.enq_instruction(0x184, 0xb33)
  tester.enq_instruction(0x4, 0xc43)
  tester.enq_instruction(0x58, 0x14b3)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x1)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x1)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x1)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x1005)
  tester.enq_instruction(0x0, 0x4553)
  tester.enq_instruction(0x0, 0x4802)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x2005)
  tester.enq_instruction(0x0, 0x4553)
  tester.enq_instruction(0x0, 0x4802)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  tester.enq_instruction(0x0, 0x0)
  print("enq instructions done")

  tester.enq_inputs(0x0)
  print('bits:',hex(tester.read(tester.addr_map['io_o_bits_0'])),
        'valid:', hex(tester.read(tester.addr_map['io_o_valid'])))
  tester.write(tester.addr_map['io_o_ready'], 1)
  tester.write(tester.addr_map['io_o_ready'], 0)


  tester.enq_inputs(0x4)
  print('bits:',hex(tester.read(tester.addr_map['io_o_bits_0'])),
        'valid:', hex(tester.read(tester.addr_map['io_o_valid'])))
  tester.write(tester.addr_map['io_o_ready'], 1)
  tester.write(tester.addr_map['io_o_ready'], 0)


  tester.enq_inputs(0x9)
  print('bits:',hex(tester.read(tester.addr_map['io_o_bits_0'])),
        'valid:', hex(tester.read(tester.addr_map['io_o_valid'])))
  tester.write(tester.addr_map['io_o_ready'], 1)
  tester.write(tester.addr_map['io_o_ready'], 0)

  tester.enq_inputs(0xf)
  print('bits:',hex(tester.read(tester.addr_map['io_o_bits_0'])),
        'valid:', hex(tester.read(tester.addr_map['io_o_valid'])))
  tester.write(tester.addr_map['io_o_ready'], 1)
  tester.write(tester.addr_map['io_o_ready'], 0)

if __name__=="__main__":
  main()
