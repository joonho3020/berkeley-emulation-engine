use crate::passes::*;
use crate::common::circuit::Circuit;
use split_sram_nodes::split_sram_nodes;
use split_reg_nodes::split_reg_nodes;
use replicate_consts::replicate_consts;
use dce::dead_code_elimination;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;
use set_rank::find_rank_order;
use partition::partition;
use check_rank::check_rank_order;
use check_connectivity::check_connectivity;
use print_stats::print_stats;
use std::time::Instant;

pub fn run_compiler_passes(c: &mut Circuit) {
    let dce_start = Instant::now();
    dead_code_elimination(c);
    let dce_time = dce_start.elapsed().as_millis();
    println!("DCE done");

    let partition_start = Instant::now();
    partition(c);
    let partition_time = partition_start.elapsed().as_millis();
    println!("Partition done");

    let sram_const_start = Instant::now();
    split_reg_nodes(c);
// check_connectivity(c);

    split_sram_nodes(c);
// check_connectivity(c);

    replicate_consts(c);
// check_connectivity(c);
    let sram_const_time = sram_const_start.elapsed().as_millis();
    println!("SRAM and constant replication done");

    let rank_start = Instant::now();
    find_rank_order(c);
    check_rank_order(c);
    let rank_time = rank_start.elapsed().as_millis();
    println!("Set rank order done");

    let schedule_start = Instant::now();
    schedule_instructions(c);
    let schedule_time = schedule_start.elapsed().as_millis();
    println!("Scheduling done");

    let map_start = Instant::now();
    map_instructions(c);
    let map_time = map_start.elapsed().as_millis();
    println!("Mapping done");

    print_stats(c);

    let compiler_time = dce_time + rank_time + partition_time + sram_const_time + schedule_time + map_time;
    println!("===============================");
    println!("Compiler Execution Time");
    println!("===============================");
    println!("DCE       : {} % {} ms", dce_time         as f32 / compiler_time as f32 * 100f32, dce_time);
    println!("rank      : {} % {} ms", rank_time        as f32 / compiler_time as f32 * 100f32, rank_time);
    println!("partition : {} % {} ms", partition_time   as f32 / compiler_time as f32 * 100f32, partition_time);
    println!("sram_const: {} % {} ms", sram_const_time  as f32 / compiler_time as f32 * 100f32, sram_const_time);
    println!("schedule  : {} % {} ms", schedule_time    as f32 / compiler_time as f32 * 100f32, schedule_time);
    println!("map       : {} % {} ms", map_time         as f32 / compiler_time as f32 * 100f32, map_time);
    println!("===============================");
}
