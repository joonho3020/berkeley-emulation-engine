use super::partition::{find_rank_order, map_to_processor};
use crate::passes::*;
use crate::primitives::*;
use inst_map::map_instructions;
use inst_schedule::schedule_instructions;

pub fn run_compiler_passes(mut c: Circuit, ctx: Context) -> Circuit {
    c.set_ctx(ctx);
    let c = dce::dead_code_elimination(c);
    let c_rank_order = find_rank_order(c);
    let c_proc_map = map_to_processor(c_rank_order);
    let c_inst_sched = schedule_instructions(c_proc_map);
    let c_inst_map = map_instructions(c_inst_sched);
    c_inst_map
}
