use crate::fsim::common::*;
use std::fmt::Debug;

#[derive(Default, Clone, Debug)]
pub enum Opcode {
    #[default]
    NOP,
    AND,
    OR,
    INV,
}

impl Opcode {
    fn perform_operation(self: &Self, operands: Vec<Bit>) -> Bit {
        let optbit = match self {
            Opcode::NOP => Some(0 as u8),
            Opcode::AND => operands.into_iter().reduce(|a, b| a & b),
            Opcode::OR => operands.into_iter().reduce(|a, b| a | b),
            Opcode::INV => Some(!operands[0]),
        };
        match optbit {
            Some(x) => x,
            None => 0,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub enum SwitchOutSel {
    #[default]
    FUNC,
    LSDM,
    EXTERNAL,
}

#[derive(Default, Clone, Debug)]
pub struct OperandInfo {
    ldm_addr: Bits32,
    sdm_addr: Bits32,
    op_sel: Bit,
}

#[derive(Default, Clone, Debug)]
pub struct Inst {
    op_infos: Vec<OperandInfo>,
    s_op_info: OperandInfo,
    opcode: Opcode,
    s_out_sel: SwitchOutSel,
    sin_id: Bits32,
}

#[derive(Default, Clone, Debug)]
struct SwitchPort {
    ip: Bit,
    op: Bit,
}

#[derive(Clone)]
pub struct Processor {
    max_steps: usize,
    imem: Vec<Inst>,
    ldm: Vec<Bit>,
    sdm: Vec<Bit>,
    external: Bit,
    step: usize,
    cycle: usize,
    s_port: SwitchPort,
}

impl Processor {
    pub fn new(max_steps_: usize) -> Self {
        Processor {
            max_steps: max_steps_,
            imem: vec![Inst::default(); max_steps_],
            ldm: vec![Bit::default(); max_steps_],
            sdm: vec![Bit::default(); max_steps_],
            external: 0,
            step: 0,
            cycle: 0,
            s_port: SwitchPort::default(),
        }
    }

    pub fn set_inst(self: &mut Self, inst: Inst, step: usize) {
        assert!(step < self.imem.len());
        self.imem[step] = inst;
    }

    pub fn step(self: &mut Self) {
        // Instruction fetch
        let cur_inst = &self.imem[self.step];

        // Read the operands from the LDM and SDM
        let mut operands: Vec<Bit> = Vec::new();
        for oi in cur_inst.op_infos.iter() {
            let ldm_bit = self.ldm[oi.ldm_addr as usize];
            let sdm_bit = self.sdm[oi.sdm_addr as usize];
            let bit = if oi.op_sel == 0 { ldm_bit } else { sdm_bit };
            operands.push(bit);
        }

        let s_op_out = if cur_inst.s_op_info.op_sel == 0 {
            self.ldm[cur_inst.s_op_info.ldm_addr as usize]
        } else {
            self.sdm[cur_inst.s_op_info.sdm_addr as usize]
        };

        // LUT lookup
        let f_out = cur_inst.opcode.perform_operation(operands);

        // Set switch out
        self.s_port.op = match cur_inst.s_out_sel {
            SwitchOutSel::FUNC => f_out,
            SwitchOutSel::LSDM => s_op_out,
            SwitchOutSel::EXTERNAL => self.external,
        };

        // Update LDM & SDM
        self.ldm[self.step] = f_out;
        self.sdm[self.step] = self.s_port.ip;

        // Increment step
        if self.step == (self.max_steps - 1) {
            self.cycle += 1;
            self.step = 0;
        } else {
            self.step += 1;
        }
    }

    pub fn get_switch_in_id(self: &Self) -> Bits32 {
        self.imem[self.step].sin_id
    }

    pub fn set_switch_in(self: &mut Self, b: Bit) {
        self.s_port.ip = b;
    }

    pub fn get_switch_out(self: &mut Self) -> Bit {
        self.s_port.op
    }
}

impl Debug for Processor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Proc[\n  {:?}\n  {:?}\n  external {:#}\n",
            self.imem[self.step], self.s_port, self.external
        )?;

        write!(f, "  ldm:\n")?;
        for chunk in self.ldm.chunks(8) {
            write!(f, "\t{:?}\n", chunk)?;
        }

        write!(f, "  sdm:\n")?;
        for chunk in self.sdm.chunks(8) {
            write!(f, "\t{:?}\n", chunk)?;
        }
        write!(f, "]\n")?;
        Ok(())
    }
}
