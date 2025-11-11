use crate::common::{
    circuit::Circuit,
    primitive::*,
    hwgraph::*,
    config::*,
};
use crate::passes::inst_schedule::SchedCandidate;
use fixedbitset::FixedBitSet;
use petgraph::{
    graph::NodeIndex,
    visit::EdgeRef,
    Direction::{Incoming, Outgoing}
};
use std::collections::BTreeSet;
use indexmap::{IndexSet, IndexMap};
use strum_macros::EnumCount as EnumCountMacro;


#[derive(Debug, Default, Clone)]
struct LUTGroupState {
    pub occupied: FixedBitSet
}

impl LUTGroupState {
    pub fn new(luts_per_group: u32) -> Self {
        Self {
            occupied: FixedBitSet::with_capacity(luts_per_group as usize)
        }
    }
}

#[derive(Debug, Default, Clone)]
struct MemTileState {
    pub lut_groups: Vec<LUTGroupState>,
    pub rd_port_busy: bool,
    pub wr_port_busy: bool,

}

impl MemTileState {
    pub fn new(lut_groups: u32, luts_per_group: u32) -> Self {
        Self {
            lut_groups: vec![LUTGroupState::new(luts_per_group); lut_groups as usize],
            rd_port_busy: false,
            wr_port_busy: false
        }
    }
}


#[derive(Clone, Debug, Default, PartialEq, EnumCountMacro)]
#[repr(u32)]
enum RegState {
    #[default]
    Empty,
    Address,
    LUTOutput
}

#[derive(Debug, Default, Clone)]
struct StreamState {
    pub regs: Vec<RegState>
}

impl StreamState {
    pub fn new(num_streams: u32) -> Self {
        Self {
            regs: vec![RegState::default(); num_streams as usize]
        }
    }
}

#[derive(Debug, Default, Clone)]
struct LPUStage {
    pub stream: StreamState,
    pub mem_tiles: Vec<MemTileState>,
}

impl LPUStage {
    pub fn new(pcfg: &PlatformConfig) -> Self {
        Self {
            stream: StreamState::new(pcfg.lpu_num_streams),
            mem_tiles: vec![
                MemTileState::new(
                    pcfg.lpu_lut_groups_per_memtile,
                    pcfg.lpu_luts_per_memtile_entry);
                pcfg.lpu_memtiles_per_stream_stage as usize
            ],
        }
    }
}

#[derive(Debug, Default, Clone)]
struct LPUState {
    pub stages: Vec<LPUStage>,
}

impl LPUState {
    pub fn new(pcfg: &PlatformConfig) -> Self {
        Self {
            stages: vec![
                LPUStage::new(pcfg);
                pcfg.lpu_stream_stages() as usize
            ]
        }
    }

    pub fn init_from_prev_cycle(prev: &Self, vxm_out: StreamState) -> Self {
        let mut new_state = Self::default();
        new_state.stages = Vec::with_capacity(prev.stages.len());

        for (stage_idx, prev_stage) in prev.stages.iter().enumerate() {
            let mut new_stage = LPUStage::default();

            // Initialize mem_tiles: reset port busy flags, copy lut_groups
            new_stage.mem_tiles = prev_stage.mem_tiles.iter().map(|prev_mem_tile| {
                MemTileState {
                    lut_groups: prev_mem_tile.lut_groups.clone(),
                    rd_port_busy: false,
                    wr_port_busy: false,
                }
            }).collect();

            // Initialize stream: first stage gets vxm_out, rest shift from previous stage
            if stage_idx == 0 {
                new_stage.stream = vxm_out.clone();
            } else {
                new_stage.stream = prev.stages[stage_idx - 1].stream.clone();
            }
            new_state.stages.push(new_stage);
        }
        new_state
    }
}



#[derive(Debug, Default, Clone)]
struct PerCycleLPUState {
    /// PC -> LPUState mapping
    state: IndexMap<u32, LPUState>
}

/// simplifying assumptions
/// - vxm can concat any two streams in any order, still takes the same amount of cycles
/// - location of the LUT within a memtile entry does not matter for concat (this actually is not
/// true because of how the LUT is encoded in reality)
pub fn schedule_instructions_lpu(circuit: &mut Circuit) {
// place_nodes(circuit);
    schedule_instructions_lpu_internal(circuit);
}

