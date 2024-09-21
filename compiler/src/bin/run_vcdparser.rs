use bee::rtlsim::vcdparser::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let mut waveform_db = WaveformDB::new(filename.to_string());

    for i in 0..19 {
        println!("-----cycle: {} -----------", i);
        let ret = waveform_db.signal_values_at_cycle(i * 2);
        println!("{:?}", ret);
    }
}
