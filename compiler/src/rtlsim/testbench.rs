use indexmap::IndexMap;
use std::cmp::max;
use std::fs;

#[derive(Debug)]
pub struct Port {
    pub name: String,
    pub width: u64,
    pub input: bool,
}

pub fn get_io(verilog_str: String, top: String) -> Vec<Port> {
    let mut collect_io = false;
    let mut cur_dir_input = true;
    let mut cur_bits_minus_one = 0;
    let mut ret: Vec<Port> = vec![];

    for line in verilog_str.lines() {
        let words: Vec<&str> = line.split(' ').filter(|x| *x != "").collect();
        if collect_io {
            if words.len() >= 1 && words[0] == ");" {
                break;
            } else if words.len() >= 3 {
                cur_dir_input = words[0] == "input";

                let x: Vec<&str> = words[1].split(':').collect();
                cur_bits_minus_one = x[0].replace('[', "").parse().unwrap();
            } else if words.len() == 2 {
                if words[0] == "input" || words[0] == "output" {
                    cur_dir_input = words[0] == "input";
                    cur_bits_minus_one = 0;
                } else {
                    let x: Vec<&str> = words[0].split(':').collect();
                    cur_bits_minus_one = x[0].replace('[', "").parse().unwrap();
                }
            }

            ret.push({
                Port {
                    name: words[words.len() - 1].replace(",", "").to_string(),
                    width: cur_bits_minus_one + 1,
                    input: cur_dir_input,
                }
            })
        } else if words.len() >= 2 && words[0] == "module" && words[1].replace('(', "") == top {
            collect_io = true;
        }
    }
    return ret;
}

pub fn generate_testbench_string(
    input_stimuli: IndexMap<String, Vec<u64>>,
    io: Vec<Port>,
    top: String,
) -> String {
    let mut testbench = "
`timescale 1 ns/10 ps

module testharness;

"
    .to_string();
    for p in io.iter() {
        if p.width == 1 {
            testbench.push_str(&format!("reg {};\n", p.name));
        } else {
            testbench.push_str(&format!("reg [{}:0] {};\n", p.width - 1, p.name));
        }
    }
    testbench.push_str(&format!(
        "
localparam T=20;
always begin
  #(T/2) clock = ~clock;
end

initial begin
  clock  = 1'b0;
  reset = 1'b0;

  #(T*2) reset = 1'b1;
  #(T*2) reset = 1'b0;

  $display($time, \" ** Start Simulation **\");

"
    ));
    let cycles = input_stimuli.values().fold(0, |x, y| max(x, y.len()));
    for cycle in 0..cycles {
        // generate display message
        let mut poke_str = "  $display($time, \" ".to_string();
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
        poke_str.push_str("  #(T);\n\n");
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

pub fn generate_testbench(
    file_path: &str,
    top_mod: &str,
    input_stimuli: IndexMap<String, Vec<u64>>,
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

pub fn get_input_stimuli(file_path: &str) -> IndexMap<String, Vec<u64>> {
    let input_str = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => "".to_string(),
    };

    let mut ret = IndexMap::new();
    let mut order = IndexMap::new();
    for (i, line) in input_str.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|x| *x != "").collect();
        for (j, w) in words.iter().enumerate() {
            if i == 0 {
                order.insert(j, w.to_string());
                ret.insert(w.to_string(), vec![]);
            } else {
                let k = &order[j];
                ret.get_mut(k).unwrap().push(w.parse().unwrap());
            }
        }
    }

    ret
}
