use crate::passes::*;
use crate::primitives::*;
use dce::dead_code_elimination;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;
use set_rank::find_rank_order;
use partition::partition;
use check_rank::check_rank_order;
use print_stats::print_stats;

pub fn run_compiler_passes(c: &mut Circuit) {
    dead_code_elimination(c);
    print_stats(c);
    find_rank_order(c);
    check_rank_order(c);
    partition(c);
    println!("partition done");
    schedule_instructions(c);
    println!("schedule instructions done");
    map_instructions(c);
    println!("map instructions done");
}
