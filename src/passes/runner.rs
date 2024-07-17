use crate::passes::{
    dce::dead_code_elimination,
    partition::partition
};
use crate::primitives::*;

pub fn run_compiler_passes(c: Circuit) -> Circuit {
    let c = dead_code_elimination(c);
    partition(c)
}
