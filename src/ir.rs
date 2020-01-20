/// A very simple IR generated from Brainfuck bytecode and a VM that interprets it
use itertools::Itertools;
use snafu::{ResultExt, Snafu};
use std::io;
use std::io::{Read, Write};
use tinyvec::{array_vec, ArrayVec};

use crate::brainfuck::Instruction as BfInstruction;

/// A (kinda) superset of brainfuck's instruction set.
/// Attempts to combine operations which are commonly repeated (increments) and precompute jumps
/// TODO: Maybe do more optimizations?
#[derive(Debug)]
pub enum Instruction {
    /// Increments the data pointer by its value
    IncrementPointer(i32),
    /// Increments the byte pointed by the data pointer by its value
    IncrementByte(i32),
    /// Writes the byte pointed by the data pointer to some output
    OutputByte,
    /// Reads a byte from some input to the byte pointed by the data pointer
    ReadByte,
    /// Increments the current program counter by its value if the byte pointed by the data pointer is equal to zero
    JumpForwardsIfZero(usize),
    /// Decrements the current program counter by its value if the byte pointed by the data pointer is not equal to zero
    JumpBackwardsIfNotZero(usize),
}

fn aggregate_byte_ops<'a, I>(iter: &mut I) -> i32
where
    I: Iterator<Item = &'a BfInstruction> + Clone,
{
    iter.take_while_ref(|&x| {
        *x == BfInstruction::IncrementByte || *x == BfInstruction::DecrementByte
    })
    .fold(0, |agg, x| match x {
        BfInstruction::IncrementByte => agg + 1,
        BfInstruction::DecrementByte => agg - 1,
        _ => unreachable!(),
    })
}

fn aggregate_pointer_ops<'a, I>(iter: &mut I) -> i32
where
    I: Iterator<Item = &'a BfInstruction> + Clone,
{
    iter.take_while_ref(|&x| {
        *x == BfInstruction::IncrementPointer || *x == BfInstruction::DecrementPointer
    })
    .fold(0, |agg, x| match x {
        BfInstruction::IncrementPointer => agg + 1,
        BfInstruction::DecrementPointer => agg - 1,
        _ => unreachable!(),
    })
}

#[derive(Snafu, Debug)]
pub enum TransformError {
    #[snafu(display("No matching jump"))]
    NoMatchingJump,
}

/// Transforms raw Brainfuck instructions into BFR IR, which should hopefully be more efficient
pub fn transform(instructions: &[BfInstruction]) -> Result<Vec<Instruction>, TransformError> {
    let mut it = instructions.iter();
    let mut transformed = Vec::with_capacity(instructions.len());

    // pass 1: combine increments
    while let Some(instr) = it.next() {
        let res = match instr {
            BfInstruction::IncrementByte => {
                let agg = 1 + aggregate_byte_ops(&mut it);

                Instruction::IncrementByte(agg)
            }
            BfInstruction::DecrementByte => {
                let agg = -1 + aggregate_byte_ops(&mut it);

                Instruction::IncrementByte(agg)
            }
            BfInstruction::IncrementPointer => {
                let agg = 1 + aggregate_pointer_ops(&mut it);

                Instruction::IncrementPointer(agg)
            }
            BfInstruction::DecrementPointer => {
                let agg = -1 + aggregate_pointer_ops(&mut it);

                Instruction::IncrementPointer(agg)
            }
            BfInstruction::OutputByte => Instruction::OutputByte,
            BfInstruction::ReadByte => Instruction::ReadByte,
            // We'll calculate jumps in the next pass.
            // We can't do it now because we don't have stable indices for instructions
            BfInstruction::JumpBackwardsIfNotZero => Instruction::JumpBackwardsIfNotZero(0),
            BfInstruction::JumpForwardsIfZero => Instruction::JumpForwardsIfZero(0),
        };

        transformed.push(res);
    }

    // pass 2: precompute jumps
    let mut stack = array_vec!([usize; 32]);
    for idx in 0..transformed.len() {
        let instr = &transformed[idx];

        match instr {
            Instruction::JumpForwardsIfZero(_) => {
                stack.push(idx);
            }
            Instruction::JumpBackwardsIfNotZero(_) => {
                let target_idx = match stack.pop() {
                    Some(idx) => idx,
                    None => return Err(TransformError::NoMatchingJump),
                };

                let distance = idx - target_idx;

                transformed[target_idx] = Instruction::JumpForwardsIfZero(distance);
                transformed[idx] = Instruction::JumpBackwardsIfNotZero(distance);
            }
            _ => (), // we do nothing for other instructions in this pass
        }
    }

    Ok(transformed)
}

