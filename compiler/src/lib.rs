pub mod common;
pub mod fsim;
pub mod passes;
pub mod rtlsim;
pub mod testing;
pub mod simif;

#[cfg(test)]
pub mod blif_sim_test {
    use crate::common::config::*;
    use crate::testing::blifsim::compare_blif_sim_to_fsim;
    use test_case::test_case;

    fn test_blif_sim(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
        num_mods: u32,
        num_procs: u32,
        inter_proc_nw_lat: u32,
        inter_mod_nw_lat: u32,
        imem_lat: u32,
        dmem_rd_lat: u32,
        dmem_wr_lat: u32,
    ) -> bool {
        let args = Args {
            verbose:            false,
            sim_dir:            format!("blif-sim-dir-{}", top_mod),
            sv_file_path:       sv_file_path.to_string(),
            top_mod:            top_mod.to_string(),
            input_stimuli_path: input_stimuli_path.to_string(),
            blif_file_path:     blif_file_path.to_string(),
            vcd:                None,
            instance_path:      "testharness.top".to_string(),
            clock_start_low:    false,
            timesteps_per_cycle: 2,
            ref_skip_cycles:    4,
            no_check_cycles:    0,
            check_cycle_period: 1,
            num_mods:           num_mods,
            num_procs:          num_procs,
            max_steps:          65536,
            lut_inputs:         3,
            inter_proc_nw_lat:  inter_proc_nw_lat,
            inter_mod_nw_lat:   inter_mod_nw_lat,
            imem_lat:           imem_lat,
            dmem_rd_lat:        dmem_rd_lat,
            dmem_wr_lat:        dmem_wr_lat,
            sram_width:         128,
            sram_entries:       1024,
            sram_rd_ports:      1,
            sram_wr_ports:      1,
            sram_rd_lat:        1,
            sram_wr_lat:        1,
            dbg_tail_length:    u32::MAX, // don't print debug graph when testing
            dbg_tail_threshold: u32::MAX  // don't print debug graph when testing
        };
        match compare_blif_sim_to_fsim(args) {
            Ok(_)  => { return true;  }
            Err(_) => { return false; }
        }
    }

    #[test_case(5, 4, 0, 0, 1, 0, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 4, 1, 0, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    pub fn test_adder(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Adder.sv",
                "Adder",
                "../examples/Adder.input",
                "../examples/Adder.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 4, 0, 0, 1, 0, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 4, 1, 0, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 4, 1, 1, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_reginit(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/TestRegInit.sv",
                "TestRegInit",
                "../examples/TestRegInit.input",
                "../examples/TestRegInit.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_const(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Const.sv",
                "Const",
                "../examples/Const.input",
                "../examples/Const.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_gcd(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/GCD.sv",
                "GCD",
                "../examples/GCD.input",
                "../examples/GCD.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_shiftreg(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/ShiftReg.sv",
                "ShiftReg",
                "../examples/ShiftReg.input",
                "../examples/ShiftReg.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_fir(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Fir.sv",
                "Fir",
                "../examples/Fir.input",
                "../examples/Fir.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_myqueue(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/MyQueue.sv",
                "MyQueue",
                "../examples/MyQueue.input",
                "../examples/MyQueue.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_1r1w_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/OneReadOneWritePortSRAM.sv",
                "OneReadOneWritePortSRAM",
                "../examples/OneReadOneWritePortSRAM.input",
                "../examples/OneReadOneWritePortSRAM.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_1rw_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/SinglePortSRAM.sv",
                "SinglePortSRAM",
                "../examples/SinglePortSRAM.input",
                "../examples/SinglePortSRAM.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    pub fn test_pointer_chasing(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/PointerChasing.sv",
                "PointerChasing",
                "../examples/PointerChasing.input",
                "../examples/PointerChasing.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }
}

#[cfg(test)]
pub mod emulation_tester {
    use test_case::test_case;
    use crate::common::config::*;
    use crate::testing::fsim::*;

