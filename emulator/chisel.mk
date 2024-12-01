debug              ?= "true"
emul_debug         ?= "false"
max_steps          ?= 128
num_mods           ?= 9
num_procs          ?= 8
imem_lat           ?= 1
inter_mod_nw_lat   ?= 0
inter_proc_nw_lat  ?= 0
sram_width         ?= 16
sram_entries       ?= 16
large_sram_width   ?= 16
large_sram_entries ?= 16
large_sram_cnt     ?= 0
blackbox_dmem      ?= "false"

# Chisel directories and files
SCALA_SRC_DIR := $(EMULATOR_DIR)/src
SCALA_FILES   := $(shell find $(SCALA_SRC_DIR) -name '*.scala')

GENERATED_DIR := generated-m$(num_mods).p$(num_procs).s$(max_steps).nwl$(inter_proc_nw_lat).nwg$(inter_mod_nw_lat).sw$(sram_width).se$(sram_entries)
MILL_BUILD     := $(EMULATOR_DIR)/out/mill.lock
MILL_BUILD_ARTIFACTS := $(MILL_BUILD) $(BUILDDIR)/synth.xdc

# Chisel generated driver stuff
chisel_elaborate: $(MILL_BUILD_ARTIFACTS)

# Mill rebuild rule
$(MILL_BUILD_ARTIFACTS): $(SCALA_FILES) | $(BUILDDIR)
	@echo "Changes detected in Scala files. Rebuilding with Mill..."
	cd $(EMULATOR_DIR) &&                           \
		mill emulator.run                             \
			--debug $(debug)                            \
			--emul-debug $(emul_debug)                  \
			--max-steps $(max_steps)                    \
			--num-mods $(num_mods)                      \
			--num-procs $(num_procs)                    \
			--imem-lat $(imem_lat)                      \
			--inter-mod-nw-lat $(inter_mod_nw_lat)      \
			--inter-proc-nw-lat $(inter_proc_nw_lat)    \
			--sram-width $(sram_width)                  \
			--sram-entries $(sram_entries)              \
			--large-sram-width $(large_sram_width)      \
			--large-sram-entries $(large_sram_entries)  \
			--large-sram-cnt $(large_sram_cnt)          \
			--blackbox-dmem $(blackbox_dmem)
	cd $(EMULATOR_DIR) &&                        \
		python build-xdc.py                        \
			--max-steps $(max_steps)                 \
			--num-mods $(num_mods)                   \
			--num-procs $(num_procs)                 \
			--inter-mod-nw-lat $(inter_mod_nw_lat)   \
			--inter-proc-nw-lat $(inter_proc_nw_lat) \
			--sram-width $(sram_width)               \
			--sram-entries $(sram_entries)
	@touch $(MILL_BUILD) # Update mill lock file timestamp
	cp -r $(EMULATOR_DIR)/$(GENERATED_DIR)/* $(BUILDDIR)/

.PHONY: chisel_elaborate
