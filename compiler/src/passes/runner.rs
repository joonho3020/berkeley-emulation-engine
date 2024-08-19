use crate::passes::*;
use crate::primitives::*;
use crate::utils::write_string_to_file;
use dce::dead_code_elimination;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;
use set_rank::find_rank_order;
use proc_map::map_to_processor;

pub fn run_compiler_passes(c: &mut Circuit) {
    dead_code_elimination(c);
    find_rank_order(c);
    map_to_processor(c);

    let _ = write_string_to_file(
        format!("{:?}", &c),
        &format!("DEBUG.dot")
    );

    schedule_instructions(c);
    map_instructions(c);
}