    fn perform_test(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
        num_mods: u32,
        num_procs: u32,
        inter_proc_nw_lat: u32,
        inter_mod_nw_lat: u32,
        imem_lat: u32,
        dmem_rd_lat: u32,
        dmem_wr_lat: u32,
    ) -> bool {
        let ret = test_emulator(Args {
            verbose:            false,
            sim_dir:            format!("sim-dir-{}", top_mod),
            sv_file_path:       sv_file_path.to_string(),
            top_mod:            top_mod.to_string(),
            input_stimuli_path: input_stimuli_path.to_string(),
            blif_file_path:     blif_file_path.to_string(),
            vcd:                None,
            instance_path:      "testharness.top".to_string(),
            clock_start_low:    false,
            timesteps_per_cycle: 2,
            ref_skip_cycles:    4,
            no_check_cycles:    0,
            check_cycle_period: 1,
            num_mods:           num_mods,
            num_procs:          num_procs,
            max_steps:          65536,
            lut_inputs:         3,
            inter_proc_nw_lat:  inter_proc_nw_lat,
            inter_mod_nw_lat:   inter_mod_nw_lat,
            imem_lat:           imem_lat,
            dmem_rd_lat:        dmem_rd_lat,
            dmem_wr_lat:        dmem_wr_lat,
            sram_width:         128,
            sram_entries:       1024,
            sram_rd_ports:      1,
            sram_wr_ports:      1,
            sram_rd_lat:        1,
            sram_wr_lat:        1,
            dbg_tail_length:    u32::MAX, // don't print debug graph when testing
            dbg_tail_threshold: u32::MAX  // don't print debug graph when testing
        });
        match ret {
            Ok(rc) => return rc == ReturnCode::TestSuccess,
            _      => return false
        }
    }

    #[test_case(5, 4, 0, 0, 1, 0, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 4, 1, 0, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 4, 1, 1, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 4, 1, 1, 1, 1, 1; "mod 5 procs 4 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_adder(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Adder.sv",
                "Adder",
                "../examples/Adder.input",
                "../examples/Adder.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 4, 0, 0, 1, 0, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 4, 1, 0, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 4, 1, 1, 1, 0, 1; "mod 5 procs 4 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 4, 1, 1, 1, 1, 1; "mod 5 procs 4 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_testreginit(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/TestRegInit.sv",
                "TestRegInit",
                "../examples/TestRegInit.input",
                "../examples/TestRegInit.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 1, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_const(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Const.sv",
                "Const",
                "../examples/Const.input",
                "../examples/Const.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 4, 0, 0, 1, 0, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(2, 4, 1, 0, 1, 0, 1; "mod 2 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(2, 4, 1, 1, 1, 0, 1; "mod 2 procs 4 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(2, 4, 1, 1, 1, 1, 1; "mod 2 procs 4 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_counter(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Counter.sv",
                "Counter",
                "../examples/Counter.input",
                "../examples/Counter.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 1, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_shiftreg(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/ShiftReg.sv",
                "ShiftReg",
                "../examples/ShiftReg.input",
                "../examples/ShiftReg.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 1, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_gcd(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/GCD.sv",
                "GCD",
                "../examples/GCD.input",
                "../examples/GCD.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 1, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_fir(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Fir.sv",
                "Fir",
                "../examples/Fir.input",
                "../examples/Fir.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 1, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_myqueue(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/MyQueue.sv",
                "MyQueue",
                "../examples/MyQueue.input",
                "../examples/MyQueue.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(9, 8, 1, 0, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 0, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(9, 8, 1, 1, 1, 1, 1; "mod 9 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_core(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/Core.sv",
                "Core",
                "../examples/Core.input",
                "../examples/Core.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 1, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_1r1w_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/OneReadOneWritePortSRAM.sv",
                "OneReadOneWritePortSRAM",
                "../examples/OneReadOneWritePortSRAM.input",
                "../examples/OneReadOneWritePortSRAM.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 8, 0, 0, 1, 0, 0; "mod 5 procs 8 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(5, 8, 1, 0, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 0, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(5, 8, 1, 1, 1, 1, 1; "mod 5 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_1rw_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/SinglePortSRAM.sv",
                "SinglePortSRAM",
                "../examples/SinglePortSRAM.input",
                "../examples/SinglePortSRAM.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 4, 0, 0, 1, 0, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 nw proc 0 nw mod 0")]
    #[test_case(2, 4, 1, 0, 1, 0, 1; "mod 2 procs 4 imem 1 dmem rd 0 wr 1 nw proc 0 nw mod 1")]
    #[test_case(2, 4, 1, 1, 1, 0, 1; "mod 2 procs 4 imem 1 dmem rd 1 wr 1 nw proc 0 nw mod 1")]
    #[test_case(2, 8, 1, 1, 1, 1, 1; "mod 2 procs 8 imem 1 dmem rd 1 wr 1 nw proc 1 nw mod 1")]
    pub fn test_pointer_chasing(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, inter_proc_nw_lat: u32, inter_mod_nw_lat: u32) {
        assert_eq!(
            perform_test(
                "../examples/PointerChasing.sv",
                "PointerChasing",
                "../examples/PointerChasing.input",
                "../examples/PointerChasing.lut.blif",
                num_mods, num_procs,
                inter_proc_nw_lat, inter_mod_nw_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }
}
