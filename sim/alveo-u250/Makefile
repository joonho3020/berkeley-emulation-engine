include ../../variables.mk

# Override default variables
emul_debug = "false"
sim_type = "alveo-u250"

EMULATOR_DIR  := ../../emulator
BUILDDIR=build-dir

# Chisel generated driver stuff
SIMIF_DIR := src/simif
SIMIF_SRC_DIR := ../simif
SIMIF_SRCS       := $(shell find $(SIMIF_SRC_DIR) -name '*.rs')
SIMIF_FILE_NAMES := $(shell find $(SIMIF_SRC_DIR) -type f -name '*.rs' -printf "%f\n")
SIMIF_COPIED := $(foreach entry,$(SIMIF_FILE_NAMES),$(SIMIF_DIR)/$(entry))
DRIVER_SRCS := $(SIMIF_COPIED) $(SIMIF_DIR)/driver_generated.rs

.DEFAULT_GOAL := build

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

include $(EMULATOR_DIR)/chisel.mk

# Build verilator shared library w/ c-bindings
$(DRIVER_SRCS): $(MILL_BUILD_ARTIFACTS) $(SIMIF_SRCS)
	@echo "Building $@ because of changes in $?"
	cargo build --release

build: $(DRIVER_SRCS)

# Run single example
run: $(DRIVER_SRCS)
	RUN_BACKTRACE=1 cargo run --release \
		--bin xdma-driver \
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
		--max-steps $(max_steps) \
		--sim-dir $(sim_dir) \
		--elf-file-path $(binary) | tee DEBUG

run_trace: $(DRIVER_SRCS)
	RUST_BACKTRACE=1 cargo run --release \
		--bin xdma-driver \
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
		--max-steps $(max_steps) \
		--sim-dir $(sim_dir) \
		--trace-mode \
		--elf-file-path $(binary) | tee DEBUG-TRACE

run_fsim: $(DRIVER_SRCS)
	RUST_BACKTRACE=1 cargo run --release \
		--bin xdma-driver \
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
		--max-steps $(max_steps) \
		--sim-dir $(sim_dir) \
		--fsim-mode \
		--elf-file-path $(binary) | tee DEBUG-TRACE

clean:
	@if [ -d "$(BUILDDIR)" ]; then \
		echo "Directory $(BUILDDIR) exists. Deleting..."; \
		rm -rf $(BUILDDIR); \
	else \
		echo "Directory $(BUILDDIR) does not exist. Skipping deletion."; \
	fi
	cargo clean
	-rm -rf build-dir
	-rm DEBUG*
	-rm $(SIMIF_DIR)/driver_generated.rs

all: run

.PHONY: build, run, clean, all
