#!/bin/bash

/usr/local/bin/firesim-fpga-util.py \
  --bitstream /scratch/joonho.whangbo/coding/xdma-test/design/project/impl/latest/XilinxU250Board.bit \
  --bdf 61:00.0 \
  --fpga-db /opt/firesim-db.json

lsmod | grep -i xdma
sudo /usr/local/bin/firesim-chmod-xdma-perm
