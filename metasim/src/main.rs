pub mod dut;
use dut::*;

fn main() {
    unsafe {
        let dut = Board_new();
    }
    println!("Hello world");
}
