use crate::fsim::module::*;
use crate::fsim::switch::*;
use crate::common::{hwgraph::*, config::*, network::*, circuit::Circuit, primitive::*};
use petgraph::graph::NodeIndex;
use indexmap::IndexMap;
use std::fmt::Debug;
use std::iter::Iterator;

/// Represents a group of emulation `Module`s connected together
pub struct Board {
    /// Global network
    global_switch: Switch,

    /// Describes the connectivity of the processors in the `global_switch`
    global_switch_edges: IndexMap<Coordinate, Coordinate>,

    /// Modules
    modules: Vec<Module>,

    /// Number of host cycles used to emulate one target cycle
    host_steps: u32,

    /// Signal mapping
    signal_map: IndexMap<String, NodeMapInfo>,

    /// Platform configuration
    pcfg: PlatformConfig
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module[\n  {:?}\n{:#?}\n]", self.global_switch, self.modules)
    }
}

impl Board {
    pub fn from(c: &Circuit) -> Self {
        let mut modules: IndexMap<u32, Module> = IndexMap::new();
        let mut signal_map: IndexMap<String, NodeMapInfo> = IndexMap::new();
        let pcfg = &c.platform_cfg;

        assert_eq!(c.emul.module_mappings.len() as u32, pcfg.num_mods);

        for (m, mmap) in c.emul.module_mappings.iter() {
            let mut module = Module::new(*m, pcfg, c.emul.host_steps);
            let mut mod_signal_map: IndexMap<String, NodeMapInfo> = IndexMap::new();

            for p in 0..pcfg.num_procs {
                let pmap = mmap.proc_mappings.get(&p).unwrap();

                for (pc, inst) in pmap.instructions.iter().enumerate() {
                    module.procs[p as usize].set_inst(inst.clone(), pc);
                }

                for (sig, nm) in pmap.signal_map.iter() {
                    signal_map.insert(sig.clone(), nm.clone());
                    mod_signal_map.insert(sig.clone(), nm.clone());
                }
            }
            module.signal_map = mod_signal_map;
            module.sram_proc.set_sram_mapping(&mmap.sram_mapping);
            modules.insert(*m, module);
        }

        modules.sort_keys();

        Board {
            global_switch: Switch::new(pcfg.total_procs(), pcfg.inter_mod_nw_lat),
            global_switch_edges: pcfg.topology.edges.clone(),
            modules: modules.into_values().collect(),
            host_steps: c.emul.host_steps,
            signal_map: signal_map,
            pcfg: pcfg.clone()
        }
    }

    pub fn print_sigmap(self: &Self) {
        println!("{:#?}", self.signal_map);
    }

    pub fn print(self: &Self) {
        for (_, module) in self.modules.iter().enumerate() {
            module.print();
        }
    }

    pub fn nodeindex(self: &Self, signal: &str) -> Option<NodeIndex> {
        match self.signal_map.get(signal) {
            Some(map) => Some(map.idx),
            None => None,
        }
    }

    pub fn peek(self: &Self, signal: &str) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => Some(self
                              .modules[map.info.coord.module as usize]
                              .procs[map.info.coord.proc as usize]
                              .ldm[map.info.pc as usize]),
            None => None,
        }
    }

    pub fn poke(self: &mut Self, signal: &str, val: Bit) -> Option<Bit> {
        match self.signal_map.get(signal) {
            Some(map) => {
                let inst = self.modules[map.info.coord.module as usize]
                               .procs[map.info.coord.proc as usize]
                               .imem[map.info.pc as usize].clone();
                if inst.opcode == Primitive::Input {
                    self.modules[map.info.coord.module as usize]
                        .procs[map.info.coord.proc as usize]
                        .set_io_i(val);
                    Some(val)
                } else {
                    println!("Signal {} to poke is not a Input", signal);
                    None
                }
            }
            None => {
                println!("Cannot find signal {} to poke", signal);
                None
            }
        }
    }

    fn set_global_switch_out(self: &mut Self) {
        for (m, module) in self.modules.iter_mut().enumerate() {
            for (p, proc) in module.procs.iter_mut().enumerate() {
                let u = Coordinate { module: m as u32, proc: p  as u32 };
                let send_port = u.id(&self.pcfg) as usize;
                self.global_switch.set_port_val(send_port, proc.get_global_switch_out());
            }
        }
    }

    fn set_global_switch_in(self: &mut Self) {
        for m in 0..self.pcfg.num_mods {
            for p in 0..self.pcfg.num_procs {
                let u = Coordinate { module: m as u32, proc: p  as u32 };
                match self.global_switch_edges.get(&u) {
                    Some(v) => {
                        let send_port = u.id(&self.pcfg) as usize;
                        let recv_proc = &mut self.modules[v.module as usize].procs[v.proc as usize];
                        recv_proc.set_global_switch_in(self.global_switch.get_port_val(send_port));
                    }
                    None => {}
                }
            }
        }
    }

    fn step(self: &mut Self) {
        for (_, module) in self.modules.iter_mut().enumerate() {
            module.compute();
        }

        // Shuffle local network bits
        for (_, module) in self.modules.iter_mut().enumerate() {
            module.set_local_switch_out();
        }

        for (_, module) in self.modules.iter_mut().enumerate() {
            module.set_local_switch_in();
        }

        // Shuffle global network bits
        self.set_global_switch_out();
        self.set_global_switch_in();

        // consume network inputs and update processor state
        for (_, module) in self.modules.iter_mut().enumerate() {
            module.update_dmem_and_pc();
        }

        // update switch network
        for (_, module) in self.modules.iter_mut().enumerate() {
            module.switch.run_cycle();
        }

        self.global_switch.run_cycle();
    }

    pub fn run_cycle(self: &mut Self, input_stimuli: &IndexMap<u32, Vec<(&str, Bit)>>) {
        for step in 0..self.host_steps {
            match input_stimuli.get(&(step as u32)) {
                Some(vec) => {
                    for (sig, bit) in vec.iter() {
                        self.poke(sig, *bit);
                    }
                }
                None => {}
            };
            self.step();
        }
    }

    pub fn run_cycle_verbose(self: &mut Self, input_stimuli: &IndexMap<u32, Vec<(&str, Bit)>>, cycle: &u32) {
        println!("==================== Running Cycle {} ======================", cycle);
        for step in 0..self.host_steps {
            match input_stimuli.get(&(step as u32)) {
                Some(vec) => {
                    for (sig, bit) in vec.iter() {
                        self.poke(sig, *bit);
                    }
                }
                None => {}
            };
            self.step();
            self.print();
            println!("------------ Step Finished {} --------------", step);
        }
    }
}
