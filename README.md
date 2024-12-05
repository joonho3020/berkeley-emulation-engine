# Berkeley Emulation Engine

---

## 1. Setup

### Generating a conda lock file from the current environment

```bash
cd scripts
conda-lock lock -p linux-64 -f env.yaml
```

- This will generate a `conda-lock.yml` file

### Install conda env

```bash
cd scripts

// Install conda env
conda-lock install -n <name of the environment>

// Change the PKG_CONFIG_PATH to point to the conda env
conda env config vars set PKG_CONFIG_PATH=$CONDA_PREFIX/lib/pkgconfig:$PKG_CONFIG_PATH
```

### Setup yosys (can be skipped if yosys is already installed)

```bash
conda config --set channel_priority true
conda config --add channels defaults

conda create -c litex-hub --prefix ~/.conda-yosys yosys=0.27_4_gb58664d44

conda config --set channel_priority strict
conda config --remove channels defaults

conda activate ~/.conda-yosys
```


## 2. Generating inputs

### Generate blif file

- The `yosys.cmd` reads the verilog file, lowers it to primitive logic level representations, and uses ABC to map it to LUTs

```bash
cd examples
yosys
> script yosys.cmd
```

## 3. Running the compiler

The compiler has a functional simulator that you can use to run tests.

### Run both emulation functional simulation and RTL simulation and compare the generated outputs

```bash
cd compiler
just \
    top=OneReadOneWritePortSRAM \
    dir=../examples num_mods=17 \
    num_procs=64 sram_entries=16384 \
    sram_width=256 \
    inter_mod_nw_lat=1 inter_proc_nw_lat=1 bee
```

### Run both emulation functional simulation and compare it with a VCD file

```bash
cd compiler
 just \
     top=DigitalTop \
     instance_path=TestDriver.testHarness.chiptop0.system \
     check_cycle_period=100 \
     sram_entries=16384 \
     imem_lat=1 \
     num_mods=17 \
     num_procs=64 \
     inter_mod_nw_lat=1 \
     dmem_rd_lat=1 \
     inter_proc_nw_lat=1 \
     bee_vcd
```

### Run both emulation functional simulation and compare it with a blif native format simulator

This is useful to check if the functional simulator has any bugs

```
cd compiler
just top=DigitalTop sram_entries=16384 sim_dir=blif-sim-dir-DigitalTop run_blifsim
```

### We can extract the IO traces from a VCD file to use as input stimuli to the functional simulator

```
cd compiler
just top=DigitalTop instance_path=TestDriver.testHarness.chiptop0.system run_test_gen_from_vcd
```

### Run existing tests from the example directory

```
cd compiler
just test
```

## 4. Testing the RTL

This essentially corresponds to metasims in FireSim: we expose AXI ports that connect to the XDMA module and simulate everything downstream.
Running the below commands will generate the RTL, verilate it and create rust bindings so that the driver can perform AXI transactions.

```
cd sim/metasims
make test
```

## 5. Building the FPGA overlay

```
cd fpga/alveo-u250/design/
make ip_project && make all
```

## 6. Running simulations

```
cd sim/alveo-u250/
make run
```
