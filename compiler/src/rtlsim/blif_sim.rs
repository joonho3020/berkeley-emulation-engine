use crate::common::circuit::*;
use crate::common::mapping::{SRAMMapping, SRAMPortType};
use crate::common::primitive::*;
use crate::rtlsim::rtlsim_utils::InputStimuliMap;
use crate::fsim::sram::{SRAMEntry, SRAMInputs};
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::VecDeque;
use petgraph::{
    Undirected,
    prelude::Dfs,
    visit::{VisitMap, Visitable},
    graph::NodeIndex,
    Direction::{Incoming, Outgoing},
};

#[derive(Debug, Default)]
pub struct SRAMState {
    pub cfg: SRAMMapping,
    pub mem: Vec<SRAMEntry>,
    pub input: SRAMInputs,
    pub rddata: SRAMEntry
}

impl SRAMState {
    pub fn new(cfg: &SRAMMapping) -> Self {
        SRAMState {
            cfg: cfg.clone(),
            mem: vec![SRAMEntry::new(cfg.width_bits); 1024], // FIXME
            input: SRAMInputs::new(cfg.width_bits),
            rddata: SRAMEntry::new(cfg.width_bits)
        }
    }

    pub fn update_cycle(self: &mut Self) {
        let (ren, wen, waddr) = match self.cfg.port_type {
            SRAMPortType::OneRdOneWrPortSRAM => {
                (self.input.rd_en != 0,
                 self.input.wr_en != 0,
                 self.input.wr_addr)
            }
            SRAMPortType::SinglePortSRAM => {
                (self.input.wr_en == 0 && self.input.rd_en != 0,
                 self.input.wr_en != 0 && self.input.rd_en != 0,
                 self.input.rd_addr)
            }
        };

        // Write to SRAM
        if wen {
            let wdata = if self.cfg.wmask_bits == 0 {
                // No mask, just write the entire input wr_data
                self.input.wr_data.clone()
            } else {
                // Read the current data
                let cur_data = self.mem.get(waddr as usize).unwrap();

                // Compute mask
                let num_bits_per_mask = self.cfg.width_bits / self.cfg.wmask_bits;
                let mut mask = vec![0u8; self.cfg.width_bits as usize];
                for i in 0..self.cfg.wmask_bits {
                    let mask_value = self.input.wr_mask.get(i as usize).unwrap();
                    for j in 0..num_bits_per_mask {
                        let idx = i * num_bits_per_mask + j;
                        *mask.get_mut(idx as usize).unwrap() = *mask_value;
                    }
                }

                assert!(mask.len() == self.input.wr_data.len(),
                    "mask {:?}, expected length: {}, num_masks: {} num_bits_per_mask: {}",
                    mask, self.input.wr_data.len(), self.cfg.wmask_bits, num_bits_per_mask);

                // Compute masked written value
                let mut ret = vec![];
                for ((m, w), r) in mask.iter().zip(self.input.wr_data.iter()).zip(cur_data.bits.iter()) {
                    if *m == 0 {
                        ret.push(*r);
                    } else {
                        ret.push(*w);
                    }
                }
                ret
            };

            // Perform the write
            self.mem[waddr as usize] = SRAMEntry { bits:  wdata };
        }

        // Read to SRAM
        if ren {
            self.rddata = self.mem.get(self.input.rd_addr as usize).unwrap().clone();
        }
    }
}

#[derive(Debug, Default)]
pub struct BlifSimulator {
    pub circuit: Circuit,
    pub input_stimulti_blasted: InputStimuliMap,
    srams: IndexMap<u32, SRAMState>,
    topo_sort_order: Vec<NodeIndex>,
    cur_cycle: u32
}

