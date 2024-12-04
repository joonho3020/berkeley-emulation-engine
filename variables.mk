# Variables for emulator platform
emul_debug         ?= "false"
max_steps          ?= 1024
num_mods           ?= 17
num_procs          ?= 64
imem_lat           ?= 1
inter_mod_nw_lat   ?= 1
inter_proc_nw_lat  ?= 1
sram_width         ?= 256
sram_entries       ?= 16384
blackbox_dmem      ?= "true"


# Variables for emulator platform
top        ?= "Adder"
sim_type   ?= "sims"
dir        := "../../examples/"
svfile     := $(dir)/$(top)".sv"
input_file := $(dir)/$(top)".input"
lut_file   := $(dir)/$(top)".lut.blif"
sim_dir    := "../sim-dir/$(sim_type)-"$(top)
