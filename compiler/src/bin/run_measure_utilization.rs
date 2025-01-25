use bee::common::config::*;
use bee::common::circuit::Circuit;
use bee::testing::try_new_circuit;
use clap::Parser;

#[derive(Debug)]
struct UtilizationInfo {
    pub avg: f32,
    pub max: f32,
    pub min: f32
}

fn meausre_utilization(circuit: &Circuit) -> f32 {
    let max_steps = circuit.emul.host_steps;
    let pcfg = &circuit.platform_cfg;
    let capacity = max_steps * pcfg.num_mods * pcfg.num_procs;
    let total_nodes = circuit.graph.node_count();

    let utilization = total_nodes as f32 / capacity as f32;
    return utilization;
}

fn collect_utilization_result(args: &Args, repeat: u32) -> std::io::Result<UtilizationInfo> {
    assert!(repeat > 0);

    let mut total = 0.0;
    let mut max = 0.0;
    let mut min = 100.0;

    for _ in 0..repeat {
        let c = try_new_circuit(args)?;
        let util = meausre_utilization(&c);
        total += util;
        max = if util > max  { util } else { max };
        min = if min  > util { util } else { min };
    }
    return Ok(UtilizationInfo {
        avg: total / repeat as f32,
        max: max,
        min: min
    });
}


fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let total_runs = 10;
    let util_info = collect_utilization_result(&args, total_runs)?;

    println!("Average utilization over {} runs: {:?}",
        total_runs, util_info);

    return Ok(());
}
