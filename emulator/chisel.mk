debug             ?= "true"
max_steps         ?= 128
num_mods          ?= 9
num_procs         ?= 8
imem_lat          ?= 1
inter_mod_nw_lat  ?= 0
inter_proc_nw_lat ?= 0
sram_width        ?= 16
sram_entries      ?= 16
blackbox_dmem     ?= "false"

# Chisel directories and files
SCALA_SRC_DIR := $(EMULATOR_DIR)/src
SCALA_FILES   := $(shell find $(SCALA_SRC_DIR) -name '*.scala')

MILL_BUILD     := $(EMULATOR_DIR)/out/mill.lock
FPGATOP_SV     := $(BUILDDIR)/FPGATop.sv
FPGATOP_MMAP   := $(BUILDDIR)/FPGATop.mmap
FPGATOP_ANNOS  := $(BUILDDIR)/FPGATop.annos
MILL_BUILD_ARTIFACTS := $(MILL_BUILD) $(FPGATOP_SV) $(FPGATOP_MMAP) $(FPGATOP_ANNOS)

# Chisel generated driver stuff
chisel_elaborate: $(MILL_BUILD_ARTIFACTS)

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
			--sram-entries $(sram_entries)           \
			--blackbox-dmem $(blackbox_dmem)
	@touch $(MILL_BUILD) # Update mill lock file timestamp
	sed -i '/.*\.v$$/d' $(EMULATOR_DIR)/FPGATop.sv # Remove last line if blackbox was used
	cp $(EMULATOR_DIR)/FPGATop.sv   $(BUILDDIR)/
	cp $(EMULATOR_DIR)/FPGATop.mmap $(BUILDDIR)/
	cp $(EMULATOR_DIR)/FPGATop.annos $(BUILDDIR)/

.PHONY: chisel_elaborate
