EMULATOR_DIR  := ../../emulator
BUILDDIR=build-dir


# Variables for emulator platform
top        ?= "Adder"
dir        := "../../examples/"
svfile     := $(dir)/$(top)".sv"
input_file := $(dir)/$(top)".input"
lut_file   := $(dir)/$(top)".lut.blif"
sim_dir    := "../sim-dir/metasim-"$(top)


VERILATOR_LIB=libVdut.so

# Chisel generated driver stuff
SIMIF_DIR := src/simif
SIMIF_SRC_DIR := ../simif
SIMIF_SRCS       := $(shell find $(SIMIF_SRC_DIR) -name '*.rs')
SIMIF_FILE_NAMES := $(shell find $(SIMIF_SRC_DIR) -type f -name '*.rs' -printf "%f\n")
SIMIF_COPIED := $(foreach entry,$(SIMIF_FILE_NAMES),$(SIMIF_DIR)/$(entry))
DRIVER_SRCS := $(SIMIF_COPIED) $(SIMIF_DIR)/driver_generated.rs
SIMULATION_SRCS := $(VERILATOR_LIB) $(DRIVER_SRCS)

.DEFAULT_GOAL := test

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

include $(EMULATOR_DIR)/chisel.mk

# Build verilator shared library w/ c-bindings
$(SIMULATION_SRCS): $(MILL_BUILD_ARTIFACTS) $(SIMIF_SRCS)
	@if [ -d "$(BUILDDIR)/obj-dir" ]; then \
		echo "Directory $(BUILDDIR)/obj-dir exists. Deleting..."; \
		rm -rf $(BUILDDIR)/obj-dir; \
	fi
	cp $(SIMIF_SRCS) $(SIMIF_DIR)
	cargo clean
	cargo build --release

# Run single example
run: $(SIMULATION_SRCS)
	cargo run --release \
		--bin fpgatopsim \
		-- -s $(svfile) \
			 -t $(top) \
			 -i $(input_file) \
			 -b $(lut_file) \
			 --num-mods $(num_mods) \
			 --num-procs $(num_procs) \
			 --max-steps $(max_steps) \
			 --sram-width $(sram_width) \
			 --sram-entries $(sram_entries) \
			 --imem-lat $(imem_lat) \
			 --inter-proc-nw-lat $(inter_proc_nw_lat) \
			 --inter-mod-nw-lat $(inter_mod_nw_lat) \
			 --large-sram-width $(large_sram_width)      \
			 --large-sram-entries $(large_sram_entries)  \
			 --large-sram-cnt $(large_sram_cnt)          \
			 --sim-dir $(sim_dir) | tee DEBUG

# Run existing tests
test: $(SIMULATION_SRCS)
	cargo nextest run --release

clean:
	@if [ -d "$(BUILDDIR)" ]; then \
		echo "Directory $(BUILDDIR) exists. Deleting..."; \
		rm -rf $(BUILDDIR); \
	else \
		echo "Directory $(BUILDDIR) does not exist. Skipping deletion."; \
	fi
	cargo clean
	-rm -rf ../sim-dir/metasim-*
	-rm libVdut.so
	-rm -f *.vcd
	-rm -f *.so
	-rm -rf blif-sim-dir-*
	-rm -rf $(BUILDDIR)
	-rm -rf sim-dir
	-rm DEBUG
	-rm $(DRIVER_SRCS)

all: test

.PHONY: run, clean, test, all
