use crate::common::primitive::Bit;
use indexmap::IndexMap;
use indicatif::ProgressStyle;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use wellen::*;
use itertools::Itertools;

pub type SignalMap = IndexMap<WaveformSignal, FourStateBit>;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FourStateBit {
    #[default]
    ZERO,
    ONE,
    X,
    Z,
}

impl FourStateBit {
    pub fn from_char(c: char) -> Self {
        match c {
            '0' => Self::ZERO,
            '1' => Self::ONE,
            'x' => Self::X,
            'z' => Self::Z,
            _ => Self::X,
        }
    }

    pub fn to_bit(self: &Self) -> Option<Bit> {
        match self {
            Self::ZERO => Some(0),
            Self::ONE => Some(1),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct WaveformSignal {
    path: Vec<String>,
}

impl WaveformSignal {
    pub fn new(path_: Vec<String>) -> Self {
        Self {
            path: path_
        }
    }

    pub fn hier(self: &Self) -> Vec<String> {
        let len = self.path.len();
        return self.path[..len-1].to_vec();
    }

    pub fn name(self: &Self) -> String {
        assert!(self.path.len() > 0, "WaveformSignal path is empty");
        return self.path.last().unwrap().to_string();
    }

    pub fn append(self: &mut Self, sig: String) {
        self.path.push(sig);
    }

    pub fn to_string(self: &Self) -> String {
        return self.path.join(".");
    }
}

impl From<String> for WaveformSignal {
    fn from(value: String) -> Self {
        let path_: Vec<String> = value.split('.').map(|s| s.to_string()).collect();
        Self {
            path: path_
        }
    }
}

const LOAD_OPTS: LoadOptions = LoadOptions {
    multi_thread: true,
    remove_scopes_with_empty_name: false,
};

pub struct WaveformDB {
    pub header: viewers::HeaderResult,
    pub body: viewers::BodyResult,
}

impl WaveformDB {
    pub fn new(vcd_file: &String) -> WaveformDB {
        let header = viewers::read_header(&vcd_file, &LOAD_OPTS).expect("Failed to load file!");
        let hierarchy = header.hierarchy;
        let body = header.body;

        // create body progress indicator
        let body_len = header.body_len;
        let (body_progress, progress) = if body_len == 0 {
            (None, None)
        } else {
            let p = Arc::new(AtomicU64::new(0));
            let p_out = p.clone();
            let done = Arc::new(AtomicBool::new(false));
            let done_out = done.clone();
            let ten_millis = std::time::Duration::from_millis(10);
            let t = thread::spawn(move || {
                let bar = indicatif::ProgressBar::new(body_len);
                bar.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {decimal_bytes} ({percent_precise}%)",
                    )
                    .unwrap(),
                );
                loop {
                    // always update
                    let new_value = p.load(Ordering::SeqCst);
                    bar.set_position(new_value);
                    thread::sleep(ten_millis);
                    // see if we are done
                    let now_done = done.load(Ordering::SeqCst);
                    if now_done {
                        bar.finish_and_clear();
                        break;
                    }
                }
            });

            (Some(p_out), Some((done_out, t)))
        };

        let body_ =
            viewers::read_body(body, &hierarchy, body_progress).expect("Failed to load body!");
        if let Some((done, t)) = progress {
            done.store(true, Ordering::SeqCst);
            t.join().unwrap();
        }

        // This is kind of stupid:
        // a way to get around the fact that body cannot be read w/o moving values out from the
        // "header", and read_body doesn't take borrowed types.
        let header2 = viewers::read_header(&vcd_file, &LOAD_OPTS).expect("Failed to load file!");

        return WaveformDB {
            header: header2,
            body: body_,
        };
    }

    /// Returns a signal name to bit value map for all signals at `cycle`
    pub fn signal_values_at_cycle(self: &mut Self, cycle: u32) -> SignalMap {
        let mut ret: SignalMap = SignalMap::new();

        let hierarchy = &self.header.hierarchy;
        for var in hierarchy.iter_vars() {
            let _signal_name: String = var.full_name(&hierarchy);
            let ids = [var.signal_ref(); 1];
            let loaded = self
                .body
                .source
                .load_signals(&ids, &hierarchy, LOAD_OPTS.multi_thread);
            let (_, loaded_signal) = loaded.into_iter().next().unwrap();

            let offset = loaded_signal.get_offset(cycle as u32);
            match offset {
                Some(idx) => {
                    for elemidx in 0..idx.elements {
                        let signal_path: Vec<String> = _signal_name.split('.').map(|s| s.to_string()).collect();
                        let name = signal_path.last().unwrap().clone();

                        let sig_val = loaded_signal.get_value_at(&idx, elemidx);
                        let numbits = match sig_val.bits() {
                            Some(x) => x,
                            _ => {
                                continue;
                            },
                        };
                        let bits = match sig_val.to_bit_string() {
                            Some(bits_as_string) => bits_as_string,
                            _ => "".to_string(),
                        };
                        let bits_array: Vec<char> = bits.chars().rev().collect();
                        assert!(numbits == bits_array.len() as u32);
                        if numbits == 1 {
                            let val = FourStateBit::from_char(bits_array[0]);
                            ret.insert(WaveformSignal::new(signal_path), val);
                        } else {
                            for bit in 0..numbits {
                                let val = FourStateBit::from_char(bits_array[bit as usize]);
                                let index = format!("[{}]", bit);

                                let mut name_bit = name.clone();
                                name_bit.push_str(&index);

                                let mut sp = signal_path.clone();
                                sp.pop();
                                sp.push(name_bit);

                                ret.insert(WaveformSignal::new(sp), val);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        return ret;
    }

    pub fn signal_values_at_cycle_rebase_top(self: &mut Self, cycle: u32, instance_path: String) -> IndexMap<String, FourStateBit> {
        let ref_signals = self.signal_values_at_cycle(cycle);
        let instance_depth = instance_path.split(".").collect_vec().len();

        let mut ret: IndexMap<String, FourStateBit> = IndexMap::new();
        for (signal_path, four_state_bit) in ref_signals.iter() {
            let name = signal_path.name();
            let mut hier = signal_path.hier();

            if hier.len() >= instance_depth {
                let hier_depth = &hier[..instance_depth];
                let hier_str = hier_depth.join(".");
                if hier_str == instance_path {
                    hier.drain(0..instance_depth);
                    hier.push(name.clone());
                } else {
                    continue;
                }
            }
            ret.insert(hier.join("."), four_state_bit.clone());
        }
        return ret;
    }

    pub fn hierarchy_to_signal(self: &mut Self) -> IndexMap<WaveformSignal, wellen::Signal> {
        let hierarchy = &self.header.hierarchy;
        let mut ret: IndexMap<WaveformSignal, wellen::Signal> = IndexMap::new();
        for var in hierarchy.iter_vars() {
            let _signal_name: String = var.full_name(&hierarchy);
            let ids = [var.signal_ref(); 1];
            let loaded = self
                .body
                .source
                .load_signals(&ids, &hierarchy, LOAD_OPTS.multi_thread);
            println!("_signal_name: {:?}", _signal_name);
            let (_, loaded_signal) = loaded.into_iter().next().unwrap();
            let signal_path: Vec<String> = _signal_name.split('.').map(|s| s.to_string()).collect();
            ret.insert(WaveformSignal::new(signal_path), loaded_signal);
        }
        return ret;
    }
}
