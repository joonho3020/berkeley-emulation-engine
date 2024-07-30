#[path = "../rtlsim/testbench.rs"]
pub mod rtlsim;
use crate::rtlsim::*;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{env, fs};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Usage: cargo run --bin rtlsim -- <sv input path> <top module name> <input stimuli file>");
        return Ok(());
    }

    let file_path = &args[1];
    let top_mod = &args[2];
    let input_stimuli = get_input_stimuli(&args[3]);
    let tb = match generate_testbench(file_path, top_mod, &input_stimuli) {
        Ok(x) => x,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    let verilog_file = Path::new(file_path);

    let tb_name = format!("{}-testbench.sv", top_mod);
    let mut tb_file = fs::File::create(&tb_name)?;
    tb_file.write(tb.as_bytes())?;

    let sim_dir = format!("sim-dir-{}", top_mod);
    let mut cwd = env::current_dir()?;
    cwd.push(sim_dir.to_string());
    fs::create_dir_all(cwd.to_path_buf())?;

    Command::new("cp")
        .arg(&verilog_file)
        .arg(cwd.to_str().unwrap())
        .status()?;

    Command::new("mv").arg(&tb_name).arg(&cwd).status()?;

    Command::new("verilator")
        .current_dir(&cwd)
        .arg("--binary")
        .arg(&tb_name)
        .arg(verilog_file.file_name().unwrap())
        .arg("-o")
        .arg("rtlsim_binary")
        .arg("--Mdir")
        .arg("build")
        .status()?;

    let stdout = Command::new("./rtlsim_binary")
        .current_dir(cwd.join("build"))
        .arg("--help")
        .output()?
        .stdout;

    let output = match String::from_utf8(stdout) {
        Ok(o) => o,
        _ => {
            return Err(std::io::Error::other(
                "Output from RTL simulation corrupted",
            ));
        }
    };

    let start_simulation_tag = "** Start Simulation **";
    let end_simulation_tag = "** End Simulation **";
    let mut start_collecting = false;
    let mut output_str = "".to_string();

    for line in output.lines() {
        if start_collecting && line.contains(end_simulation_tag) {
            output_str.push_str(end_simulation_tag);
            output_str.push_str("\n");
            break;
        } else if start_collecting {
            let mut words: Vec<&str> = line.split(' ').filter(|x| *x != "").collect();
            let timestamp: u64 = words[0].parse().unwrap();
            words.remove(0);

            // TODO: fix hardcoded rtl sim period
            let cycle = (timestamp - 80) / 20;
            output_str.push_str(&format!("{} ", cycle));
            output_str.push_str(&words.join(" "));
            output_str.push_str("\n");
        } else if line.contains(start_simulation_tag) {
            start_collecting = true;
            output_str.push_str(start_simulation_tag);
            output_str.push_str("\n");
        }
    }

    let mut sim_out_file = fs::File::create(format!(
        "{}/{}-simulation.out",
        cwd.to_str().unwrap(),
        top_mod
    ))?;
    sim_out_file.write(output_str.as_bytes())?;

    Ok(())
}
