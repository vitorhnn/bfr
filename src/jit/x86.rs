// Sincerely, fuck this ISA
#[allow(dead_code)]
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
    pub index: usize,
    buffer: &'a mut [u8],
}

impl<'a> Emitter<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Emitter { index: 0, buffer }
    }

    #[inline(always)]
    pub fn emit(&mut self, emitted: &[u8]) {
        self.buffer[self.index..self.index + emitted.len()].copy_from_slice(emitted);
        self.index += emitted.len();
    }

    // mod is a keyword in Rust!
    fn modrm(&self, mode: u8, reg: u8, rm: u8) -> u8 {
        let mode = (mode & 0b11) << 6;
        let reg = (reg & 0b111) << 3;
        let rm = rm & 0b111;

        mode | reg | rm
    }

    fn rexw_r(&self, register: Register) -> u8 {
        let mut rexw = 0b0100_1000;

        if register >= Register::R8 {
            rexw |= 0b0000_0100;
        }

        rexw
    }

    fn rexw_r_rm(&self, register: Register, rm: Register) -> u8 {
        let mut rexw = self.rexw_r(register);

        if rm >= Register::R8 {
            rexw |= 0b1;
        }

        rexw
    }

    fn rex_rm(&self, register: Register) -> Option<u8> {
        if register > Register::R8 {
            Some(0b0100_0001)
        } else {
            None
        }
    }

    pub fn addu8_reg(&mut self, register: Register, imm: u8) {
        let op = [
            self.rexw_r(register),
            0x83,
            self.modrm(0b11, 0, register as u8),
            imm,
        ];

        self.emit(&op);
    }

    pub fn subu8_reg(&mut self, register: Register, imm: u8) {
        let op = [
            self.rexw_r(register),
            0x83,
            self.modrm(0b11, 5, register as u8),
            imm,
        ];

        self.emit(&op);
    }

    pub fn addu8_ptr(&mut self, register: Register, imm: u8) {
        let op = [0x80, self.modrm(0b00, 0, register as u8), imm];

        self.emit(&op);
    }

    pub fn subu8_ptr(&mut self, register: Register, imm: u8) {
        let op = [0x80, self.modrm(0b00, 5, register as u8), imm];

        self.emit(&op);
    }

    pub fn addu8_ptr_u8disp(&mut self, register: Register, disp: u8, imm: u8) {
        let op = [0x80, self.modrm(0b01, 0, register as u8), disp, imm];

        self.emit(&op);
    }

    pub fn subu8_ptr_u8disp(&mut self, register: Register, disp: u8, imm: u8) {
        let op = [0x80, self.modrm(0b01, 5, register as u8), disp, imm];

        self.emit(&op);
    }

    pub fn cmpu8_ptr(&mut self, register: Register, imm: u8) {
        let op = [0x80, self.modrm(0b00, 7, register as u8), imm];

        self.emit(&op);
    }

    pub fn jneu32(&mut self, offset: u32) {
        let mut op = [0x0f, 0x85, 0, 0, 0, 0];

        let le_bytes = offset.to_le_bytes();

        op[2] = le_bytes[0];
        op[3] = le_bytes[1];
        op[4] = le_bytes[2];
        op[5] = le_bytes[3];

        self.emit(&op);
    }

    pub fn jeu32(&mut self, offset: u32) {
        let mut op = [0x0f, 0x84, 0, 0, 0, 0];

        let le_bytes = offset.to_le_bytes();

        op[2] = le_bytes[0];
        op[3] = le_bytes[1];
        op[4] = le_bytes[2];
        op[5] = le_bytes[3];

        self.emit(&op);
    }

    pub fn call64(&mut self, register: Register) {
        if let Some(rexrm) = self.rex_rm(register) {
            let op = [rexrm, 0xff, self.modrm(0b11, 2, register as u8)];
            self.emit(&op);
        } else {
            let op = [0xff, self.modrm(0b11, 2, register as u8)];
            self.emit(&op);
        }
    }

    pub fn push(&mut self, register: Register) {
        if let Some(rexrm) = self.rex_rm(register) {
            let op = [rexrm, 0xff, self.modrm(0b11, 6, register as u8)];
            self.emit(&op);
        } else {
            let op = [0xff, self.modrm(0b11, 6, register as u8)];
            self.emit(&op);
        }
    }

    pub fn pop(&mut self, register: Register) {
        if let Some(rexrm) = self.rex_rm(register) {
            let op = [rexrm, 0x8f, self.modrm(0b11, 0, register as u8)];
            self.emit(&op);
        } else {
            let op = [0x8f, self.modrm(0b11, 0, register as u8)];
            self.emit(&op);
        }
    }

    // I chose to match Intel's syntax for movs to keep my sanity while debugging
    pub fn mov64_reg(&mut self, dst: Register, src: Register) {
        let op = [
            self.rexw_r_rm(src, dst),
            0x89,
            self.modrm(0b11, src as u8, dst as u8),
        ];

        self.emit(&op);
    }
}
