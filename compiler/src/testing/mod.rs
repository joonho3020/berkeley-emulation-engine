pub mod fsim;
pub mod blifsim;

use std::env;
use std::process::Command;
use crate::common::config::*;
use crate::common::circuit::*;
use crate::passes::blif_to_circuit::blif_to_circuit;
use crate::passes::runner::run_compiler_passes;

pub fn try_new_circuit(args: &Args) -> std::io::Result<Circuit> {
    let mut cwd = env::current_dir()?;
    cwd.push(args.sim_dir.clone());
    Command::new("mkdir").arg(&cwd).status()?;

    println!("Parsing blif file");
    let res = blif_to_circuit(&args.blif_file_path);
    let mut circuit = match res {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    assert!(args.large_sram_width >= args.sram_width);
    assert!(args.large_sram_entries >= args.sram_entries);

    circuit.set_cfg(
        PlatformConfig {
            num_mods:           args.num_mods,
            num_procs:          args.num_procs,
            max_steps:          args.max_steps,
            lut_inputs:         args.lut_inputs,
            inter_proc_nw_lat:  args.inter_proc_nw_lat,
            inter_mod_nw_lat:   args.inter_mod_nw_lat,
            imem_lat:           args.imem_lat,
            dmem_rd_lat:        args.dmem_rd_lat,
            dmem_wr_lat:        args.dmem_wr_lat,
            sram_width:         args.sram_width,
            sram_entries:       args.sram_entries,
            sram_rd_ports:      args.sram_rd_ports,
            sram_wr_ports:      args.sram_wr_ports,
            sram_rd_lat:        args.sram_rd_lat,
            sram_wr_lat:        args.sram_wr_lat,
            sram_ip_pl:         args.sram_ip_pl,
            large_sram_width:   args.large_sram_width,
            large_sram_entries: args.large_sram_entries,
            large_sram_cnt:     args.large_sram_cnt,
            topology: GlobalNetworkTopology::new(args.num_mods, args.num_procs)
        },
        CompilerConfig {
            top_module: args.top_mod.clone(),
            output_dir: cwd.to_str().unwrap().to_string(),
            dbg_tail_length: args.dbg_tail_length,
            dbg_tail_threshold: args.dbg_tail_threshold,
        }
    );

    println!("Running compiler passes with config: {:#?}", &circuit.platform_cfg);
    run_compiler_passes(&mut circuit);
    println!("Compiler pass finished");

    circuit.save_emulator_instructions()?;
    circuit.save_emulator_sigmap()?;
    return Ok(circuit);
}
