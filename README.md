# BFR: Brainfuck, rapidly

A bunch of different Brainfuck interpreters and JIT compilers:

 * A naive bytecode interpreter that does no optimization passes and calculates jump locations as it executes
 * A IR interpreter that does very simple peephole optimizations of pointer and byte increments and decrements and precalculates jump locations (~6x speedup on mandelbrot.b)
 * A simple JIT compiler, targeting x86_64

The transformation pipeline is:
```
Raw Brainfuck source code (Iterator of u8's) -> Vec<brainfuck::Instruction> (brainfuck::parse) -> Vec<ir::Instruction> (ir::transform) -> jit::Program (jit::transform)
                                                - Interpreted by brainfuck::Vm (--vm rawbf)       - Interpreted by ir::Vm (--vm bfr)      - JIT executed by jit::Vm (--vm jit)
```

TODO:
 * More IR level optimizations
 * A better way to represent IR level transformations, to allow turning them on and off as desired (something like a Transformation trait)
 * Something using [inkwell](https://github.com/TheDan64/inkwell) or [Cranelift](https://github.com/bytecodealliance/cranelift)?
