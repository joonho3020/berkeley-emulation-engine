use crate::fsim::common::*;
use crate::primitives::*;
use std::fmt::Debug;

#[derive(Default, Clone, Debug)]
struct SwitchPort {
    ip: Bit,
    op: Bit,
}

#[derive(Clone)]
pub struct Processor {
    max_steps: usize,
    imem: Vec<Instruction>,
    ldm: Vec<Bit>,
    sdm: Vec<Bit>,
    io_i: Bit,
    io_o: Bit,
    pc: usize,
    cycle: usize,
    s_port: SwitchPort,
}

impl Processor {
    pub fn new(max_steps_: usize) -> Self {
        Processor {
            max_steps: max_steps_,
            imem: vec![Instruction::default(); max_steps_],
            ldm: vec![Bit::default(); max_steps_],
            sdm: vec![Bit::default(); max_steps_],
            io_i: 0,
            io_o: 0,
            pc: 0,
            cycle: 0,
            s_port: SwitchPort::default(),
        }
    }

    pub fn set_inst(self: &mut Self, inst: Instruction, step: usize) {
        assert!(step < self.imem.len());
        self.imem[step] = inst;
    }

    pub fn step(self: &mut Self) {
        // Instruction fetch
        let cur_inst = &self.imem[self.pc];

        // Read the operands from the LDM and SDM
        let mut operands: Vec<Bit> = Vec::new();
        for op in cur_inst.operands.iter() {
            if op.valid {
                let rs = op.rs as usize;
                let bit = if op.local { self.ldm[rs] } else { self.sdm[rs] };
                operands.push(bit);
            }
        }

        // LUT lookup
        let f_out = 0;
        // cur_inst.opcode.perform_operation(operands);

        // Set switch out
        self.s_port.op = f_out;

        // Update LDM & SDM
        self.ldm[self.pc] = f_out;
        self.sdm[self.pc] = self.s_port.ip;

        // Increment step
        if self.pc == (self.max_steps - 1) {
            self.cycle += 1;
            self.pc = 0;
        } else {
            self.pc += 1;
        }
    }

    pub fn get_switch_in_id(self: &Self) -> Bits32 {
        self.imem[self.pc].sin.idx
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
            "Proc[\n  {:?}\n  {:?}\n  io_i {} io_o {}\n",
            self.imem[self.pc], self.s_port, self.io_i, self.io_o
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
