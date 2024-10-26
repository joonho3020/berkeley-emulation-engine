use std::process::Command;
use std::env;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::io::{self, BufRead, Write, BufWriter};

pub struct Args {
    pub sv_file_path: String,
    pub build_dir: String
}

#[derive(Debug)]
struct Signal {
    input: bool, // "in" or "out"
    name: String,
    bits: u32,
}

fn parse_file(file_path: &str) -> io::Result<Vec<Signal>> {
    let mut signals = Vec::new();
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?.trim().to_string();
        if line.contains("VL_IN") || line.contains("VL_OUT") {
            let parts: Vec<String> = line
                .replace("(", ",") // Replace '(' with ',' to unify delimiter
                .replace(")", ",") // Replace ')' with ',' to unify delimiter
                .replace("&", ",") // Replace ')' with ',' to unify delimiter
                .split(',')        // Split by ','
                .map(|s| {
                    s.trim().to_string()
                }) // Trim whitespace around parts
                .collect();

            if parts.len() >= 5 {
                // Extract direction, name, and bit width
                let input = if parts[0].contains("VL_IN") { true } else { false };
                let name = parts[2].to_string();
                let bits: u32 = parts[3].parse::<u32>().unwrap() - parts[4].parse::<u32>().unwrap() + 1;

                // Store the signal in the vector
                signals.push(Signal { input, name, bits });
            }
        }
    }
    Ok(signals)
}

fn generate_c_bindings(top: &str, signals: &Vec<Signal>, output_path: &str) -> io::Result<()> {
    let vtop = format!("V{}", top);
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "#include <inttypes.h>")?;
    writeln!(writer, "#include \"verilated.h\"")?;
    writeln!(writer, "#include \"verilated_vcd_c.h\"")?;
    writeln!(writer, "#include \"V{}.h\"", top)?;
    writeln!(writer, "extern \"C\" {{\n")?;

    writeln!(writer, "    {}* {}_new() {{", vtop, top)?;
    writeln!(writer, "        return new {};", vtop)?;
    writeln!(writer, "    }}\n")?;

    writeln!(writer, "    void {}_eval({}* dut) {{", top, vtop)?;
    writeln!(writer, "        dut->eval();")?;
    writeln!(writer, "    }}\n")?;

    writeln!(writer, "    void {}_delete({}* dut) {{", top, vtop)?;
    writeln!(writer, "        delete dut;")?;
    writeln!(writer, "    }}\n")?;

    writeln!(writer, "    VerilatedVcdC* enable_trace({}* dut) {{", vtop)?;
    writeln!(writer, "        // Enable tracing for waveform generation")?;
    writeln!(writer, "        Verilated::traceEverOn(true);")?;
    writeln!(writer, "        VerilatedVcdC* tfp = new VerilatedVcdC;")?;
    writeln!(writer, "        dut->trace(tfp, 99);")?;
    writeln!(writer, "        tfp->open(\"sim.vcd\");")?;
    writeln!(writer, "        return tfp;")?;
    writeln!(writer, "    }}\n")?;

    writeln!(writer, "     void close_trace(VerilatedVcdC* tfp) {{")?;
    writeln!(writer, "       tfp->close();")?;
    writeln!(writer, "     }}\n")?;

    writeln!(writer, "    void dump_vcd(VerilatedVcdC* tfp, unsigned int i) {{")?;
    writeln!(writer, "      tfp->dump(i);")?;
    writeln!(writer, "    }}\n")?;

    // Write the generated functions
    for signal in signals {
        assert!(signal.bits <= 64, "Signal {} width {} :(", signal.name, signal.bits);

        if signal.input {
            writeln!(writer, "    void poke_{} ({}* dut, uint64_t {}) {{", signal.name, vtop, signal.name)?;
            writeln!(writer, "        dut->{} = {};", signal.name, signal.name)?;
            writeln!(writer, "    }}\n")?;
        } else {
            writeln!(writer, "    uint64_t peek_{} ({}* dut) {{", signal.name, vtop)?;
            writeln!(writer, "        return dut->{};", signal.name)?;
            writeln!(writer, "    }}\n")?;
        }
    }

    writeln!(writer, "}} // extern \"C\"\n")?;

    Ok(())
}

