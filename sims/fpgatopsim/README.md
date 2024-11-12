# RTL simulation of FPGATop

`FPGATop` exposes AXI4 and AXI4-lite ports which should be connected to the XDMA IP.
Internally, it converts the AXI4 transactions into multiple decoupled interfaces which has two use cases.
First of all, it is used to push the compiled instruction stream into the emulator.
Next, it is used to push/pull IO signals to/from the emulator.
The AXI4-lite ports are used for control registers such as `host_steps` and to setup SRAM processor configurations.

## Usage

- Requires a `conda env` with `Verilator` installed
- Since we require the verilated shared library, we can't run the tests in a single process. We can use the `cargo-nextest` program to solve this issue

```bash
cargo nextest run --release
```

---

# Rust based testbench library

This directory can be cleaned up to become a more generic Rust based testbench library.

## Flow

- Compile RTL into C++ using verilator
- Generate C APIs
- Compiler the C APIs and link it with the the verilated design -> `libVdut.so`
- Generate Rust bindings for the above C APIs
- Compile and run using `cargo`

## Future work: making this into a full blown rust based testbench API library

- Better APIs
    - It would be nice if we had `fork`, `join` constructs like `ChiselTest` in order to orchestrate port independently
    - It must be trivial to run parallel simulations based off of arguments. This is useful when we have multiple input-stimuli files that we want to run in parallel
    - Ability to inspect/inject internal signal values (not just the IO)
- It would be super cool if we can have a repl like testing environment
    - While we write the testbench, we can see the waveform getting updated in a browser real time
    - This would be useful for unit testing small designs
- High performance
    - We must benchmark the performance for various designs
    - Compare it against raw SV TB, Cocotb, ChiselTest
- Support multiple simulation backends
