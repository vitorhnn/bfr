pub mod brainfuck;
pub mod ir;

use clap::arg_enum;
use structopt::StructOpt;

use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;

arg_enum! {
#[derive(Debug)]
    enum Vm {
        RawBf,
        Bfr,
    }
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(short, long, possible_values = &Vm::variants(), case_insensitive = true)]
    vm: Vm,
    #[structopt(parse(from_os_str))]
    program: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut buf = Vec::new();
    let mut bf = File::open(opt.program)?;
    bf.read_to_end(&mut buf)?;

    let parsed_bf = brainfuck::parse(buf);

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    match opt.vm {
        Vm::RawBf => brainfuck::Vm::new(parsed_bf).vm_loop(&mut stdin, &mut stdout)?,
        Vm::Bfr => {
            let ir = ir::transform(&parsed_bf)?;
            ir::Vm::new(ir).vm_loop(&mut stdin, &mut stdout)?;
        }
    }

    Ok(())
}
