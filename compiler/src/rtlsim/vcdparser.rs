use crate::common::primitive::FourStateBit;
use indexmap::IndexMap;
use indicatif::ProgressStyle;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use wellen::*;

const LOAD_OPTS: LoadOptions = LoadOptions {
    multi_thread: true,
    remove_scopes_with_empty_name: false,
};

pub struct WaveformDB {
    pub header: viewers::HeaderResult,
    pub body: viewers::BodyResult,
}

impl WaveformDB {
    pub fn new(vcd_file: String) -> WaveformDB {
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
    pub fn signal_values_at_cycle(self: &mut Self, cycle: u32) -> IndexMap<String, FourStateBit> {
        let hierarchy = &self.header.hierarchy;

        let mut ret: IndexMap<String, FourStateBit> = IndexMap::new();

        for var in hierarchy.get_unique_signals_vars().iter().flatten() {
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
                        let name = _signal_name.split('.').last().unwrap().to_string();
                        let sig_val = loaded_signal.get_value_at(&idx, elemidx);
                        let numbits = match sig_val.bits() {
                            Some(x) => x,
                            _ => 0,
                        };
                        let bits = match sig_val.to_bit_string() {
                            Some(bits_as_string) => bits_as_string,
                            _ => "".to_string(),
                        };
                        let bits_array: Vec<char> = bits.chars().rev().collect();
                        assert!(numbits == bits_array.len() as u32);
                        if numbits == 1 {
                            ret.insert(name, FourStateBit::from_char(bits_array[0]));
                        } else {
                            for bit in 0..numbits {
                                let val = FourStateBit::from_char(bits_array[bit as usize]);
                                let index = format!("[{}]", bit);
                                let mut name_bit = name.clone();
                                name_bit.push_str(&index);
                                ret.insert(name_bit, val);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        return ret;
    }
}