fn schedule_instructions_lpu_internal(circuit: &mut Circuit) {
    let mut cpn: IndexSet<NodeIndex> = IndexSet::new();
    for nidx in circuit.graph.node_indices() {
        let rank = &circuit.graph.node_weight(nidx).unwrap().info().rank;
        if rank.asap == rank.alap {
            cpn.insert(nidx);
        }
    }

    let pcfg = circuit.platform_cfg.clone();
    let max_rank = circuit.emul.max_rank;

    let mut pc_min = 0;
    let mut pc = 0;
    let mut per_cycle_lpu_state = PerCycleLPUState::default();
    per_cycle_lpu_state.state.insert(0, LPUState::new(&pcfg));

    // start from the highest rank
    for next_rank in 1..max_rank+1 {
        let mut must_schedule_candidates:        BTreeSet<SchedCandidate> = BTreeSet::new();
        let mut best_effort_schedule_candidates: BTreeSet<SchedCandidate> = BTreeSet::new();
        let mut extra_effort_schedule_candidates: BTreeSet<SchedCandidate> = BTreeSet::new();

        // Search for all the nodes to schedule in this round
        for nidx in circuit.graph.node_indices() {
            let odeg = circuit.graph.neighbors_directed(nidx, Outgoing).count() as u32;
            let node = circuit.graph.node_weight_mut(nidx).unwrap();
            let info = node.info_mut();
            if next_rank <= info.rank.alap && !info.scheduled {
                let mob = info.rank.alap - next_rank;
                info.rank = RankInfo { mob: mob, ..info.rank };
                if info.rank.asap <= next_rank && (cpn.contains(&nidx) || mob == 0) {
                    must_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                } else if info.rank.asap <= next_rank {
                    best_effort_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                } else {
                    extra_effort_schedule_candidates.insert(SchedCandidate::new(nidx, mob, odeg));
                }
            }
        }

        pc_min = pc;

        while !must_schedule_candidates.is_empty() {
            if pc == 0 {
                per_cycle_lpu_state.state.insert(pc, LPUState::new(&pcfg));
            } else {
                let prev_pc = pc - 1;
                per_cycle_lpu_state.state.insert(pc,
                    LPUState::init_from_prev_cycle(
                        per_cycle_lpu_state.state.get(&prev_pc).unwrap(),
                        StreamState::new(pcfg.lpu_num_streams)));
            }

            let lpu_state = per_cycle_lpu_state.state.get_mut(&pc).unwrap();
            let _scheduled = schedule_prev_rank(
                circuit,
                &mut must_schedule_candidates,
                lpu_state,
                pc);

            pc += 1;
        }
        for try_pc in pc_min..pc {
            let lpu_state = per_cycle_lpu_state.state.get_mut(&try_pc).unwrap();
            let _scheduled = schedule_prev_rank(
                circuit,
                &mut best_effort_schedule_candidates,
                lpu_state,
                try_pc);
        }
        for try_pc in pc_min..pc {
            let lpu_state = per_cycle_lpu_state.state.get_mut(&try_pc).unwrap();
            let _scheduled = schedule_prev_rank(
                circuit,
                &mut extra_effort_schedule_candidates,
                lpu_state,
                try_pc);
        }
    }
}


/// Get the stream stage index for a given memtile
fn get_stream_stage_for_memtile(mem_tile: u32, pcfg: &PlatformConfig) -> u32 {
    mem_tile / pcfg.lpu_memtiles_per_stream_stage
}

/// Get the memtile index within a stream stage for a given memtile
fn get_memtile_index_in_stage(mem_tile: u32, pcfg: &PlatformConfig) -> usize {
    (mem_tile % pcfg.lpu_memtiles_per_stream_stage) as usize
}

