use std::env;
use std::fs;

pub struct Args {
    pub sim_mmap_file_path: String,
}

fn generate_driver_impl(mmap_path: String) -> std::io::Result<()> {
    let macro_str = std::fs::read_to_string(mmap_path)?;

    let dst_path = format!("{}/src/simif/driver_generated.rs", env::current_dir()?.to_str().unwrap());
    let macro_code = format!(r#"
    use crate::simif::simif::*;
    use crate::simif::mmioif::*;
    use crate::simif::dmaif::*;

    {}
    "#, macro_str);

    // Write the macro code to the file.
    fs::write(&dst_path, macro_code).unwrap();
    return Ok(());
}

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // Unfortunately, there is no good way to pass command line arguments
    // to build.rs
    let args = Args {
        sim_mmap_file_path: "./build-dir/FPGATop.mmap".to_string(),
    };

    generate_driver_impl(args.sim_mmap_file_path)?;

    return Ok(());
}
