/// Toy x86_64 JIT
use std::alloc::{Layout, alloc, dealloc};
use std::ptr::write_bytes;
use std::slice;
use std::mem::transmute;
use std::io::{Read, Write};
use libc;

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
        let layout = Layout::from_size_align(PAGE_SIZE, size).unwrap();
        let contents = unsafe {
            let raw = alloc(layout);
            write_bytes(raw, 0xc3, size);
            libc::mprotect(raw as *mut libc::c_void, size, libc::PROT_NONE);
            raw
        };

        Program {
            contents,
            size
        }
    }

    pub fn to_sliceable(self) -> SliceableProgram {
        SliceableProgram::new(self)
    }

    pub fn to_callable(self) -> CallableProgram {
        CallableProgram::new(self)
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(PAGE_SIZE, self.size).unwrap();
        unsafe { dealloc(self.contents, layout); }
    }
}

pub struct SliceableProgram {
    program: Program,
}

impl SliceableProgram {
    pub fn new(program: Program) -> Self {
        unsafe { libc::mprotect(program.contents as *mut libc::c_void, program.size, libc::PROT_READ | libc::PROT_WRITE); }
        SliceableProgram {
            program,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.program.contents, self.program.size * PAGE_SIZE) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.program.contents, self.program.size * PAGE_SIZE) }
    }

    pub fn lock(self) -> Program {
        unsafe { libc::mprotect(self.program.contents as *mut libc::c_void, self.program.size, libc::PROT_NONE); }
        self.program
    }
}

pub struct CallableProgram {
    program: Program,
}

impl CallableProgram {
    pub fn new(program: Program) -> Self {
        unsafe { libc::mprotect(program.contents as *mut libc::c_void, program.size, libc::PROT_READ | libc::PROT_EXEC); }

        CallableProgram {
            program,
        }
    }

    pub fn as_function(&mut self) -> unsafe extern "C" fn(*mut u8) -> i32 {
        unsafe { transmute(self.program.contents) }
    }

    pub fn lock(self) -> Program {
        self.program
    }
}


pub fn transform(instructions: &[Instruction]) -> Program {
    // we'll emit something that respects x86_64 system-v:
    // rdi (1st parameter): pointer to cell array
    let program = Program::new(1);
    let mut sliceable = program.to_sliceable();
    
    let slice = sliceable.as_mut_slice();
    let mut emitter = x86::Emitter::new(slice);
    emitter.emit(0x31);
    emitter.emit(0xc0);

    emitter.addu8_reg(x86::Register::Rax, 42);

    /*
    for instr in instructions {
        match instr {
            Instruction::IncrementPointer(inc) => {
                if inc.is_positive() {

                }
                
                break;
            }
            _ => break,
        }
    }
    */

    sliceable.lock()
}

pub struct Vm {
    program: CallableProgram,
    cells: [u8; 30000],
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Vm {
            program: program.to_callable(),
            cells: [0; 30000],
        }
    }

    pub fn vm_loop(&mut self, input: &mut dyn Read, output: &mut dyn Write) {
        let program = self.program.as_function();
        let res = unsafe { program(self.cells.as_mut_ptr() as *mut u8) };

        panic!("{:?}", res);
    }
}