BUILDDIR="build-dir"
VERILATOR_LIB=libVdut.so

# Variables for emulator platform
top        ?= "Adder"
dir        := "../../examples/"
svfile     := $(dir)/$(top)".sv"
input_file := $(dir)/$(top)".input"
lut_file   := $(dir)/$(top)".lut.blif"
sim_dir    := "sim-dir-"$(top)

debug             ?= "true"
max_steps         ?= 128
num_mods          ?= 9
num_procs         ?= 8
imem_lat          ?= 1
inter_mod_nw_lat  ?= 0
inter_proc_nw_lat ?= 0
sram_width        ?= 16
sram_entries      ?= 16

# Chisel directories and files
EMULATOR_DIR  := ../../emulator
SCALA_SRC_DIR := $(EMULATOR_DIR)/src
SCALA_FILES   := $(shell find $(SCALA_SRC_DIR) -name '*.scala')

MILL_BUILD           := $(EMULATOR_DIR)/out/mill.lock
FPGATOP              := $(EMULATOR_DIR)/FPGATop.sv
MMAP                 := $(EMULATOR_DIR)/FPGATop.mmap
MILL_BUILD_ARTIFACTS := $(MILL_BUILD) $(FPGATOP) $(MMAP)

# Build target
.PHONY: chisel_elaborate
chisel_elaborate: $(MILL_BUILD_ARTIFACTS)

# Mill rebuild rule
$(MILL_BUILD_ARTIFACTS): $(SCALA_FILES)
	@echo "Changes detected in Scala files. Rebuilding with Mill..."
	cd $(EMULATOR_DIR) &&                        \
		mill emulator.run  --debug $(debug)        \
			--max-steps $(max_steps)                 \
			--num-mods $(num_mods)                   \
			--num-procs $(num_procs)                 \
			--imem-lat $(imem_lat)                   \
			--inter-mod-nw-lat $(inter_mod_nw_lat)   \
			--inter-proc-nw-lat $(inter_proc_nw_lat) \
			--sram-width $(sram_width)               \
			--sram-entries $(sram_entries)
	@touch $(MILL_BUILD) # Update mill lock file timestamp

# Build verilator shared library w/ c-bindings
$(VERILATOR_LIB): $(MILL_BUILD_ARTIFACTS)
	@if [ -d "$(BUILDDIR)" ]; then \
		echo "Directory $(BUILDDIR) exists. Deleting..."; \
		rm -rf $(BUILDDIR); \
	else \
		echo "Directory $(BUILDDIR) does not exist. Skipping deletion."; \
	fi
	cargo clean
	cargo update
	cargo build --release -vvv

# Run single example
run: $(VERILATOR_LIB)
	cargo run --release \
		--bin fpgatopsim \
		-- -s $(svfile) \
			 -t $(top) \
			 -i $(input_file) \
			 -b $(lut_file) \
			 --num-mods $(num_mods) \
			 --num-procs $(num_procs) \
			 --sram-width $(sram_width) \
			 --sram-entries $(sram_entries) \
			 --imem-lat $(imem_lat) \
			 --inter-proc-nw-lat $(inter_proc_nw_lat) \
			 --inter-mod-nw-lat $(inter_mod_nw_lat) \
			 --max-steps $(max_steps) | tee DEBUG

# Run existing tests
test: $(VERILATOR_LIB)
	cargo nextest run --release

clean:
	@if [ -d "$(BUILDDIR)" ]; then \
		echo "Directory $(BUILDDIR) exists. Deleting..."; \
		rm -rf $(BUILDDIR); \
	else \
		echo "Directory $(BUILDDIR) does not exist. Skipping deletion."; \
	fi
	cargo clean
	-rm libVdut.so
	-rm -f *.vcd
	-rm -f *.so
	-rm -rf blif-sim-dir-*
	-rm -rf build-dir
	-rm -rf sim-dir
	-rm DEBUG
	-rm $(MILL_BUILD_ARTIFACTS)
	-rm src/driver_generated.rs

all:
	test

.PHONY: run, clean, test, all
