/// A (kinda) superset of brainfuck's instruction set.
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
    JumpForwardsIfZero(u32),
    /// Decrements the current program counter by its value if the byte pointed by the data pointer is not equal to zero
    JumpBackwardsIfNotZero(u32),
}