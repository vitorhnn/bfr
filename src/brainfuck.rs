use snafu::{ResultExt, Snafu};
use std::io;
use std::io::{Read, Write};

/// A representation of all Brainfuck instructions
#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    /// Increments the data pointer by one
    IncrementPointer,
    /// Decrements the data pointer by one
    DecrementPointer,
    /// Increments the byte pointed by the data pointer by its value
    IncrementByte,
    /// Decrements the byte pointed by the data pointer by its value
    DecrementByte,
    /// Writes the byte pointed by the data pointer to some output
    OutputByte,
    /// Reads a byte from some input to the byte pointed by the data pointer
    ReadByte,
    /// Increments the current program counter to a matching JumpBackwardsIfNotZero if the byte pointed by the data pointer is equal to zero
    JumpForwardsIfZero,
    /// Decrements the current program counter to a matching JumpForwardsIfZero if the byte pointed by the data pointer is not equal to zero
    JumpBackwardsIfNotZero,
}

/// Parses a stream of bytes (assumed to be brainfuck source code) into a Vec of Brainfuck instructions
///
/// Does no optimizations at all
pub fn parse(stream: impl IntoIterator<Item = u8>) -> Vec<Instruction> {
    stream
        .into_iter()
        .filter_map(|byte| match byte {
            b'>' => Some(Instruction::IncrementPointer),
            b'<' => Some(Instruction::DecrementPointer),
            b'+' => Some(Instruction::IncrementByte),
            b'-' => Some(Instruction::DecrementByte),
            b'.' => Some(Instruction::OutputByte),
            b',' => Some(Instruction::ReadByte),
            b'[' => Some(Instruction::JumpForwardsIfZero),
            b']' => Some(Instruction::JumpBackwardsIfNotZero),
            _ => None,
        })
        .collect()
}

/// A pure Brainfuck virtual machine
///
/// Does no optimizations and is probably as slow as it gets
pub struct Vm {
    program: Vec<Instruction>,
    program_counter: usize,
    cells: [u8; 30000],
    data_pointer: usize,
}

#[derive(Snafu, Debug)]
pub enum VmError {
    #[snafu(display("Failed to find a matching jump"))]
    NoMatchingJump,
    #[snafu(display("Failed to write byte to output"))]
    FailedToWrite { source: io::Error },
    #[snafu(display("Failed to read byte from input"))]
    FailedToRead { source: io::Error },
}

impl Vm {
    /// Creates a new instance of a plain brainfuck vm, using a stream of instructions as the program
    pub fn new(program: Vec<Instruction>) -> Self {
        Vm {
            program,
            program_counter: 0,
            data_pointer: 0,
            cells: [0; 30000],
        }
    }

    fn current_byte(&mut self) -> &mut u8 {
        // safety: we do bounds checking on increments and decrements to self.data_pointer
        unsafe { self.cells.get_unchecked_mut(self.data_pointer) }
    }

    /// Executes a single brainfuck instruction
    pub fn step(&mut self, input: &mut dyn Read, output: &mut dyn Write) -> Result<(), VmError> {
        let instruction = &self.program[self.program_counter];

        match instruction {
            Instruction::IncrementPointer => {
                if self.data_pointer.wrapping_add(1) > self.cells.len() {
                    panic!("data pointer out of bounds!");
                }

                self.data_pointer = self.data_pointer.wrapping_add(1);
                self.program_counter = self.program_counter.wrapping_add(1);
            }
            Instruction::DecrementPointer => {
                if self.data_pointer.wrapping_sub(1) > self.cells.len() {
                    panic!("data pointer out of bounds!");
                }

                self.data_pointer = self.data_pointer.wrapping_sub(1);
                self.program_counter = self.program_counter.wrapping_add(1);
            }
            Instruction::IncrementByte => {
                let byte = self.current_byte();
                *byte = byte.wrapping_add(1);
                self.program_counter = self.program_counter.wrapping_add(1);
            }
            Instruction::DecrementByte => {
                let byte = self.current_byte();
                *byte = byte.wrapping_sub(1);
                self.program_counter = self.program_counter.wrapping_add(1);
            }
            Instruction::OutputByte => {
                let byte = self.current_byte();
                output.write(&[*byte]).context(FailedToWrite)?;
                self.program_counter = self.program_counter.wrapping_add(1);
            }
            Instruction::ReadByte => {
                input
                    .read(&mut self.cells[self.data_pointer..1])
                    .context(FailedToRead)?;
                self.program_counter += 1;
            }
            Instruction::JumpForwardsIfZero => {
                let byte = self.current_byte();

                // this is quite a dumb way to do this
                if *byte == 0 {
                    let mut opened = 1;
                    let mut jump = self.program_counter;

                    loop {
                        jump = jump.wrapping_add(1);

                        if jump >= self.program.len() {
                            return Err(VmError::NoMatchingJump);
                        }

                        let instruction = &self.program[jump];

                        match instruction {
                            Instruction::JumpForwardsIfZero => opened += 1,
                            Instruction::JumpBackwardsIfNotZero => opened -= 1,
                            _ => (),
                        }

                        if opened == 0 {
                            break;
                        }
                    }

                    self.program_counter = jump;
                } else {
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
            }
            Instruction::JumpBackwardsIfNotZero => {
                let byte = self.current_byte();

                if *byte != 0 {
                    let mut closed = 1;
                    let mut jump = self.program_counter;

                    loop {
                        jump = jump.wrapping_sub(1);

                        if jump >= self.program.len() {
                            return Err(VmError::NoMatchingJump);
                        }

                        let instruction = &self.program[jump];

                        match instruction {
                            Instruction::JumpForwardsIfZero => closed -= 1,
                            Instruction::JumpBackwardsIfNotZero => closed += 1,
                            _ => (),
                        }

                        if closed == 0 {
                            break;
                        }
                    }

                    self.program_counter = jump;
                } else {
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
            }
        }

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
