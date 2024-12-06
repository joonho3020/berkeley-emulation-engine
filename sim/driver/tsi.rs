use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct TSI {
    pub i: VecDeque<u32>,
    pub o: VecDeque<u32>
}
