# Building a Xilinx U250 shell using Vivado batchmode 

## Create IP project

```bash
make ip_project
```

- Creates a directory containing IP `.xci` (xilinx custom interface) files
- This file specifies various parameters, and IO ports of the IP
- You can see the template in `ip/xdma_0/xdma_0.veo` for example
- [Xilinx IP configuration docs](https://docs.amd.com/r/en-US/ug896-vivado-ip/Using-the-Manage-IP-Flow)

## Building the design

```bash
make all
```

## Programming the FPGA

```bash
./flash-fpga.sh
```

This is assumes that firesim scripts has been installed.
This is because using the XDMA interface requires certain PCIe configurations which is done in the `firesim-fpga-util.py` script.
For now, I'm just going to reuse this script.

## Testing the bitstream w/ the driver

```bash
cd xdma-driver
cargo run
```

## Misc notes

- Seems like stuff works for MMIO quite robustly
- For DMA, there can perform transactions up to 128B at a time
- For both MMIO & DMA, there is some race condition, but I don't think this is really a problem for now
