
use crate::primitives::*;
use crate::passes::dce;

pub fn run_compiler_passes(c: Circuit) -> Circuit {
    dce::dead_code_elimination(c)
}