/// A BFR IR virtual machine
///
/// Slightly more optimized than the pure Brainfuck vm
pub struct Vm {
    program: Vec<Instruction>,
    program_counter: usize,
    cells: [u8; 30000],
    data_pointer: usize,
}

#[derive(Snafu, Debug)]
pub enum VmError {
    #[snafu(display("Failed to write byte to output"))]
    FailedToWrite { source: io::Error },
    #[snafu(display("Failed to read byte from input"))]
    FailedToRead { source: io::Error },
}

impl Vm {
    /// Creates a new instance of a BFR IR vm, using a stream of instructions as the program
    pub fn new(program: Vec<Instruction>) -> Self {
        Vm {
            program,
            program_counter: 0,
            data_pointer: 0,
            cells: [0; 30000],
        }
    }

    fn current_byte_mut(&mut self) -> &mut u8 {
        // safety: we do bounds checking on increments and decrements to self.data_pointer
        unsafe { self.cells.get_unchecked_mut(self.data_pointer) }
    }

    fn current_byte(&self) -> &u8 {
        // safety: we do bounds checking on increments and decrements to self.data_pointer
        unsafe { self.cells.get_unchecked(self.data_pointer) }
    }

    /// Executes a single BFR IR instruction
    pub fn step(&mut self, input: &mut dyn Read, output: &mut dyn Write) -> Result<(), VmError> {
        let pc = match self.program[self.program_counter] {
            Instruction::IncrementPointer(inc) => {
                if self.data_pointer.wrapping_add(inc as usize) > self.cells.len() {
                    panic!("data pointer out of bounds!");
                }

                self.data_pointer = self.data_pointer.wrapping_add(inc as usize);
                self.program_counter.wrapping_add(1)
            }
            Instruction::IncrementByte(inc) => {
                let byte = self.current_byte_mut();
                let extended = *byte as i32;
                // TODO: I'm fairly sure this is wrong
                *byte = extended.wrapping_add(inc) as u8;
                self.program_counter.wrapping_add(1)
            }
            Instruction::OutputByte => {
                let byte = self.current_byte();
                output.write(&[*byte]).context(FailedToWrite)?;
                self.program_counter.wrapping_add(1)
            }
            Instruction::ReadByte => {
                input
                    .read(&mut self.cells[self.data_pointer..1])
                    .context(FailedToRead)?;
                self.program_counter.wrapping_add(1)
            }
            Instruction::JumpForwardsIfZero(jmp) => {
                let byte = self.current_byte();

                if *byte == 0 {
                    self.program_counter.wrapping_add(jmp)
                } else {
                    self.program_counter.wrapping_add(1)
                }
            }
            Instruction::JumpBackwardsIfNotZero(jmp) => {
                let byte = self.current_byte();

                if *byte != 0 {
                    self.program_counter.wrapping_sub(jmp)
                } else {
                    self.program_counter.wrapping_add(1)
                }
            }
        };

        self.program_counter = pc;

        Ok(())
    }

    /// Runs the program to end
    pub fn vm_loop(&mut self, input: &mut dyn Read, output: &mut dyn Write) -> Result<(), VmError> {
        while self.program_counter < self.program.len() {
            self.step(input, output)?;
        }

        Ok(())
    }
}
