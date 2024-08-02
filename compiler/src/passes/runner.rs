use crate::passes::*;
use crate::primitives::*;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;
use partition::{find_rank_order, map_to_processor};
use dce::dead_code_elimination;

pub fn run_compiler_passes(c: &mut Circuit) {
    dead_code_elimination(c);
    println!("dce done");
    find_rank_order(c);
    println!("rank order done");
    map_to_processor(c);
    println!("processor mapping done");
    schedule_instructions(c);
    println!("scheduling done");
    map_instructions(c);
    println!("instruction generation done");
}
