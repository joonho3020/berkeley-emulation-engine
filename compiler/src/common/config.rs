use crate::common::primitive::*;
use crate::common::network::*;
use strum::EnumCount;
use serde::Serialize;
use derivative::Derivative;
use std::fmt::Debug;
use clap::Parser;
use indexmap::IndexMap;

#[derive(Clone, Default, Serialize)]
pub struct GlobalNetworkTopology {
    pub edges: IndexMap<Coordinate, Coordinate>,
    pub inter_mod_paths: IndexMap<(u32, u32), Vec<NetworkPath>>
}

impl GlobalNetworkTopology {
    pub fn new(num_mods: u32, num_procs: u32) -> Self {
        let mut ret = GlobalNetworkTopology::default();
        if num_mods == 1 {
            return ret;
        }
        let num_mods_1 = num_mods - 1;
        let grp_sz = num_procs / num_mods_1;

        assert!(num_mods_1 & (num_mods_1 - 1) == 0, "num_mods should be 2^n + 1");
        assert!(num_procs  & (num_procs - 1)  == 0, "num_procs should be 2^n + 1");
        assert!(num_procs >= num_mods_1, "num_procs {} < num_mods - 1 {}", num_procs, num_mods_1);

        for m in 0..num_mods_1 {
            for p in 0..num_procs {
                let r = p % grp_sz;
                let q = (p - r) / grp_sz;
                let src = Coordinate { module: m, proc: p };
                let dst = if q == m {
                    let dm = num_mods_1;
                    let dp = p;
                    Coordinate { module: dm, proc: dp }
                } else {
                    let dm = q;
                    let dp = m * grp_sz + r;
                    Coordinate { module: dm, proc: dp }
                };
                ret.edges.insert(src, dst);
                ret.edges.insert(dst, src);
                ret.add_path(src, dst);
                ret.add_path(dst, src);
            }
        }
        return ret;
    }

    fn add_path(self: &mut Self, src: Coordinate, dst: Coordinate) {
        if !self.inter_mod_paths.contains_key(&(src.module, dst.module)) {
            self.inter_mod_paths.insert((src.module, dst.module), vec![]);
        }
        if !self.inter_mod_paths.contains_key(&(dst.module, src.module)) {
            self.inter_mod_paths.insert((dst.module, src.module), vec![]);
        }
        let paths = self.inter_mod_paths.get_mut(&(src.module, dst.module)).unwrap();
        paths.push(NetworkPath::new(src, dst));
    }

    /// Returns a Vec<NetworkPath> where the path connects some processor in
    /// src.module to some processor in dst.module
    pub fn inter_mod_paths(self: &Self, src: Coordinate, dst: Coordinate) -> Vec<NetworkPath> {
        let paths = self.inter_mod_paths.get(&(src.module, dst.module)).unwrap();
        return paths.to_vec();
    }

    /// Returns a Vec<NetworkRoute> where the route connects src.module to dst.module
    /// while hopping to one intermediate module
    pub fn inter_mod_routes(self: &Self, src: Coordinate, dst: Coordinate) -> Vec<NetworkRoute> {
        let mut ret: Vec<NetworkRoute> = vec![];
        let mut src_to_inter: IndexMap<u32, Vec<NetworkPath>> = IndexMap::new();
        let mut inter_to_dst: IndexMap<u32, Vec<NetworkPath>> = IndexMap::new();
        for ((m1, m2), paths) in self.inter_mod_paths.iter() {
            if *m1 == src.module && *m2 != dst.module {
                if !src_to_inter.contains_key(m2) {
                    src_to_inter.insert(*m2, vec![]);
                }
                src_to_inter.get_mut(m2).unwrap().append(&mut paths.clone());
            }
            if *m1 != src.module && *m2 == dst.module {
                if !inter_to_dst.contains_key(m2) {
                    inter_to_dst.insert(*m1, vec![]);
                }
                inter_to_dst.get_mut(m1).unwrap().append(&mut paths.clone());
            }
        }
        for imod in src_to_inter.keys() {
            for s2i_path in src_to_inter.get(imod).unwrap().iter() {
                for i2d_path in inter_to_dst.get(imod).unwrap().iter() {
                    let route = if s2i_path.dst == i2d_path.src {
                        NetworkRoute::from([*s2i_path, *i2d_path])
                    } else {
                        NetworkRoute::from([*s2i_path,
                                           NetworkPath::new(
                                               s2i_path.dst,
                                               i2d_path.src),
                                           *i2d_path])
                    };
                    ret.push(route);
                }
            }
        }
        return ret;
    }
}

