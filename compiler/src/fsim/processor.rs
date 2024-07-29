use crate::fsim::common::*;
use crate::instruction::Instruction;
use crate::primitives::Primitives;
use std::fmt::Debug;

#[derive(Default, Clone, Debug)]
pub struct SwitchPort {
    ip: Bit,
    op: Bit,
}

#[derive(Clone)]
pub struct Processor {
    pub host_steps: usize,
    pub imem: Vec<Instruction>,
    pub ldm: Vec<Bit>,
    pub sdm: Vec<Bit>,
    pub io_i: Bit,
    pub io_o: Bit,
    pub pc: usize,
    pub target_cycle: usize,
    pub s_port: SwitchPort,
}

impl Processor {
    pub fn new(host_steps_: usize) -> Self {
        Processor {
            host_steps: host_steps_,
            imem: vec![Instruction::default(); host_steps_],
            ldm: vec![Bit::default(); host_steps_],
            sdm: vec![Bit::default(); host_steps_],
            io_i: 0,
            io_o: 0,
            pc: 0,
            target_cycle: 0,
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

        // Update SDM
        self.sdm[self.pc] = self.s_port.ip;

        // println!("cur_inst: {:?}", cur_inst);

        // Read the operands from the LDM and SDM
        let mut operands: Vec<Bit> = Vec::new();
        for op in cur_inst.operands.iter() {
            let rs = op.rs as usize;
            let bit = if op.local { self.ldm[rs] } else { self.sdm[rs] };
            operands.push(bit);
        }

        // println!("operands: {:?}", operands);

        // LUT lookup
        let f_out = match &cur_inst.opcode {
            Primitives::NOP => 0,
            Primitives::Input => self.io_i,
            Primitives::Lut => {
                let mut entry = 0;
                for (i, bit) in operands.iter().enumerate() {
                    entry = entry + (bit << i);
                }
                ((cur_inst.lut >> entry) & 1) as u8
            }
            Primitives::Output => {
                assert!(operands.len() == 1, "Output has {} inputs", operands.len());
                let bit = *operands.get(0).unwrap();
                self.io_o = bit;
                bit
            }
            Primitives::Gate | Primitives::Latch => {
                assert!(
                    operands.len() == 1,
                    "Gate/Latch has {} inputs",
                    operands.len()
                );
                *operands.get(0).unwrap()
            }
            _ => 0,
        };

        // println!("f_out: {:?}", f_out);

        // Set switch out
        self.s_port.op = f_out;

        // Update LDM
        self.ldm[self.pc] = f_out;

        // Increment step
        if self.pc == (self.host_steps - 1) {
            self.target_cycle += 1;
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

    pub fn set_io_i(self: &mut Self, x: Bit) {
        self.io_i = x
    }

    pub fn get_io_o(self: &mut Self) -> Bit {
        self.io_o
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
