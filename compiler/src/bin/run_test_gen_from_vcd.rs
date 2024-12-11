use bee::rtlsim::{rtlsim_utils::get_io, vcdparser::*};
use bee::common::utils::write_string_to_file;
use indexmap::{IndexSet, IndexMap};
use clap::Parser;
use std::fs;
use std::cmp::max;
use bee::rtlsim::rtlsim_utils::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// SystemVerilog file path
    #[arg(short, long, default_value = "")]
    pub sv_file_path: String,

    /// VCD file path
    #[arg(short, long)]
    pub vcd: String,

    /// Name of the top module
    #[arg(short, long)]
    pub top_mod: String,

    /// Instance path
    #[arg(short, long)]
    pub instance_path: String,

    /// output file path
    #[arg(short, long)]
    pub output: String,

    /// clock starts low
    #[arg(short, long)]
    pub clock_start_low: bool,

    /// timesteps per cycle
    #[arg(short, long, default_value_t = 2)]
    pub timesteps_per_cycle: u32,

    /// number of cycles to skip when parsing reference rtl sim vcd
    #[arg(long, default_value_t = 4)]
    pub ref_skip_cycles: u32,
}
fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let verilog_str = match fs::read_to_string(&args.sv_file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(std::io::Error::other(format!(
                "Error while parsing:\n{}",
                e
            )));
        }
    };

    let mut ret = "".to_string();
    let ios = get_io(verilog_str, args.top_mod);

    let mut signals: IndexSet<WaveformSignal> = IndexSet::new();
    for io in ios {
        if io.input && !is_clock_signal(&io.name) {
            assert!(io.width <= 64, "input width {} > 64", io.width);
            let mut s = WaveformSignal::from(args.instance_path.clone());
            s.append(io.name.clone());
            signals.insert(s);

            ret.push_str(&format!("{} ", io.name));
        }
    }
    ret.push_str("\n");

    let mut waveform_db = WaveformDB::new(&args.vcd.to_string());
    let h2s: IndexMap<WaveformSignal, wellen::Signal> = waveform_db
        .hierarchy_to_signal()
        .into_iter()
        .filter(|(k, _)| signals.contains(k))
        .collect();

    let max_steps = h2s
        .values()
        .map(|s| *s.time_indices().last().unwrap_or(&0))
        .reduce(|a, b| max(a, b))
        .unwrap_or(0);
    println!("max_steps: {}", max_steps);

    assert!(args.timesteps_per_cycle > 0,
        "timesteps_per_cycle should be a nonzero integer");

    let max_cycles = max_steps / args.timesteps_per_cycle;
    let offset = if args.clock_start_low { 1 } else { 0 };

    for cycle in args.ref_skip_cycles..max_cycles {
        let step = cycle * args.timesteps_per_cycle + offset;
        for (h, s) in h2s.iter() {
            match s.get_offset(step) {
                Some(idx) => {
                    assert!(idx.elements == 1);
                    for elemidx in 0..idx.elements {
                        let binary = &s.get_value_at(&idx, elemidx).to_string();
                        let decimal = binary_to_decimal(binary).unwrap_or(0);
                        ret.push_str(&format!("{:width$} ", decimal, width = h.name().len()));
                    }
                }
                None => { }
            }
        }
        ret.push_str("\n");
    }

    write_string_to_file(ret, &args.output)?;

    return Ok(());
}
