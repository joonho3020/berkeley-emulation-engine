#!/bin/bash

conda-lock install -n bee
conda activate bee
conda env config vars set PKG_CONFIG_PATH=$CONDA_PREFIX/lib/pkgconfig:$PKG_CONFIG_PATH
