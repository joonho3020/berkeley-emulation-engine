use crate::rtlsim::rtlsim_utils::*;
use crate::common::utils;
use indexmap::IndexMap;
use std::cmp::max;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{env, fs};

/// Generates a testharness String
fn generate_testbench_string(
    input_stimuli: &InputStimuliMap,
    io: Vec<Port>,
    top: String,
) -> String {
    let mut testbench = "
`timescale 1 ns/10 ps

module testharness;

"
    .to_string();
    for p in io.iter() {
        let reg_or_wire = if p.input { "reg" } else { "wire" };
        if p.width == 1 {
            testbench.push_str(&format!("{} {};\n", reg_or_wire, p.name));
        } else {
            testbench.push_str(&format!(
                "{} [{}:0] {};\n",
                reg_or_wire,
                p.width - 1,
                p.name
            ));
        }
    }
    testbench.push_str(&format!(
        "
localparam T=20;
always begin
  #(T/2) clock <= ~clock;
end

initial begin
  clock  = 1'b1;
  reset = 1'b1;

  #(T/2*8) reset = 1'b1;

  $display($time, \" ** Start Simulation **\");

"
    ));
    let cycles = input_stimuli.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        let mut poke_str = "".to_string();

        // Need a #(0) after a posedge as we want to push inputs "after"
        // the posedge
        poke_str.push_str("  @(posedge clock);#(0);\n");

        // generate display message
        poke_str.push_str("  $display($time, \" ");
        for o in io.iter().filter(|x| !x.input) {
            poke_str.push_str(&format!("{} %x ", o.name));
        }
        poke_str.push_str(&format!("\""));
        for o in io.iter().filter(|x| !x.input) {
            poke_str.push_str(&format!(", top.{}", o.name));
        }
        poke_str.push_str(&format!(");\n"));

        // poke inputs
        for key in input_stimuli.keys() {
            let val = input_stimuli[key].get(cycle);
            match val {
                Some(b) => {
                    poke_str.push_str(&format!("  {} = {};\n", key, b));
                }
                None => {}
            }
        }
        poke_str.push_str("\n");

        testbench.push_str(&poke_str);
    }
    testbench.push_str(&format!(
        "
  $display($time, \" ** End Simulation **=\");
  $finish;
end

// dump the state of the design
// VCD (Value Change Dump) is a standard dump format defined in Verilog.
initial begin
  $dumpfile(\"sim.vcd\");
  $dumpvars(0, testharness);
end

{} top(
",
        top
    ));

    for (i, x) in io.iter().enumerate() {
        testbench.push_str(&format!("  .{}({})", x.name, x.name));
        if i < io.len() - 1 {
            testbench.push_str(",\n");
        } else {
            testbench.push_str("\n");
        }
    }
    testbench.push_str(
        ");
\nendmodule\n",
    );
    return testbench;
}

/// Reads from a verilog file, and returns the generated testharness String
/// when successfull
fn generate_testbench(
    file_path: &str,
    top_mod: &str,
    input_stimuli: &InputStimuliMap,
) -> Result<String, String> {
    let verilog_str = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Error while parsing:\n{}", e).to_string());
        }
    };

    let ports = get_io(verilog_str.to_string(), top_mod.to_string());
    let tb = generate_testbench_string(input_stimuli, ports, top_mod.to_string());
    Ok(tb)
}

pub fn run_rtl_simulation(
    sv_file_path: &str,
    top_mod: &str,
    input_stimuli_path: &str,
    sim_dir: &str,
    sim_output_file: &str,
) -> std::io::Result<()> {
    let input_stimuli = get_input_stimuli(input_stimuli_path);
    let tb = match generate_testbench(sv_file_path, top_mod, &input_stimuli) {
        Ok(x) => x,
        Err(e) => {
            return Err(std::io::Error::other(format!("{}", e)));
        }
    };

    let verilog_file = Path::new(sv_file_path);

    let tb_name = format!("{}-testbench.sv", top_mod).to_string();
    utils::write_string_to_file(tb, &tb_name)?;

    let mut cwd = env::current_dir()?;
    cwd.push(sim_dir.to_string());
    fs::create_dir_all(cwd.to_path_buf())?;

    Command::new("cp")
        .arg(&verilog_file)
        .arg(cwd.to_str().unwrap())
        .status()?;

    Command::new("mv").arg(&tb_name).arg(&cwd).status()?;

    Command::new("mkdir")
        .current_dir(&cwd)
        .arg("build")
        .status()?;

    Command::new("vcs")
        .current_dir(&cwd)
        .arg("-sverilog")
        .arg("-full64")
        .arg("+notimingchecks")
        .arg(&tb_name)
        .arg(verilog_file.file_name().unwrap())
        .arg("-o")
        .arg("build/rtlsim_binary")
        .status()?;

    let stdout = Command::new("./rtlsim_binary")
        .current_dir(cwd.join("build"))
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

    let mut sim_out_file =
        fs::File::create(format!("{}/{}", cwd.to_str().unwrap(), sim_output_file))?;
    sim_out_file.write(output_str.as_bytes())?;

    Ok(())
}

pub fn output_value_fmt(values: &InputStimuliMap) -> String {
    let mut output_str = "** Start Simulation **\n".to_string();
    let cycles = values.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        output_str.push_str(&format!("{}", cycle));
        for k in values.keys() {
            output_str.push_str(&format!(" {} {}", k, values[k][cycle]));
        }
        output_str.push_str("\n");
    }
    output_str.push_str("** End Simulation **\n");
    return output_str;
}
