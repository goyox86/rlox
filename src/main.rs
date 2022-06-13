mod bytecode;
mod value;

use bytecode::{Chunk, OpCode};
use value::Value;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(Value::Number(1.2));
    chunk.write(OpCode::AddConstant as u8, 123);
    chunk.write(constant as u8, 123);
    chunk.write(OpCode::Return as u8, 123);
    let disasm = chunk.disassemble("test chunk");
    println!("{}", disasm)
}
