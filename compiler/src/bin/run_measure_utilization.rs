use bee::common::config::*;
use bee::common::circuit::Circuit;
use bee::testing::try_new_circuit;
use clap::Parser;
use std::fmt::Debug;
use itertools::Itertools;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct ProfileArgs {
    #[clap(flatten)]
    pub bee_args: Args,

    #[arg(long, default_value_t = 10)]
    pub iterations: u32,

    #[arg(long, default_value_t = 3)]
    pub ldm_port_cnt: u32,

    #[arg(long, default_value_t = 3)]
    pub sdm_port_cnt: u32,
}

struct UtilizationInfo {
    pub cnt: u32,
    pub sum: f32,
    pub max: f32,
    pub min: f32
}

impl UtilizationInfo {
    pub fn add(self: &mut Self, util: f32) {
        self.max = if self.max > util { self.max } else { util };
        self.min = if self.min < util { self.min } else { util };
        self.cnt += 1;
        self.sum += util;
    }

    pub fn profile(circuit: &Circuit) -> f32 {
        let max_steps = circuit.emul.host_steps;
        let pcfg = &circuit.platform_cfg;
        let capacity = max_steps * pcfg.num_mods * pcfg.num_procs;
        let total_nodes = circuit.graph.node_count();

        let utilization = total_nodes as f32 / capacity as f32;
        return utilization;
    }
}

impl Debug for UtilizationInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Utilization min: {}% max: {}% avg: {}%",
            self.min * 100.0, self.max * 100.0, self.sum / self.cnt as f32 * 100.0)
    }
}

impl Default for UtilizationInfo {
    fn default() -> Self {
        Self {
            cnt: 0,
            sum: 0.0,
            max: 0.0,
            min: 100.0
        }
    }
}

#[derive(Default)]
struct PortContentionInfo {
    pub merge_pairs: u32,
    pub total_insts: u32,
    pub has_operands: u32,
    pub ldm_ports: u32,
    pub sdm_ports: u32,
}

impl PortContentionInfo {
    pub fn add(self: &mut Self, other: &Self) {
        self.merge_pairs += other.merge_pairs;
        self.total_insts += other.total_insts;

        self.has_operands += other.has_operands;
        self.ldm_ports += other.ldm_ports;
        self.sdm_ports += other.sdm_ports;
    }

    pub fn percent(self: &Self) -> f32 {
        self.merge_pairs as f32 / self.total_insts as f32 * 100.0
    }

    pub fn profile(circuit: &Circuit, ldm_port_cnt: u32, sdm_port_cnt: u32) -> Self {
        let module_mapping = &circuit.emul.module_mappings;

        let mut merge_pairs = 0;
        let mut total_insts = 0;

        let mut ldm_port = 0;
        let mut sdm_port = 0;
        let mut has_operands = 0;

        for (_, mmap) in module_mapping.iter() {
            for (_, pmap) in mmap.proc_mappings.iter() {
                let insts = &pmap.instructions;
                for (a, b) in insts.iter().tuple_windows() {
                    let ap = a.ports_used();
                    let bp = b.ports_used();

                    if ap.0.unwrap_or(0) + bp.0.unwrap_or(0) <= ldm_port_cnt &&
                        ap.1.unwrap_or(0) + bp.1.unwrap_or(0) <= sdm_port_cnt {
                            merge_pairs += 1;
                    }
                    total_insts += 1;

                    ldm_port += ap.0.unwrap_or(0);
                    sdm_port += ap.1.unwrap_or(0);
                    has_operands += if a.operands.len() > 0 { 1 } else { 0 };
                }
            }
        }

        Self {
            merge_pairs: merge_pairs,
            total_insts: total_insts,
            ldm_ports: ldm_port,
            sdm_ports: sdm_port,
            has_operands: has_operands,
        }
    }
}

impl Debug for PortContentionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Total inst pairs {}, mergeable inst pairs {}, percentage: {}%",
            self.total_insts, self.merge_pairs, self.percent())?;

        writeln!(f, "Avg LDM port cnt {}%, SDM port cnt {}%",
            self.ldm_ports as f32 / self.has_operands as f32,
            self.sdm_ports as f32 / self.has_operands as f32)
    }
}

struct ProfileInfo {
    pub port: PortContentionInfo,
    pub util: UtilizationInfo
}

impl Debug for ProfileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self.util)?;
        write!(f, "{:?}", self.port)
    }
}

fn collect_stats(args: &ProfileArgs) -> std::io::Result<ProfileInfo> {
    assert!(args.iterations > 0);

    let mut port_contention = PortContentionInfo::default();
    let mut utilization = UtilizationInfo::default();

    for _ in 0..args.iterations {
        let c = try_new_circuit(&args.bee_args)?;

        println!("Collecting statistics....");
        utilization.add(UtilizationInfo::profile(&c));
        port_contention.add(&PortContentionInfo::profile(&c, args.ldm_port_cnt, args.sdm_port_cnt));
    }

    return Ok(ProfileInfo {
        util: utilization,
        port: port_contention
    });
}

fn main() -> std::io::Result<()> {
    let args = ProfileArgs::parse();
    println!("{:#?}", args);

    let stats = collect_stats(&args)?;
    println!("{:#?}", stats);

    return Ok(());
}
