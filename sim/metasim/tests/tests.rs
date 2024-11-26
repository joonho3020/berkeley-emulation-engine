

#[cfg(test)]
pub mod fpgatop_test {
    use fpgatopsim::start_test;
    use bee::common::config::Args;
    use test_case::test_case;

    fn test_emulator_rtl(
        sv_file_path: &str,
        top_mod: &str,
        input_stimuli_path: &str,
        blif_file_path: &str,
    ) -> bool {
        let args = Args {
            verbose:             false,
            sim_dir:             format!("../sim-dir/metasim-{}", top_mod),
            sv_file_path:        sv_file_path.to_string(),
            top_mod:             top_mod.to_string(),
            input_stimuli_path:  input_stimuli_path.to_string(),
            blif_file_path:      blif_file_path.to_string(),
            vcd:                 None,
            instance_path:       "testharness.top".to_string(),
            clock_start_low:     false,
            timesteps_per_cycle: 2,
            ref_skip_cycles:     4,
            no_check_cycles:     0,
            check_cycle_period:  1,
            num_mods:            17,
            num_procs:           64,
            max_steps:           1024,
            lut_inputs:          3,
            inter_proc_nw_lat:   1,
            inter_mod_nw_lat:    1,
            imem_lat:            1,
            dmem_rd_lat:         0,
            dmem_wr_lat:         1,
            sram_width:          256,
            sram_entries:        16384,
            sram_rd_ports:       1,
            sram_wr_ports:       1,
            sram_rd_lat:         1,
            sram_wr_lat:         1,
            sram_ip_pl:          1,
            large_sram_cnt:      0,
            large_sram_width:    256,
            large_sram_entries:  16384,
            dbg_tail_length:     u32::MAX, // don't print debug graph when testing
            dbg_tail_threshold:  u32::MAX  // don't print debug graph when testing
        };

        match start_test(&args) {
            Ok(_) => {
                println!("Test Success!");
                return true;
            }
            Err(emsg) => {
                println!("Test Failed {:?}", emsg);
                return false;
            }
        }
    }

// #[test_case("Core"; "Core")]
    #[test_case("Adder"; "Adder Test")]
    #[test_case("TestRegInit"; "TestRegInit Test")]
    #[test_case("Const"; "Const Test")]
    #[test_case("GCD"; "GCD Test")]
    #[test_case("ShiftReg"; "ShiftReg Test")]
    #[test_case("Fir"; "Fir Test")]
    #[test_case("MyQueue"; "MyQueue Test")]
    #[test_case("PointerChasing"; "PointerChasing Test")]
    #[test_case("SinglePortSRAM"; "SinglePortSRAM Test")]
    #[test_case("OneReadOneWritePortSRAM"; "OneReadOneWritePortSRAM Test")]
    pub fn test(top: &str) {
        assert_eq!(
            test_emulator_rtl(
                &format!("../../examples/{}.sv", top),
                &top,
                &format!("../../examples/{}.input", top),
                &format!("../../examples/{}.lut.blif", top)),
                true
        );
    }
}
