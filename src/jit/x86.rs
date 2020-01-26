#[allow(dead_code)]
// Sincerely, fuck this ISA

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Register {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

pub struct Emitter<'a> {
    index: usize,
    buffer: &'a mut [u8],
}

impl<'a> Emitter<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Emitter {
            index: 0,
            buffer,
        }
    }

    pub fn emit(&mut self, byte: u8) {
        self.buffer[self.index] = byte;
        self.index += 1;
    }

    // mod is a keyword in Rust!
    fn modrm(&mut self, mode: u8, reg: u8, rm: u8) {
        let mode = (mode & 0b11) << 6;
        let reg = (reg & 0b111) << 3;
        let rm = rm & 0b111;

        self.emit(mode | reg | rm);
    }

    fn rexw_r(&mut self, register: Register) {
        let mut rexw = 0b0100_1000;

        if register >= Register::R8 {
            rexw |= 0b0000_0100;
        }

        self.emit(rexw);
    }

    pub fn addu8_reg(&mut self, register: Register, imm: u8) {
        self.rexw_r(register);
        self.emit(0x83);
        self.modrm(0b11, 0, register as u8);
        self.emit(imm);
    }
}
