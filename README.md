# BFR: Brainfuck, rapidly

A bunch of different Brainfuck interpreters and JIT compilers:

 * A naive bytecode interpreter that does no optimization passes and calculates jump locations as it executes
 * A IR interpreter that does very simple peephole optimizations of pointer and byte increments and decrements and precalculates jump locations (~6x speedup on mandelbrot.b)

TODO:
 * More optimizations for the IR interpreter
 * A simple JIT compiler, targeting x86_64
 * Something using [inkwell](https://github.com/TheDan64/inkwell) or [Cranelift](https://github.com/bytecodealliance/cranelift)?