fn generate_rust_bindings(top: &str, signals: &Vec<Signal>, output_path: &str) -> io::Result<()> {
    let vtop = format!("V{}", top);
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "#[repr(C)]")?;
    writeln!(writer, "pub struct V{} {{", top)?;
    writeln!(writer, "    _private: [u8; 0], // Opaque type for FFI")?;
    writeln!(writer, "}}\n")?;

    writeln!(writer, "#[repr(C)]")?;
    writeln!(writer, "pub struct VerilatedVcdC {{")?;
    writeln!(writer, "    _private: [u8; 0], // Opaque type for FFI")?;
    writeln!(writer, "}}\n")?;

    writeln!(writer, "extern \"C\" {{\n")?;

    writeln!(writer, "    pub fn {}_new() -> *mut {};", top, vtop)?;
    writeln!(writer, "    pub fn {}_eval(dut: *mut {});", top, vtop)?;
    writeln!(writer, "    pub fn {}_delete(dut: *mut {});", top, vtop)?;
    writeln!(writer, "    pub fn enable_trace(dut: *mut {}) -> *mut VerilatedVcdC;", vtop)?;
    writeln!(writer, "    pub fn close_trace(tfp: *mut VerilatedVcdC);")?;
    writeln!(writer, "    pub fn dump_vcd(tfp: *mut VerilatedVcdC, timestep: u32);")?;

    // Write the generated functions
    for signal in signals {
        assert!(signal.bits <= 64, "Signal {} width {} :(", signal.name, signal.bits);

        if signal.input {
            writeln!(writer, "    pub fn poke_{} (dut: *mut {}, {}: u64);", signal.name, vtop, signal.name)?;
        } else {
            writeln!(writer, "    pub fn peek_{} (dut: *mut {});", signal.name, vtop)?;
        }
    }

    writeln!(writer, "}} // extern \"C\"\n")?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // Unfortunately, there is no good way to pass command line arguments
    // to build.rs
    let args = Args {
        sv_file_path: "../emulator/Board.sv".to_string(),
        build_dir:      "build-dir".to_string()
    };

    let mut cwd = env::current_dir()?;
    cwd.push(args.build_dir.to_string());
    fs::create_dir_all(cwd.to_path_buf())?;

    Command::new("cp")
        .arg(&args.sv_file_path)
        .arg(cwd.to_str().unwrap())
        .status()?;

    Command::new("which")
        .arg("verilator")
        .status().expect("verilator is not in path");

    let sv_file_path = Path::new(&args.sv_file_path);
    let sv_file = sv_file_path.file_name().unwrap().to_str().unwrap();
    Command::new("verilator")
        .current_dir(&cwd)
        .arg("--cc").arg(sv_file)
        .arg("--build")
        .arg("-j").arg("32")
        .arg("-CFLAGS").arg("-fPIC")
        .arg("--trace")
        .status()?;

    let top = sv_file_path.file_stem().unwrap().to_str().unwrap();
    let obj_dir = format!("{}/obj_dir", cwd.to_str().unwrap());
    let vtop_h_path = format!("{}/V{}.h", obj_dir, top);

    // Parse the file and collect signals
    let signals = parse_file(&vtop_h_path)?;
    let ctop_c_path = format!("{}/C{}.c", obj_dir, top);

    // Generate C bindings
    generate_c_bindings(top, &signals, &ctop_c_path)?;

    let conda_prefix = env::var("CONDA_PREFIX").unwrap();
    let verilator_path = format!("{}/share/verilator/include", conda_prefix);

    // Compile C<top>.o
    Command::new("g++")
        .current_dir(&cwd)
        .arg("-I.")
        .arg("-MMD")
        .arg(&format!("-I{}", verilator_path))
        .arg(&format!("-I{}/vltstd", verilator_path))
        .arg("-DVM_COVERAGE=0")
        .arg("-DVM_SC=0")
        .arg("-DVM_TIMING=0")
        .arg("-DVM_TRACE=1")
        .arg("-DVM_TRACE_FST=0")
        .arg("-DVM_TRACE_VCD=1")
        .arg("-faligned-new")
        .arg("-fcf-protection=none")
        .arg("-Wno-bool-operation")
        .arg("-Wno-shadow")
        .arg("-Wno-sign-compare")
        .arg("-Wno-tautological-compare")
        .arg("-Wno-uninitialized")
        .arg("-Wno-unused-but-set-parameter")
        .arg("-Wno-unused-but-set-variable")
        .arg("-Wno-unused-parameter")
        .arg("-Wno-unused-variable")
        .arg("-fPIC")
        .arg("-Os")
        .arg("-c")
        .arg("-o")
        .arg(&format!("{}/C{}.o", obj_dir, top))
        .arg(&format!("{}/C{}.c", obj_dir, top))
        .status()?;

    // Compile libV<top>.so
    Command::new("g++")
        .current_dir(&cwd)
        .arg("-shared")
        .arg("-o")
        .arg("../libVdut.so")
        .arg(&format!("{}/verilated.o", obj_dir))
        .arg(&format!("{}/V{}.o", obj_dir, top))
        .arg(&format!("{}/verilated_threads.o", obj_dir))
        .arg(&format!("{}/verilated_vcd_c.o", obj_dir))
        .arg(&format!("{}/C{}.o", obj_dir, top))
        .arg(&format!("{}/V{}__ALL.a", obj_dir, top))
        .status()?;

    let rust_binding_path = format!("{}/src/dut.rs", env::current_dir()?.to_str().unwrap());
    generate_rust_bindings(top, &signals, &rust_binding_path)?;

    println!("cargo:rustc-link-search=native=./");
    println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}/lib,-rpath,{}", conda_prefix, "./");
    println!("cargo:rustc-link-lib=dylib=Vdut");

    return Ok(());
}
