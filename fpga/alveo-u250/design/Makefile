BUILD_SUFFIX := $(shell date +%Y-%m-%d_%H-%M)
TOP_DIR := $(shell pwd)

NUM_THREADS ?= 8

VIVADO_PATH ?= $(shell which vivado)
VIVADO_OPTS ?= -nolog -nojournal -mode batch

# Variables for emulator platform
debug              ?= "true"
max_steps          ?= 1024
num_mods           ?= 5
num_procs          ?= 4
imem_lat           ?= 1
inter_mod_nw_lat   ?= 1
inter_proc_nw_lat  ?= 1
sram_width         ?= 16
sram_entries       ?= 16
large_sram_width   ?= 16
large_sram_entries ?= 16
large_sram_cnt     ?= 0
blackbox_dmem      ?= "true"

PROJECT_DIR = $(TOP_DIR)/project

# Chisel directories and files
EMULATOR_DIR := ../../../emulator
BUILDDIR     := $(PROJECT_DIR)/srcs/$(BUILD_SUFFIX)

BOARD_PART_TCL   = $(TOP_DIR)/au250.tcl
VLOG_SOURCES     += $(shell find $(TOP_DIR)/rtl            -type f -name "*.v")
CONSTRAINTS      += $(shell find $(TOP_DIR)/constraints    -type f -name "*.xdc")

VLOG_SOURCES     += $(shell find $(BUILDDIR)            -type f -name "*.v")
SVLOG_SOURCES    += $(shell find $(BUILDDIR)            -type f -name "*.sv")
CONSTRAINTS      += $(shell find $(BUILDDIR)            -type f -name "*.xdc")

IP               += $(shell find $(TOP_DIR)/ip -maxdepth 2 -type f -name "*.xci")
TOP_MODULE = XilinxU250Board


.DEFAULT_GOAL := all

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

include $(EMULATOR_DIR)/chisel.mk

$(PROJECT_DIR)/target.tcl: $(VLOG_SOURCES) $(CONSTRAINTS) $(IP) $(MILL_BUILD_ARTIFACTS)
	@echo "source                             $(BOARD_PART_TCL)"    > $@
	@echo "set TOP_MODULE                     $(TOP_MODULE)"    >> $@
	@echo "set TOP_DIR                        $(TOP_DIR)"    >> $@
	@echo "set PROJECT_DIR                    $(PROJECT_DIR)"    >> $@
	@echo "set BUILD_SUFFIX                   $(BUILD_SUFFIX)"    >> $@
	@echo "set_param general.maxThreads       $(NUM_THREADS)"    >> $@
	@echo "set_param general.maxBackupLogs    0"    >> $@
	@echo -n "set VLOG_SOURCES { " >> $@
	@FLIST="$(VLOG_SOURCES)"; for f in $$FLIST; do echo -n "$$f " ; done >> $@
	@echo "}" >> $@
	@echo -n "set SVLOG_SOURCES { " >> $@
	@FLIST="$(SVLOG_SOURCES)"; for f in $$FLIST; do echo -n "$$f " ; done >> $@
	@echo "}" >> $@
	@echo -n "set CONSTRAINTS { " >> $@
	@FLIST="$(CONSTRAINTS)"; for f in $$FLIST; do echo -n "$$f " ; done >> $@
	@echo "}" >> $@
	@echo -n "set IP { " >> $@
	@FLIST="$(IP)"; for f in $$FLIST; do echo -n "$$f " ; done >> $@
	@echo "}" >> $@

setup: $(PROJECT_DIR)/target.tcl

$(PROJECT_DIR)/synth/latest/$(TOP_MODULE).dcp $(PROJECT_DIR)/synth/latest/$(TOP_MODULE).xdc: $(PROJECT_DIR)/target.tcl synthesis.tcl
	mkdir -p $(PROJECT_DIR)/synth/$(BUILD_SUFFIX)
	cd $(PROJECT_DIR)/synth/$(BUILD_SUFFIX) && \
	$(VIVADO_PATH) $(VIVADO_OPTS) -source $(TOP_DIR)/synthesis.tcl |& tee synth.log

syn: $(PROJECT_DIR)/synth/latest/$(TOP_MODULE).dcp

$(BUILDDIR)/$(TOP_MODULE).bit: $(PROJECT_DIR)/target.tcl impl.tcl $(PROJECT_DIR)/synth/latest/$(TOP_MODULE).dcp $(PROJECT_DIR)/synth/latest/$(TOP_MODULE).xdc
	mkdir -p $(PROJECT_DIR)/impl/$(BUILD_SUFFIX)
	cd $(PROJECT_DIR)/impl/$(BUILD_SUFFIX) && \
	$(VIVADO_PATH) $(VIVADO_OPTS) -source $(TOP_DIR)/impl.tcl |& tee impl.log
	cp $(PROJECT_DIR)/impl/latest/$(TOP_MODULE).bit $@

impl: $(BUILDDIR)/$(TOP_MODULE).bit

all: $(BUILDDIR)/$(TOP_MODULE).bit

ip_project:
	$(VIVADO_PATH) $(VIVADO_OPTS) -source $(TOP_DIR)/create_ip_project.tcl

clean:
	rm -rf $(PROJECT_DIR)/*

delete:
	rm -rf $(PROJECT_DIR)/*
	rm -rf ip*

.PHONY: setup synth impl all clean delete
