import cocotb
from cocotb.triggers import FallingEdge, Timer

async def generate_clock(dut):
  """Generate clock pulses."""
  for _ in range(200):
    dut.clock.value = 0
    await Timer(1, units="ns")
    dut.clock.value = 1
    await Timer(1, units="ns")


@cocotb.test()
async def test_bench(dut):
  await cocotb.start(generate_clock(dut))  # run the clock "in the background"

  await Timer(5, units="ns")  # wait a bit
  await FallingEdge(dut.clock)  # wait for falling edge/"negedge"

  dut._log.info("my_signal_1 is %s", dut.io_host_steps.value)
