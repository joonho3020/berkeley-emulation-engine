use crate::passes::{dce::dead_code_elimination, partition::kaminpar_partition};
use crate::primitives::*;

use super::partition::find_rank_order;

pub fn run_compiler_passes(mut c: Circuit, ctx: Context) -> Circuit {
    c.set_ctx(ctx);
    let c = dead_code_elimination(c);
    let (partition, c) = kaminpar_partition(c);
    let mut ret = find_rank_order(c);
    let graph = &mut ret.graph;
    for nidx in graph.node_indices() {
        let node = graph.node_weight_mut(nidx).unwrap();
        let info = node.get_info();
        println!("NodeIndex: {}, node: {:?}", nidx.index(), node);
    }
    ret
}