/// Check if all parent nodes are scheduled and their LUT outputs can be concatenated
/// Parent LUT outputs must arrive at the same time:
/// - Same stream stage, OR
/// - A multiple of lpu_vxm_lat stages before (so data appears N*lpu_vxm_lat cycles later)
/// The concatenation engine has lpu_vxm_lat cycles latency (fully pipelined)
fn can_concatenate_parents(
    circuit: &Circuit,
    nidx: NodeIndex,
    pcfg: &PlatformConfig,
    pc: u32,
) -> bool {
    let node = circuit.graph.node_weight(nidx).unwrap();
    let parent_edges: Vec<_> = circuit.graph.edges_directed(nidx, Incoming).collect();

    if parent_edges.is_empty() {
        return true;
    }

    // Collect parent nodes that produce LUT outputs (need concatenation)
    let mut parent_lut_outputs: Vec<(NodeIndex, u32, u32)> = vec![];

    for pedge in parent_edges {
        let parent_idx = pedge.source();
        let parent_node = circuit.graph.node_weight(parent_idx).unwrap();

        // Check if this parent produces a LUT output that needs to be concatenated
        // For LUT nodes, their output is used as input to the current node
        if parent_node.is() == Primitive::Lut || parent_node.is() == Primitive::ConstLut {
            if !parent_node.info().scheduled {
                return false; // Parent not scheduled yet
            }

            let parent_pc = parent_node.info().pc;
            let parent_mem_tile = parent_node.info().lpu.mem_tile;

            if let Some(mem_tile) = parent_mem_tile {
                let parent_stage = get_stream_stage_for_memtile(mem_tile, pcfg);
                parent_lut_outputs.push((parent_idx, parent_pc, parent_stage));
            } else {
                return false; // Parent not mapped to memtile
            }
        }
        // For other primitives (Input, ConstLut with direct value, etc.), 
        // they don't need concatenation timing constraints
    }

    if parent_lut_outputs.is_empty() {
        return true; // No LUT outputs to concatenate
    }

    // Get the target node's memtile and stage
    let target_mem_tile = node.info().lpu.mem_tile;
    if target_mem_tile.is_none() {
        return false; // Node not mapped to memtile
    }
    let target_stage = get_stream_stage_for_memtile(target_mem_tile.unwrap(), pcfg);

    // Check if all parent LUT outputs can arrive at the same time for concatenation
    // They need to be in the same stream stage OR a multiple of lpu_vxm_lat stages before
    let mut arrival_times: Vec<u32> = vec![];

    for (_parent_idx, parent_pc, parent_stage) in &parent_lut_outputs {
        // Calculate the stage difference (how many stages forward the data needs to travel)
        let stage_diff = if *parent_stage <= target_stage {
            target_stage - parent_stage
        } else {
            // Wrapped around (shouldn't happen normally, but handle it)
            pcfg.lpu_stream_stages() - parent_stage + target_stage
        };

        // Check if stage_diff is a multiple of lpu_vxm_lat (including 0 = same stage)
        // This ensures data arrives at the right time for concatenation
        if stage_diff % pcfg.lpu_vxm_lat != 0 {
            return false; // Cannot concatenate - not aligned with vxm latency
        }

        // Calculate when this parent's output will be available at the target stage
        // Data flows forward through stages: data from stage S arrives at stage S+1 after 1 cycle
        // So data from parent_stage arrives at target_stage after stage_diff cycles
        // The concatenation engine adds lpu_vxm_lat cycles of latency
        // However, since concatenation is fully pipelined, we need to check when
        // the concatenated result is ready
        let arrival_pc = parent_pc + stage_diff + pcfg.lpu_vxm_lat;
        arrival_times.push(arrival_pc);
    }

    // All parent outputs must arrive at the same PC for concatenation
    if arrival_times.len() > 1 {
        let first_arrival = arrival_times[0];
        for arrival in &arrival_times[1..] {
            if *arrival != first_arrival {
                return false; // Different arrival times - cannot concatenate
            }
        }
    }

    // Check if the expected arrival time matches the current PC
    // The concatenated result (address) should be ready at pc
    if !arrival_times.is_empty() {
        let expected_arrival = arrival_times[0];
        if expected_arrival != pc {
            return false; // Cannot concatenate at this PC
        }
    }

    true
}

