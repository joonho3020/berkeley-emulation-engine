use crate::passes::dce;
use crate::primitives::*;

pub fn run_compiler_passes(c: Circuit) -> Circuit {
    dce::dead_code_elimination(c)
}
