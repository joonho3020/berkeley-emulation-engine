# Berkeley Logic Interchange Format (BLIF) parser

---

## Example usage

### Setup yosys (can be skipped if yosys is already installed)

```bash
conda config --set channel_priority true
conda config --add channels defaults

conda create -c litex-hub --prefix ~/.conda-yosys yosys=0.27_4_gb58664d44

conda config --set channel_priority strict
conda config --remove channels defaults

conda activate ~/.conda-yosys
```

### Generate blif file

- The `yosys.cmd` reads the verilog file, lowers it to primitive logic level representations, and uses ABC to map it to LUTs

```bash
cd examples
yosys
> script yosys.cmd
```

### Run RTL simulation to obtain a reference output

- Run:

```bash
cargo run --bin run_refrtlsim -- ../examples/GCD.sv GCD ../examples/GCD.input
```

### Run both emulation functional simulation and RTL simulation and compare the generated outputs

- Run:

```bash
cd compiler
cargo run --bin blif-parser -- ../examples/GCD.sv GCD  ../examples/GCD.input ../examples/GCD-2bit.lut.blif
```

### Running RTL simulations of the emulation processor

- To generate the RTL processor run:

```bash
cd emulator
mill emulator.run
mv OpalKellyEmulatorModuleTop.sv test/
```

- To generate the testharness run:

```bash
cd compiler
cargo run --bin run_emulrtlsim  ../examples/GCD.sv GCD ../examples/GCD.input ../examples/GCD-2bit.lut.blif
mv TestHarness.sv ../emulator/test
```

- To generate RTL simulations, run it, and see waveforms:

```
cd emulator/test
iverilog TestHarness.sv OpalKellyEmulatorModuleTop.sv
./a.out
gtkwave sim.vcd
```
