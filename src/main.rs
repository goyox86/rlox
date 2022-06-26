mod bytecode;
mod compiler;
mod value;
mod vm;

use std::{
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
    process::exit,
};

use bytecode::Chunk;
use vm::Vm;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    trace_execution: bool,

    // Loc source code file path
    file_path: Option<PathBuf>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let vm_opts = vm::Options {
        trace_execution: args.trace_execution,
    };

    if let Some(ref file_path) = args.file_path {
        run_file(file_path, None)?;
    } else {
        repl(None)?;
    }

    Ok(())
}

fn run_file(file_path: &Path, vm_opts: Option<vm::Options>) -> std::io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;

    let mut vm = Vm::new(Chunk::new(), vm_opts);
    let _result = vm.compile(buffer);

    exit(0)

    // match result {
    //     Ok(_) => exit(0),
    //     Err(vm::Error::Compile(_)) => exit(65),
    //     Err(vm::Error::Runtime(_)) => exit(70),
    // }
}

fn repl(vm_opts: Option<vm::Options>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut vm = Vm::new(Chunk::new(), vm_opts);

    print!("> ");
    std::io::stdout().flush()?;
    for line in stdin.lock().lines() {
        print!("> ");
        let _ = vm.compile(line?);
    }

    Ok(())
}

// fn main() {
//     let mut chunk = Chunk::new();
//
//     let mut constant = chunk.add_constant(Value::Number(1.2));
//     chunk.write(OpCode::AddConstant as u8, 123);
//     chunk.write(constant as u8, 123);
//
//     constant = chunk.add_constant(Value::Number(3.4));
//     chunk.write(OpCode::AddConstant as u8, 123);
//     chunk.write(constant as u8, 123);
//
//     chunk.write(OpCode::Add as u8, 123);
//
//     constant = chunk.add_constant(Value::Number(5.6));
//     chunk.write(OpCode::AddConstant as u8, 123);
//     chunk.write(constant as u8, 123);
//
//     chunk.write(OpCode::Divide as u8, 123);
//
//     for _ in 0..1_000_000 {
//         chunk.write(OpCode::Negate as u8, 123);
//     }
//
//     // chunk.write(OpCode::Negate as u8, 123);
//     chunk.write(OpCode::Return as u8, 123);
//
//     let cli_args = Args::parse();
//     let vm_opts = vm::Options {
//         trace_execution: cli_args.trace_execution,
//     };
//
//     let mut vm = Vm::new(chunk, Some(vm_opts));
//     let _ = vm.interpret();
// }