impl Debug for GlobalNetworkTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent: &str = "    ";

        write!(f, "digraph {{\n")?;

        let mut map: IndexMap<Coordinate, u32> = IndexMap::new();

        for (i, (src, _)) in self.edges.iter().enumerate() {
            map.insert(*src, i as u32);

            write!(
                f,
                "{}{} [ label = \"{:?}\" ]\n",
                indent,
                i,
                src
            )?;
        }
        for (i, (_, dst)) in self.edges.iter().enumerate() {
            write!(
                f,
                "{}{} {} {} ",
                indent,
                i,
                "->",
                map.get(dst).unwrap()
            )?;
            writeln!(f, "[ ]")?;
        }

        write!(f, "}}")
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct KaMinParConfig {
    /// Random seed for partitioner
    pub seed: u64,

    /// Partitioner hyperparameter
    pub epsilon: f64,

    /// Number of threads to perform partitioning
    pub nthreads: u32,
}

impl Default for KaMinParConfig {
    fn default() -> Self {
        KaMinParConfig {
            seed: 123,
            epsilon: 0.03,
            nthreads: 16
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct CompilerConfig {
    /// Name of the top module
    pub top_module: String,

    /// Path to the output directory
    pub output_dir: String,

    /// Number of consecutive PCs that is identified as a scheduling tail
    pub dbg_tail_length: u32,

    /// Number of nodes scheduled per PC for that PC to be classified as a tail
    pub dbg_tail_threshold: u32
}

/// # Context
/// - Config of the underlying hardware emulation platform
#[derive(Clone, Serialize, Derivative)]
#[derivative(Debug)]
pub struct PlatformConfig {
    /// Num modules
    pub num_mods: u32,

    /// Number of processor in a module
    pub num_procs: u32,

    /// Maximum host steps that can be run
    pub max_steps: u32,

    /// Number of lut inputs
    pub lut_inputs: Cycle,

    /// Latency of the switch network between processors in the same module
    pub inter_proc_nw_lat: Cycle,

    /// Latency of the switch network between modules
    pub inter_mod_nw_lat: Cycle,

    /// Number of cycles to access i-mem
    pub imem_lat: Cycle,

    /// Number of cycles to read d-mem
    pub dmem_rd_lat: Cycle,

    /// Number of cycles to write d-mem
    pub dmem_wr_lat: Cycle,

    /// SRAM width in bits (per module)
    pub sram_width: u32,

    /// Number of SRAM entries (per module)
    pub sram_entries: u32,

    /// Number of SRAM read ports
    pub sram_rd_ports: u32,

    /// Number of SRAM write ports
    pub sram_wr_ports: u32,

    /// Latency of the SRAM read latency
    pub sram_rd_lat: u32,

    /// Latency of the SRAM write latency
    pub sram_wr_lat: u32,

    /// Global network topology
    #[derivative(Debug="ignore")]
    pub topology: GlobalNetworkTopology
}

impl Default for PlatformConfig {
    fn default() -> Self {
        PlatformConfig {
            num_mods: 1,
            num_procs: 64,
            max_steps: 128,
            lut_inputs: 3,
            inter_proc_nw_lat: 0,
            inter_mod_nw_lat: 0,
            imem_lat: 0,
            dmem_rd_lat: 0,
            dmem_wr_lat: 1,
            sram_width: 64,
            sram_entries: 1024,
            sram_rd_ports: 1,
            sram_wr_ports: 1,
            sram_rd_lat: 1,
            sram_wr_lat: 1,
            topology: GlobalNetworkTopology::default()
        }
    }
}

impl PlatformConfig {
    fn power_of_2(self: &Self, v: u32) -> bool {
        return v & (v - 1) == 0;
    }

    fn log2ceil(self: &Self, v: u32) -> u32 {
        let log2x = u32::BITS - v.leading_zeros();
        if self.power_of_2(v) {
            log2x - 1
        } else {
            log2x
        }
    }

    /// log2Ceil(self.max_steps)
    pub fn index_bits(self: &Self) -> u32 {
        self.log2ceil(self.max_steps)
    }

    /// log2Ceil(self.num_procs)
    pub fn switch_bits(self: &Self) -> u32 {
        self.log2ceil(self.num_procs)
    }

    /// log2Ceil(number of Opcode)
    pub fn opcode_bits(self: &Self) -> u32 {
        self.log2ceil(Opcode::COUNT as u32)
    }

    /// number of bits for the LUT
    pub fn lut_bits(self: &Self) -> u32 {
        1 << self.lut_inputs
    }

    pub fn total_procs(self: &Self) -> u32 {
        self.num_mods * self.num_procs
    }

    /// - I can start using bits computed from a local processor at
    /// `local.pc + intra_proc_dep_lat`
    ///   <me> | read imem | read dmem | compute + write dmem |
    ///   <me>                                    | read imem | read dmem | compute
// pub fn intra_proc_dep_lat(self: &Self) -> Cycle {
// self.dmem_rd_lat + self.dmem_wr_lat
// }

    /// - I can start using bits computed from a remote processor at
    /// `remote.pc + inter_proc_dep_lat`.
    ///   <other> | read imem | read dmem | lut + network | write dmem |
    ///   <me>                                             | read imem | read dmem | compute |
// pub fn inter_proc_dep_lat(self: &Self) -> Cycle {
// self.dmem_rd_lat + self.inter_proc_nw_lat + self.dmem_wr_lat
// }

    /// - I have to receive a incoming bit from a remote processor at
    /// `remote.pc + remote_sin_lat`
    /// <other> | read imem | read dmem | compute | network |
    /// <me>                        | read imem | read dmem | compute + write sdm |
    pub fn remote_sin_lat(self: &Self) -> Cycle {
        self.inter_proc_nw_lat
    }

    /// If the current pc is `X`, store the current local compute result in
    /// `X - pc_ldm_offset`
    pub fn pc_ldm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat
    }

    /// If the current pc is `X`, store the current switch compute result in
    /// `X - pc_sdm_offset`
    pub fn pc_sdm_offset(self: &Self) -> Cycle {
        self.imem_lat + self.dmem_rd_lat + self.inter_proc_nw_lat
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_path_lat(self: &Self, path: &NetworkPath) -> u32 {
        match path.tpe {
            PathTypes::ProcessorInternal => 0,
            PathTypes::InterProcessor    => self.inter_proc_nw_lat,
            PathTypes::InterModule       => self.inter_mod_nw_lat
        }
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_route_lat(self: &Self, route: &NetworkRoute) -> u32 {
        let mut latency = 0;
        for (hop, path) in route.iter().enumerate() {
            latency += self.nw_path_lat(path);
            if hop != route.len() - 1 {
                latency += self.dmem_wr_lat
            }
        }
        return latency;
    }

    // TODO: Add global network latency, fix these functions for proper abstraction
    pub fn nw_route_dep_lat(self: &Self, route: &NetworkRoute) -> u32 {
        return self.nw_route_lat(route) + self.dmem_wr_lat;
    }

    pub fn sram_rd_en_offset(self: &Self) -> u32 {
        0
    }

    pub fn sram_wr_en_offset(self: &Self) -> u32 {
       self.sram_rd_en_offset() + 1 
    }

    pub fn sram_rd_addr_offset(self: &Self) -> u32 {
        self.sram_wr_en_offset() + 1
    }

    pub fn sram_wr_addr_offset(self: &Self) -> u32 {
        self.sram_rd_addr_offset() + self.sram_entries
    }

    pub fn sram_wr_data_offset(self: &Self) -> u32 {
        self.sram_wr_addr_offset() + self.sram_entries
    }

    pub fn sram_wr_mask_offset(self: &Self) -> u32 {
        self.sram_wr_data_offset() + self.sram_width
    }

    pub fn sram_rdwr_en_offset(self: &Self) -> u32 {
        self.sram_wr_mask_offset() + self.sram_width
    }

    pub fn sram_rdwr_mode_offset(self: &Self) -> u32 {
        self.sram_rdwr_en_offset() + 1
    }

    pub fn sram_rdwr_addr_offset(self: &Self) -> u32 {
        self.sram_rdwr_mode_offset() + 1
    }

    pub fn sram_other_offset(self: &Self) -> u32 {
        self.sram_rdwr_mode_offset() + self.sram_entries
    }

    pub fn index_to_sram_input_type(self: &Self, idx: u32) -> (Primitive, u32) {
        if idx >= self.sram_other_offset() {
            assert!(false, "Unknown index to sram input type: {}", idx);
            (Primitive::NOP, 0)
        } else if idx >= self.sram_rdwr_addr_offset() {
            (Primitive::SRAMRdWrAddr, idx - self.sram_rdwr_addr_offset())
        } else if idx >= self.sram_rdwr_mode_offset() {
            (Primitive::SRAMRdWrMode, idx - self.sram_rdwr_mode_offset())
        } else if idx >= self.sram_rdwr_en_offset() {
            (Primitive::SRAMRdWrEn, idx - self.sram_rdwr_en_offset())
        } else if idx >= self.sram_wr_mask_offset() {
            (Primitive::SRAMWrMask, idx - self.sram_wr_mask_offset())
        } else if idx >= self.sram_wr_data_offset() {
            (Primitive::SRAMWrData, idx - self.sram_wr_data_offset())
        } else if idx >= self.sram_wr_addr_offset() {
            (Primitive::SRAMWrAddr, idx - self.sram_wr_addr_offset())
        } else if idx >= self.sram_rd_addr_offset() {
            (Primitive::SRAMRdAddr, idx - self.sram_rd_addr_offset())
        } else if idx >= self.sram_wr_en_offset() {
            (Primitive::SRAMWrEn, idx - self.sram_wr_en_offset())
        } else {
            (Primitive::SRAMRdEn, idx - self.sram_rd_en_offset())
        }
    }
}


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// SystemVerilog file path
    #[arg(short, long, default_value = "")]
    pub sv_file_path: String,

    /// Name of the top module
    #[arg(short, long, default_value = "")]
    pub top_mod: String,

    /// Input value file path
    #[arg(short, long, default_value = "")]
    pub input_stimuli_path: String,

    /// Blif file path
    #[arg(short, long, default_value = "")]
    pub blif_file_path: String,

    /// number of modules
    #[arg(long, default_value_t = 1)]
    pub num_mods: u32,

    /// number of processors in a module
    #[arg(long, default_value_t = 8)]
    pub num_procs: u32,

    /// maximum number of instructions per processor
    #[arg(long, default_value_t = 128)]
    pub max_steps: u32,

    /// lut inputs
    #[arg(long, default_value_t = 3)]
    pub lut_inputs: u32,

    /// network latency between procs in a module
    #[arg(long, default_value_t = 0)]
    pub inter_proc_nw_lat: u32,

    /// network latency between modules
    #[arg(long, default_value_t = 0)]
    pub inter_mod_nw_lat: u32,

    /// imem latency
    #[arg(long, default_value_t = 0)]
    pub imem_lat: u32,

    /// dmem rd latency
    #[arg(long, default_value_t = 0)]
    pub dmem_rd_lat: u32, 

    /// dmem wr latency
    #[arg(long, default_value_t = 1)]
    pub dmem_wr_lat: u32,

    /// SRAM width in bits (per module)
    #[arg(long, default_value_t = 128)]
    pub sram_width: u32,

    /// Number of SRAM entries (per module)
    #[arg(long, default_value_t = 1024)]
    pub sram_entries: u32,

    /// Number of SRAM read ports
    #[arg(long, default_value_t = 1)]
    pub sram_rd_ports: u32,

    /// Number of SRAM write ports
    #[arg(long, default_value_t = 1)]
    pub sram_wr_ports: u32,

    /// Latency of the SRAM read latency
    #[arg(long, default_value_t = 1)]
    pub sram_rd_lat: u32,

    /// Latency of the SRAM write latency
    #[arg(long, default_value_t = 1)]
    pub sram_wr_lat: u32,

    /// debug tail length
    #[arg(long, default_value_t = 10)]
    pub dbg_tail_length: u32,

    /// debug tail threshold
    #[arg(long, default_value_t = 5)]
    pub dbg_tail_threshold: u32, 
}
