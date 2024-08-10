import sys
from typing import Dict, Optional
import time

sys.path.append("/Users/joonhohwangbo/Desktop/UCB-BAR/opal-kelly/FrontPanel-Mac/API/Python")
import ok

class Tester:

  def __init__(self):
    self.addr_map = {
        'enq_val':   0x00,
        'enq_data':  0x01,
        'deq_rdy':   0x02,
        'enq_rdy':   0x20,
        'deq_val':   0x21,
        'deq_data':  0x22,
        'rst':       0x80,
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

    if (self.xem.NoError != self.xem.ConfigureFPGA("InterfaceTest.bit")):
      print ("FPGA configuration failed.")
      exit(1)

    # Check for FrontPanel support in the FPGA configuration.
    if (False == self.xem.IsFrontPanelEnabled()):
      print ("FrontPanel support is not available.")
      exit(1)

  def read(self, addr: int) -> int:
    self.xem.UpdateWireOuts()
    return self.xem.GetWireOutValue(addr)

  def write(self, addr: int, val: int, mask: int = 0xff) -> None:
    self.xem.SetWireInValue(addr, val, mask)
    self.xem.UpdateWireIns()

  def reset(self) -> None:
    print('reset')
    self.write(self.addr_map['rst'], 0xff)
    self.write(self.addr_map['rst'], 0)

  def enq(self, val: int) -> Optional[int]:
    print(f'enq {val}')
    rdy = self.read(self.addr_map['enq_rdy'])
    if rdy != 0:
      self.write(self.addr_map['enq_data'], val)
      self.write(self.addr_map['enq_val'], 0)
      self.write(self.addr_map['enq_val'], 1)
      return val
    else:
      return None

  def deq(self) -> Optional[int]:
    val = self.read(self.addr_map['deq_val'])
    if val != 0:
      ret = self.read(self.addr_map['deq_data'])
      self.write(self.addr_map['deq_rdy'], 1)
      return ret
    else:
      return None

def main():
  tester = Tester()
  tester.reset()
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))
  print('-----------------')

  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 2)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()


  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')

  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')

  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 3)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()

  tester.write(tester.addr_map['enq_val'], 0)
  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 4)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()

  tester.write(tester.addr_map['enq_val'], 0)
  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 5)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()

  tester.write(tester.addr_map['enq_val'], 0)
  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 6)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()

  tester.write(tester.addr_map['enq_val'], 0)
  tester.xem.SetWireInValue(tester.addr_map['enq_data'], 7)
  tester.xem.SetWireInValue(tester.addr_map['enq_val'], 1)
  tester.xem.UpdateWireIns()
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')
  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')
  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')
  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')
  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))

  print('-----------------')
  tester.write(tester.addr_map['enq_val'], 0)
  tester.write(tester.addr_map['deq_rdy'], 0)
  tester.write(tester.addr_map['deq_rdy'], 1)
  print(hex(tester.read(tester.addr_map['enq_rdy'])))
  print(hex(tester.read(tester.addr_map['deq_val'])))
  print(hex(tester.read(tester.addr_map['deq_data'])))





  # tester.write(tester.addr_map['enq_data'], 3)
  # tester.write(tester.addr_map['enq_val'], 0)
  # tester.write(tester.addr_map['enq_val'], 1)


# val = tester.deq()
# if val != None:
# print(f'Expected none for initial deq, got {val}')

# print(tester.read(tester.addr_map['enq_rdy']))
# print(tester.write(tester.addr_map['enq_data'], 1))
# print(tester.write(tester.addr_map['enq_val'], 1))

# rc = tester.enq(1)
# if rc == None:
# print(f'Failed to enq value')
# print(hex(tester.read(tester.addr_map['enq_rdy'])))

# print(hex(tester.read(tester.addr_map['deq_val'])))

# val = tester.deq()
# if val != 1:
# print(f'Expected 1 for deq, got {val}')

  # val = tester.deq()
  # if val != None:
  #   print(f'Expected None for deq, got {val}')

  # enq_vals = [2, 3, 4, 5, 6]
  # for v in enq_vals:
  #   rc = tester.enq(v)
  #   if rc == None:
  #     print(f'Failed to enq value {v}')

  # for _ in range(len(enq_vals)):
  #   v = tester.deq()
  #   print(f'Dequeued {v}')


if __name__=="__main__":
  main()