impl BlifSimulator {
    pub fn new(circuit: Circuit, input_stimulti_blasted: InputStimuliMap) -> Self {
        let mut topo_sort_order = vec![];

        // compute indeg for the entire graph
        let mut indeg: IndexMap<NodeIndex, u32> = IndexMap::new();
        for nidx in circuit.graph.node_indices() {
            indeg.insert(nidx, 0);
        }
        for eidx in circuit.graph.edge_indices() {
            let e = circuit.graph.edge_endpoints(eidx).unwrap();
            let dst = e.1;
            *indeg.get_mut(&dst).unwrap() += 1;
        }

        let undir_graph = circuit.graph.clone().into_edge_type::<Undirected>();
        let mut vis_map = circuit.graph.visit_map();
        for curidx in circuit.graph.node_indices() {
            if vis_map.is_visited(&curidx) {
                continue;
            }

            // Found new connected component
            // DFS to search for all the relevant nodes
            let mut ff_nodes: Vec<NodeIndex> = vec![];
            let mut in_nodes: Vec<NodeIndex> = vec![];
            let mut sr_nodes: Vec<NodeIndex> = vec![];

            let mut dfs = Dfs::new(&undir_graph, curidx);
            while let Some(nx) = dfs.next(&undir_graph) {
                vis_map.visit(nx);

                let node = circuit.graph.node_weight(nx).unwrap();
                match node.is() {
                    Primitive::Latch => {
                        ff_nodes.push(nx);
                    }
                    Primitive::Gate => {
                        ff_nodes.push(nx);
                    }
                    Primitive::Input => {
                        in_nodes.push(nx);
                    }
                    Primitive::ConstLut => {
                        in_nodes.push(nx);
                    }
                    Primitive::SRAMRdData => {
                        sr_nodes.push(nx);
                    }
                    _ => {
                    }
                }
            }

            // Start topological sort
            let mut q: VecDeque<NodeIndex> = VecDeque::new();
            for nidx in ff_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in in_nodes.iter() {
                q.push_back(*nidx);
            }
            for nidx in sr_nodes.iter() {
                q.push_back(*nidx);
            }

            // BFS
            let mut topo_vis_map = circuit.graph.visit_map();
            while !q.is_empty() {
                let nidx = q.pop_front().unwrap();
                if topo_vis_map.is_visited(&nidx) {
                    continue;
                }

                topo_vis_map.visit(nidx);
                topo_sort_order.push(nidx);

                let childs = circuit.graph.neighbors_directed(nidx, Outgoing);
                for cidx in childs {
                    let cnode = circuit.graph.node_weight(cidx).unwrap();
                    if !topo_vis_map.is_visited(&cidx)    &&
                        cnode.is() != Primitive::Gate     &&
                        cnode.is() != Primitive::Latch    &&
                        cnode.is() != Primitive::Input    &&
                        cnode.is() != Primitive::ConstLut &&
                        cnode.is() != Primitive::SRAMRdData {
                        *indeg.get_mut(&cidx).unwrap() -= 1;
                        if *indeg.get(&cidx).unwrap() == 0 {
                            q.push_back(cidx);
                        }
                    }
                }
            }
        }

        // Assume that we have one SRAM per module for now
        let mut srams: IndexMap<u32, SRAMState> = IndexMap::new();
        for (m, mmap) in circuit.emul.module_mappings.iter() {
            let smap = &mmap.sram_mapping;
            if smap.width_bits == 0 {
                continue;
            }
            srams.insert(*m, SRAMState::new(&smap));

        }

        return BlifSimulator {
            circuit: circuit,
            input_stimulti_blasted: input_stimulti_blasted,
            topo_sort_order: topo_sort_order,
            srams: srams,
            cur_cycle: 0
        };
    }

    pub fn run_cycle(self: &mut Self) {
        for nidx in self.topo_sort_order.iter() {
            let parents = self.circuit.graph.neighbors_directed(*nidx, Incoming);
            let node = self.circuit.graph.node_weight(*nidx).unwrap();
            let module = node.info().coord.module;
            let prim = node.prim.clone();

            let mut node_value = 0;
            match prim {
                CircuitPrimitive::Gate { .. } => {
                    assert!(false, "Should find no gates here");
                }
                CircuitPrimitive::ConstLut { val, .. } => {
                    node_value = val;
                }
                CircuitPrimitive::Lut { inputs, output:_, table } => {
                    let mut pvs = vec![];
                    for pidx in parents {
                        let pnode = self.circuit.graph.node_weight(pidx).unwrap();
                        let pval = pnode.info().debug.val;
                        let idx = inputs.iter().position(|n| n == pnode.name()).unwrap();
                        pvs.push((idx, pval));
                    }
                    pvs.sort_by(|a, b| a.0.cmp(&b.0));
                    let ivs = pvs.iter().map(|x| x.1).collect_vec();
                    node_value = if table.contains(&ivs) { 1 } else { 0 };
                }
                CircuitPrimitive::Latch { input:_, output:_, control:_, init:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;
                }
                CircuitPrimitive::Input { name } => {
                    node_value = *self.input_stimulti_blasted
                        .get(&name).unwrap()
                        .get(self.cur_cycle as usize).unwrap() as Bit;
                }
                CircuitPrimitive::Output { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;
                }
                CircuitPrimitive::SRAMRdEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_en(node_value);
                }
                CircuitPrimitive::SRAMWrEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_en(node_value);
                }
                CircuitPrimitive::SRAMRdAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMWrAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMWrMask { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_mask(node_value, idx);
                }
                CircuitPrimitive::SRAMWrData { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_data(node_value, idx);
                }
                CircuitPrimitive::SRAMRdWrEn { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_en(node_value);
                }
                CircuitPrimitive::SRAMRdWrMode { name:_ } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_wr_en(node_value);
                }
                CircuitPrimitive::SRAMRdWrAddr { name:_, idx } => {
                    node_value = self.circuit.graph
                        .node_weight(parents.last().unwrap()).unwrap()
                        .info().debug.val;

                    self.srams.get_mut(&module).unwrap()
                        .input.set_rd_addr(node_value, idx);
                }
                CircuitPrimitive::SRAMRdData { name:_, idx } => {
                    node_value = self.srams.get(&module).unwrap().rddata.bit(idx);
                }
                _ => {
                }
            }
            self.circuit.graph
                .node_weight_mut(*nidx).unwrap()
                .info_mut().debug.val = node_value;
        }

        for (_, s) in self.srams.iter_mut() {
            s.update_cycle();
        }

        self.cur_cycle += 1;
    }
}

