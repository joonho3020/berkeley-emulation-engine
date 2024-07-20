# Berkeley Logic Interchange Format (BLIF) parser

---

# Requirements

The actual C++ code requires:

- Modern C++-20 ready compiler such as g++ version 10 or higher
- A C++17 port requiring g++ version 7.2.0 or higher is available in branch c++17
- CMake
- Intel Thread Building Blocks library (TBB)
- `libnuma-dev` on ubuntu

## Setup commands on Ubuntu

- Update GCC version:

```bash
sudo apt update
sudo apt install software-properties-common
sudo add-apt-repository ppa:ubuntu-toolchain-r/test
sudo apt install gcc-13 g++-13
sudo update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-13 100 --slave /usr/bin/g++ g++ /usr/bin/g++-13
gcc --version
```

- Install `libtbb` & `libnuma`:

```bash
sudo apt-get install libnuma-dev
sudo apt install libtbb-dev
```

## Setup commands using Conda

```bash
conda create -n <name> python=<version>
conda install conda-forge::gcc_linux-64"
conda install conda-forge::gcc -y"
conda install 'gxx[version=">=14"]'
conda install conda-forge::tbb-devel
conda install libnuma numactl
```

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

### Parse the blif file and obtain a pdf of the graph using graphviz

- By hand:

```bash
cargo run > examples/Adder.dot
dot examples/Adder.dot -Tpdf > examples/Adder.pdf
```

- Or alternatively run:

```bash
./run.py --blif examples/GCD.lut.blif --dot
```
