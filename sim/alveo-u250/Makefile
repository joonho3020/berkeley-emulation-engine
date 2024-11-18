BUILDDIR=build-dir

# Variables for emulator platform
top        ?= "Adder"
dir        := "../../examples/"
svfile     := $(dir)/$(top)".sv"
input_file := $(dir)/$(top)".input"
lut_file   := $(dir)/$(top)".lut.blif"
sim_dir    := "../sim-dir/alveo-u250-"$(top)

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
MMAP                 := $(BUILDDIR)/FPGATop.mmap
FPGATOP              := $(BUILDDIR)/FPGATop.sv
MILL_BUILD_ARTIFACTS := $(MILL_BUILD) $(MMAP) $(FPGATop)

# Chisel generated driver stuff
SIMIF_DIR := src/simif
SIMIF_SRC_DIR := ../simif
SIMIF_FILES := $(shell find $(SIMIF_SRC_DIR) -type f -name '*.rs' -printf "%f\n")
SIMIF_SRCS  := $(shell find $(SIMIF_SRC_DIR) -name '*.rs')
DRIVER_GENERATED := driver_generated.rs
DRIVER_SRCS := $(SIMIF_DIR)/$(SIMIF_FILES) $(SIMIF_DIR)/$(DRIVER_GENERATED)
SIMULATION_SRCS := $(DRIVER_SRCS)

.DEFAULT_GOAL := build

# Build target
chisel_elaborate: $(MILL_BUILD_ARTIFACTS)

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

# Mill rebuild rule
$(MILL_BUILD_ARTIFACTS): $(SCALA_FILES) | $(BUILDDIR)
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
	cp $(EMULATOR_DIR)/FPGATop.mmap $(BUILDDIR)/
	cp $(EMULATOR_DIR)/FPGATop.sv   $(BUILDDIR)/

# Build verilator shared library w/ c-bindings
$(SIMULATION_SRCS): $(MILL_BUILD_ARTIFACTS) $(SIMIF_SRCS)
	cp $(SIMIF_SRCS) $(SIMIF_DIR)
	cargo build --release -vvv

build: $(SIMULATION_SRCS)

# Run single example
# run: $(SIMULATION_SRCS)
# cargo run --release \
# --bin xdma_driver \
# -- -s $(svfile) \
# -t $(top) \
# -i $(input_file) \
# -b $(lut_file) \
# --num-mods $(num_mods) \
# --num-procs $(num_procs) \
# --sram-width $(sram_width) \
# --sram-entries $(sram_entries) \
# --imem-lat $(imem_lat) \
# --inter-proc-nw-lat $(inter_proc_nw_lat) \
# --inter-mod-nw-lat $(inter_mod_nw_lat) \
# --max-steps $(max_steps) \
# --sim-dir $(sim_dir) | tee DEBUG

clean:
	@if [ -d "$(BUILDDIR)" ]; then \
		echo "Directory $(BUILDDIR) exists. Deleting..."; \
		rm -rf $(BUILDDIR); \
	else \
		echo "Directory $(BUILDDIR) does not exist. Skipping deletion."; \
	fi
	cargo clean
	-rm -rf build-dir
	-rm DEBUG
	-rm $(MILL_BUILD_ARTIFACTS)
	-rm $(DRIVER_GENERATED)

all: run

.PHONY: chisel_elaborate, build, run, clean, all