#[cfg(test)]
pub mod blif_sim_test {
    use super::*;
    use crate::common::config::*;
    use crate::common::utils::save_graph_pdf;
    use crate::passes::runner;
    use crate::passes::blif_to_circuit::blif_to_circuit;
    use crate::fsim::board::*;
    use crate::rtlsim::rtlsim_utils::*;
    use std::env;
    use std::fs;
    use std::cmp::max;
    use std::process::Command;
    use indicatif::ProgressBar;
    use test_case::test_case;

    fn compare_blif_sim_to_fsim(args: Args) -> std::io::Result<()> {
        let sim_dir = format!("blif-sim-dir-{}", args.top_mod);
        let mut cwd = env::current_dir()?;
        cwd.push(sim_dir.clone());
        Command::new("mkdir").arg(&cwd).status()?;

        println!("Parsing blif file");
        let res = blif_to_circuit(&args.blif_file_path);
        let mut circuit = match res {
            Ok(c) => c,
            Err(e) => {
                return Err(std::io::Error::other(format!("{}", e)));
            }
        };

        circuit.set_cfg(
            PlatformConfig {
                num_mods:          args.num_mods,
                num_procs:         args.num_procs,
                max_steps:         args.max_steps,
                lut_inputs:        args.lut_inputs,
                inter_proc_nw_lat: args.inter_proc_nw_lat,
                inter_mod_nw_lat:  args.inter_mod_nw_lat,
                imem_lat:          args.imem_lat,
                dmem_rd_lat:       args.dmem_rd_lat,
                dmem_wr_lat:       args.dmem_wr_lat,
                sram_width:        args.sram_width,
                sram_entries:      args.sram_entries,
                sram_rd_ports:     args.sram_rd_ports,
                sram_wr_ports:     args.sram_wr_ports,
                sram_rd_lat:       args.sram_rd_lat,
                sram_wr_lat:       args.sram_wr_lat,
                topology: GlobalNetworkTopology::new(args.num_mods, args.num_procs)
            },
            CompilerConfig {
                top_module: args.top_mod.clone(),
                output_dir: cwd.to_str().unwrap().to_string(),
                dbg_tail_length: args.dbg_tail_length,
                dbg_tail_threshold: args.dbg_tail_threshold,
            }
        );

        println!("Running compiler passes with config: {:#?}", &circuit.platform_cfg);
        runner::run_compiler_passes(&mut circuit);
        println!("Compiler pass finished");

        let verilog_str = match fs::read_to_string(&args.sv_file_path) {
            Ok(content) => content,
            Err(e) => {
                return Err(std::io::Error::other(format!(
                    "Error while parsing:\n{}",
                    e
                )));
            }
        };

        // convert input stimuli to bit-blasted input stimuli
        let ports = get_io(verilog_str.to_string(), args.top_mod.to_string());
        let input_stimuli = get_input_stimuli(&args.input_stimuli_path);
        let input_stimuli_blasted = bitblast_input_stimuli(&input_stimuli, &ports);

        let mut board = Board::from(&circuit);
        let mut bsim  = BlifSimulator::new(circuit.clone(), input_stimuli_blasted.clone());

        let cycles = input_stimuli_blasted.values().fold(0, |x, y| max(x, y.len()));
        assert!(cycles > 1, "No point in running {}", cycles);

        let bar = ProgressBar::new(cycles as u64);
        for cycle in 0..(cycles-1) {
            bar.inc(1);

            // Collect input stimuli for the current cycle by name
            let mut input_stimuli_by_name: IndexMap<String, Bit> = IndexMap::new();
            for key in input_stimuli_blasted.keys() {
                let val = input_stimuli_blasted[key].get(cycle);
                match val {
                    Some(b) => input_stimuli_by_name.insert(key.to_string(), *b as Bit),
                    None => None
                };
            }

            // Find the step at which the input has to be poked
            // Save that in the input_stimuli_by_step
            let mut input_stimuli_by_step: IndexMap<u32, Vec<(&str, Bit)>> = IndexMap::new();
            for (sig, bit) in input_stimuli_by_name.iter() {
                match board.nodeindex(sig) {
                    Some(nidx) => {
                        let pc = circuit.graph.node_weight(nidx).unwrap().info().pc;
                        let step = pc + circuit.platform_cfg.pc_ldm_offset();
                        if input_stimuli_by_step.get(&step) == None {
                            input_stimuli_by_step.insert(step, vec![]);
                        }
                        input_stimuli_by_step.get_mut(&step).unwrap().push((sig, *bit));
                    }
                    None => {
                    }
                }
            }

            // Run emulator & blif simulator
            board.run_cycle(&input_stimuli_by_step);
            bsim.run_cycle();

            let mut found_mismatch = false;
            for nidx in bsim.circuit.graph.node_indices() {
                let node = bsim.circuit.graph.node_weight(nidx).unwrap();
                let bsim_val = node.info().debug.val;
                let opt_emul_val = board.peek(node.name());
                match opt_emul_val {
                    Some(emul_val) => {
                        if bsim_val != emul_val {
                            found_mismatch = true;

                            println!("node: {:?} blif sim val {} emul sim val {}",
                                circuit.graph.node_weight(nidx).unwrap(),
                                bsim_val,
                                emul_val);

                            save_graph_pdf(
                                &circuit.debug_graph_2(nidx, &board),
                                &format!("{}/after-cycle-{}-signal-{}.dot",
                                    cwd.to_str().unwrap(), cycle, node.name()),
                                &format!("{}/after-cycle-{}-signal-{}.pdf",
                                    cwd.to_str().unwrap(), cycle, node.name()))?;
                        }
                    }
                    None => {
                    }
                }
            }

            if found_mismatch {
                return Err(std::io::Error::other(format!("Simulation mismatch")));
            }
        }
        bar.finish();

        return Ok(());
    }

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

    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_adder(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Adder.sv",
                "Adder",
                "../examples/Adder.input",
                "../examples/Adder.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(5, 4, 0, 0, 1, 0; "mod 5 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_testreginit(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/TestRegInit.sv",
                "TestRegInit",
                "../examples/TestRegInit.input",
                "../examples/TestRegInit.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_const(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Const.sv",
                "Const",
                "../examples/Const.input",
                "../examples/Const.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_gcd(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/GCD.sv",
                "GCD",
                "../examples/GCD.input",
                "../examples/GCD.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_fir(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/Fir.sv",
                "Fir",
                "../examples/Fir.input",
                "../examples/Fir.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(9, 8, 0, 0, 1, 0; "mod 9 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_myqueue(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/MyQueue.sv",
                "MyQueue",
                "../examples/MyQueue.input",
                "../examples/MyQueue.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_1r1w_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/OneReadOneWritePortSRAM.sv",
                "OneReadOneWritePortSRAM",
                "../examples/OneReadOneWritePortSRAM.input",
                "../examples/OneReadOneWritePortSRAM.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 8, 0, 0, 1, 0; "mod 2 procs 8 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_1rw_sram(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/SinglePortSRAM.sv",
                "SinglePortSRAM",
                "../examples/SinglePortSRAM.input",
                "../examples/SinglePortSRAM.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }

    #[test_case(2, 4, 0, 0, 1, 0; "mod 2 procs 4 imem 0 dmem rd 0 wr 1 network 0")]
    pub fn test_pointer_chasing(num_mods: u32, num_procs: u32, imem_lat: u32, dmem_rd_lat: u32, dmem_wr_lat: u32, network_lat: u32) {
        assert_eq!(
            test_blif_sim(
                "../examples/PointerChasing.sv",
                "PointerChasing",
                "../examples/PointerChasing.input",
                "../examples/PointerChasing.lut.blif",
                num_mods, num_procs,
                network_lat, network_lat, imem_lat, dmem_rd_lat, dmem_wr_lat
            ),
            true
        );
    }
}
