#!/bin/bash

BITSTREAM_PATH=/scratch/joonho.whangbo/coding/berkeley-emulation-engine/fpga/alveo-u250/large-version

/usr/local/bin/firesim-fpga-util.py \
  --bitstream $BITSTREAM_PATH/XilinxU250Board.bit \
  --bdf 17:00.0 \
  --fpga-db /opt/firesim-db.json

lsmod | grep -i xdma
sudo /usr/local/bin/firesim-chmod-xdma-perm
