#![allow(unused)]

mod vm;

use std::{
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
    process::exit,
};

use rlox_compiler::compiler::CompilerOptions;
use vm::Vm;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    trace_execution: bool,
    #[clap(short, long, value_parser)]
    print_code: bool,

    // Loc source code file path
    file_path: Option<PathBuf>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let vm_opts = vm::Options {
        trace_execution: args.trace_execution,
        compiler: CompilerOptions {
            print_code: args.print_code,
        },
    };

    if let Some(ref file_path) = args.file_path {
        run_file(file_path, Some(vm_opts))?;
    } else {
        repl(None)?;
    }

    Ok(())
}

fn run_file(file_path: &Path, vm_opts: Option<vm::Options>) -> std::io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut source = String::new();

    file.read_to_string(&mut source)?;

    let mut vm = Vm::new(vm_opts);
    let result = vm.interpret(source);

    match result {
        Ok(_) => exit(0),
        Err(vm::VmError::Compile(error)) => {
            eprintln!("compile error: {:?}", error);
            exit(65)
        }
        Err(vm::VmError::Runtime(error)) => {
            eprintln!("runtime error: {:?}", error);
            exit(70)
        }
    }
}

fn repl(vm_opts: Option<vm::Options>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut vm = Vm::new(vm_opts);

    print!("> ");
    std::io::stdout().flush()?;
    for line in stdin.lock().lines() {
        print!("> ");
        let _ = vm.interpret(line?);
    }

    Ok(())
}
