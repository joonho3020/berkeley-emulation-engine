# Berkeley Logic Interchange Format (BLIF) parser


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
