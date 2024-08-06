# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

VERILOG_SOURCES += $(PWD)/OpalKellyEmulatorModuleTop.sv
# use VHDL_SOURCES for VHDL files

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = OpalKellyEmulatorModuleTop

# MODULE is the basename of the Python test file
MODULE = test

WAVES=1

# EXTRA_ARGS += --trace --trace-structs

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
