/// Toy x86_64 JIT

use libc;
use std::alloc::{alloc, dealloc, Layout};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::io::{Read, Write, stdout};
use std::mem::transmute;
use std::ptr::write_bytes;
use std::slice;

mod x86;

use crate::ir::Instruction;

const PAGE_SIZE: usize = 4096;

pub struct Program {
    contents: *mut u8,
    size: usize,
}

impl Program {
    pub fn new(size: usize) -> Self {
        // allocate some memory to write our instructions
        let size = size * PAGE_SIZE;
        let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
        let contents = unsafe {
            let raw = alloc(layout);
            write_bytes(raw, 0xc3, size);
            libc::mprotect(raw as *mut libc::c_void, size, libc::PROT_NONE);
            raw
        };

        Program { contents, size }
    }

    pub fn into_sliceable(self) -> SliceableProgram {
        SliceableProgram::new(self)
    }

    pub fn into_callable(self) -> CallableProgram {
        CallableProgram::new(self)
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size, PAGE_SIZE).unwrap();
        unsafe {
            dealloc(self.contents, layout);
        }
    }
}

pub struct SliceableProgram {
    program: Program,
}

impl SliceableProgram {
    pub fn new(program: Program) -> Self {
        unsafe {
            libc::mprotect(
                program.contents as *mut libc::c_void,
                program.size,
                libc::PROT_READ | libc::PROT_WRITE,
            );
        }
        SliceableProgram { program }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.program.contents, self.program.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.program.contents, self.program.size) }
    }

    pub fn lock(self) -> Program {
        unsafe {
            libc::mprotect(
                self.program.contents as *mut libc::c_void,
                self.program.size,
                libc::PROT_NONE,
            );
        }
        self.program
    }
}

pub struct CallableProgram {
    program: Program,
}

impl CallableProgram {
    pub fn new(program: Program) -> Self {
        unsafe {
            libc::mprotect(
                program.contents as *mut libc::c_void,
                program.size,
                libc::PROT_READ | libc::PROT_EXEC,
            );
        }

        CallableProgram { program }
    }

    pub fn as_function(&mut self) -> unsafe extern "C" fn(*mut u8, *mut c_void) -> i32 {
        unsafe { transmute(self.program.contents) }
    }

    pub fn lock(self) -> Program {
        self.program
    }
}

#[derive(Debug)]
struct JumpInfo {
    asm_offset: usize,
    target: usize,
}

pub fn transform(instructions: &[Instruction]) -> Program {
    // we'll emit something that respects x86_64 system-v:
    // rdi (1st parameter): pointer to cell array
    let program = Program::new(8);
    let mut sliceable = program.into_sliceable();

    let slice = sliceable.as_mut_slice();
    let mut emitter = x86::Emitter::new(slice);

    let mut jumps = BTreeMap::new();

    for (idx, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::IncrementPointer(inc) => {
                if inc.is_positive() {
                    emitter.addu8_reg(x86::Register::Rdi, *inc as u8);
                } else if inc.is_negative() {
                    emitter.subu8_reg(x86::Register::Rdi, -*inc as u8);
                }
            }
            Instruction::IncrementByte(inc) => {
                if inc.is_positive() {
                    emitter.addu8_ptr(x86::Register::Rdi, *inc as u8);
                } else if inc.is_negative() {
                    emitter.subu8_ptr(x86::Register::Rdi, -*inc as u8);
                }
            }
            Instruction::IncrementPointerAndByte(pointer_inc, byte_inc) => {
                if byte_inc.is_positive() {
                    emitter.addu8_ptr_u8disp(
                        x86::Register::Rdi,
                        *pointer_inc as u8,
                        *byte_inc as u8,
                    );
                } else if byte_inc.is_negative() {
                    emitter.subu8_ptr_u8disp(
                        x86::Register::Rdi,
                        *pointer_inc as u8,
                        -*byte_inc as u8,
                    );
                }

                if pointer_inc.is_positive() {
                    emitter.addu8_reg(x86::Register::Rdi, *pointer_inc as u8);
                } else if pointer_inc.is_negative() {
                    emitter.subu8_reg(x86::Register::Rdi, -*pointer_inc as u8);
                }
            }
            // The way I've implemented jumps is terribly hacky. I should probably find a better solution someday
            Instruction::JumpBackwardsIfNotZero(jmp) => {
                emitter.cmpu8_ptr(x86::Register::Rdi, 0);

                let jumpinfo = JumpInfo {
                    target: idx - jmp,
                    asm_offset: emitter.index,
                };
                jumps.insert(idx, jumpinfo);

                // bogus temp value
                emitter.jneu32(42);
            }
            Instruction::JumpForwardsIfZero(jmp) => {
                emitter.cmpu8_ptr(x86::Register::Rdi, 0);

                let jumpinfo = JumpInfo {
                    target: idx + jmp,
                    asm_offset: emitter.index,
                };

                jumps.insert(idx, jumpinfo);
                // bogus temp value
                emitter.jeu32(42);
            }
            Instruction::OutputByte => {
                emitter.call64(x86::Register::Rsi);
            }
            _ => (),
        }
    }

    for jumpinfo in jumps.values() {
        let target = jumps.get(&jumpinfo.target).unwrap();

        // this is kinda nuts, but I'll try to explain
        // we encode jumps as x86 *near* (used to be short but brainfuck hates me) jumps
        // which are *six* bytes: two opcodes and 7 bytes of offset from the NEXT INSTRUCTION (I think?)
        // we do this indexing crazyness to rewrite our offset to our target's next instruction offset
        // TODO: x86 jumps are hard. IIRC MIPS also does this. Check when I'm less sleepy and fix these comments
        let offset = (target.asm_offset as isize) - (jumpinfo.asm_offset as isize);

        let le_bytes = (offset as u32).to_le_bytes();
        slice[jumpinfo.asm_offset + 2] = le_bytes[0];
        slice[jumpinfo.asm_offset + 3] = le_bytes[1];
        slice[jumpinfo.asm_offset + 4] = le_bytes[2];
        slice[jumpinfo.asm_offset + 5] = le_bytes[3];
    }

    sliceable.lock()
}

unsafe extern "C" fn write_trampoline(byte: *mut u8) {
    let mut output = stdout();
    output.write(&[*byte]).unwrap();
}

pub struct Vm {
    program: CallableProgram,
    cells: [u8; 30000],
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Vm {
            program: program.into_callable(),
            cells: [0; 30000],
        }
    }

    #[inline(never)]
    pub fn vm_loop<'a>(&mut self, input: &mut dyn Read, output: &'a mut dyn Write) {
        let program = self.program.as_function();
        let res = unsafe { program(self.cells.as_mut_ptr() as *mut u8, write_trampoline as *mut c_void) };

        println!("{:?}", res);
    }
}
