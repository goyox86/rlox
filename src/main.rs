mod bytecode;
mod value;
mod vm;

use bytecode::{Chunk, OpCode};
use value::Value;
use vm::Vm;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    trace_execution: bool,
}

fn main() {
    let mut chunk = Chunk::new();

    let mut constant = chunk.add_constant(Value::Number(1.2));
    chunk.write(OpCode::AddConstant as u8, 123);
    chunk.write(constant as u8, 123);

    constant = chunk.add_constant(Value::Number(3.4));
    chunk.write(OpCode::AddConstant as u8, 123);
    chunk.write(constant as u8, 123);

    chunk.write(OpCode::Add as u8, 123);

    constant = chunk.add_constant(Value::Number(5.6));
    chunk.write(OpCode::AddConstant as u8, 123);
    chunk.write(constant as u8, 123);

    chunk.write(OpCode::Divide as u8, 123);
    chunk.write(OpCode::Negate as u8, 123);
    chunk.write(OpCode::Return as u8, 123);

    let cli_args = Args::parse();
    let vm_opts = vm::Options {
        trace_execution: cli_args.trace_execution,
    };

    let mut vm = Vm::new(chunk, Some(vm_opts));
    let _ = vm.interpret();
}
