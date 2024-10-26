pub mod dut;
use dut::*;

unsafe fn step(dut: *mut VBoard, vcd: *mut VerilatedVcdC, cycle: &mut u32) {
    let time = *cycle * 2;
    poke_clock(dut, 1);
    Board_eval(dut);
    dump_vcd(vcd, time);

    poke_clock(dut, 0);
    Board_eval(dut);
    dump_vcd(vcd, time + 1);
    *cycle += 1;
}

fn main() {
    unsafe {
        let dut = Board_new();
        if dut.is_null() {
            panic!("Failed to create dut instance");
        }
        let vcd = enable_trace(dut);
        poke_reset(dut, 1);
        poke_clock(dut, 0);
        Board_eval(dut);
        poke_reset(dut, 0);

        let mut cycle = 0;

        // Do nothing for 10 cycles
        for _ in 0..10 {
            step(dut, vcd, &mut cycle);
            println!("cycle: {}", cycle);
        }

        poke_io_cfg_in_0_host_steps(dut, 100);
        poke_io_cfg_in_1_host_steps(dut, 100);
        poke_io_cfg_in_2_host_steps(dut, 100);

        step(dut, vcd, &mut cycle);


        close_trace(vcd);
        Board_delete(dut);
    }
    println!("Test finished");
}
