use crate::passes::*;
use crate::primitives::*;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;
use partition::{find_rank_order, map_to_processor};
use dce::dead_code_elimination;

pub fn run_compiler_passes(c: &mut Circuit) {
    dead_code_elimination(c);
    find_rank_order(c);
    map_to_processor(c);
    schedule_instructions(c);
    map_instructions(c);
}