/// Check if memtile ports are available and stream state is correct
fn can_schedule_at_memtile(
    circuit: &Circuit,
    nidx: NodeIndex,
    lpu: &LPUState,
    pcfg: &PlatformConfig,
    pc: u32,
) -> bool {
    let node = circuit.graph.node_weight(nidx).unwrap();
    let mem_tile = node.info().lpu.mem_tile;

    if mem_tile.is_none() {
        return false; // Node not mapped to memtile
    }

    let mem_tile_id = mem_tile.unwrap();
    let stream_stage_idx = get_stream_stage_for_memtile(mem_tile_id, pcfg);
    let memtile_idx_in_stage = get_memtile_index_in_stage(mem_tile_id, pcfg);

    if stream_stage_idx as usize >= lpu.stages.len() {
        return false; // Invalid stage index
    }

    let stage = &lpu.stages[stream_stage_idx as usize];

    if memtile_idx_in_stage >= stage.mem_tiles.len() {
        return false; // Invalid memtile index
    }

    let mem_tile_state = &stage.mem_tiles[memtile_idx_in_stage];

    // Check if read port is available (for reading LUT output)
    if mem_tile_state.rd_port_busy {
        return false;
    }

    // For LUT nodes, we need to read from memtile using an address
    // The address comes from concatenated parent LUT outputs
    // If concatenation is successful (checked separately), we assume address is available
    // We need at least one stream slot available for the address
    if node.is() == Primitive::Lut || node.is() == Primitive::ConstLut {
        // Check if we have space for address (either Empty slot or can use existing Address)
        // For simplicity, we'll allow if there's at least one Empty or Address slot
        let has_space = stage.stream.regs.iter().any(|r| {
            *r == RegState::Empty || *r == RegState::Address
        });
        if !has_space {
            return false; // No space for address in stream state
        }
    }

    true
}

/// Update LPU state after scheduling a node
fn update_lpu_state_after_schedule(
    lpu: &mut LPUState,
    nidx: NodeIndex,
    circuit: &Circuit,
    pcfg: &PlatformConfig,
    pc: u32,
) {
    let node = circuit.graph.node_weight(nidx).unwrap();
    let mem_tile = node.info().lpu.mem_tile.unwrap();
    let stream_stage_idx = get_stream_stage_for_memtile(mem_tile, pcfg);
    let memtile_idx_in_stage = get_memtile_index_in_stage(mem_tile, pcfg);

    let stage = &mut lpu.stages[stream_stage_idx as usize];
    let mem_tile_state = &mut stage.mem_tiles[memtile_idx_in_stage];

    // Mark read port as busy
    mem_tile_state.rd_port_busy = true;

    // For LUT nodes, consume Address and produce LUTOutput
    if node.is() == Primitive::Lut || node.is() == Primitive::ConstLut {
        // Find a stream with Address state and replace it with LUTOutput
        // If no Address state exists, use an Empty slot (address was just created via concatenation)
        for reg in &mut stage.stream.regs {
            if *reg == RegState::Address || *reg == RegState::Empty {
                *reg = RegState::LUTOutput;
                break;
            }
        }
    }
}

fn schedule_prev_rank(
    circuit: &mut Circuit,
    candidates: &mut BTreeSet<SchedCandidate>,
    lpu: &mut LPUState,
    pc: u32,
) -> Vec<SchedCandidate> {
    let mut scheduled = vec![];
    let pcfg = &circuit.platform_cfg;

    for cand in candidates.iter() {
        let node = circuit.graph.node_weight(cand.index).unwrap();

        // Skip if already scheduled
        if node.info().scheduled {
            continue;
        }

        // Check if parent nodes are scheduled
        let parent_edges: Vec<_> = circuit.graph.edges_directed(cand.index, Incoming).collect();
        let mut all_parents_scheduled = true;
        for pedge in &parent_edges {
            let parent_node = circuit.graph.node_weight(pedge.source()).unwrap();
            if !parent_node.info().scheduled {
                all_parents_scheduled = false;
                break;
            }
        }

        if !all_parents_scheduled {
            continue;
        }

        // Check concatenation constraints for parent LUT outputs
        if !can_concatenate_parents(circuit, cand.index, pcfg, pc) {
            continue;
        }

        // Check if memtile ports are available
        if !can_schedule_at_memtile(circuit, cand.index, lpu, pcfg, pc) {
            continue;
        }

        // Node can be scheduled
        let mut node_mut = circuit.graph.node_weight_mut(cand.index).unwrap();
        let info = node_mut.info_mut();
        info.pc = pc;
        info.scheduled = true;

        // Update LPU state
        update_lpu_state_after_schedule(lpu, cand.index, circuit, pcfg, pc);

        scheduled.push(*cand);
    }

    // Remove scheduled nodes from candidates
    for sched in &scheduled {
        candidates.remove(sched);
    }

    return scheduled;
}
