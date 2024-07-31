#[path = "../rtlsim/vcdparser.rs"]
pub mod rtlsim;
use crate::rtlsim::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let mut waveform_db = WaveformDB::new(filename.to_string());
    waveform_db.print_all_signals();
}
