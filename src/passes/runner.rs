use super::partition::{find_rank_order, map_to_processor};
use crate::passes::*;
use crate::primitives::*;

pub fn run_compiler_passes(mut c: Circuit, ctx: Context) -> Circuit {
    c.set_ctx(ctx);
    let c = dce::dead_code_elimination(c);
    let c_rank_order = find_rank_order(c);
    let c_proc_map = map_to_processor(c_rank_order);

    // let graph = &mut ret.graph;
    // for nidx in graph.node_indices() {
    // let node = graph.node_weight_mut(nidx).unwrap();
    // let info = node.get_info();
    // println!("NodeIndex: {}, node: {:?}", nidx.index(), node);
    // }
    c_proc_map
}
