pub mod brainfuck;
pub mod ir;

use std::io;

fn main() {
    let bf = include_str!("./mandelbrot.b");

    let parsed = brainfuck::parse(bf.bytes());

    brainfuck::Vm::new(parsed)
        .vm_loop(&mut io::stdin(), &mut io::stdout())
        .unwrap();
}
