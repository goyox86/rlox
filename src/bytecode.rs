use std::write;

use crate::value::Value;
use rlox_common::array::Array;

/// A chunk of bytecode.
///
/// A heap allocated, dynamic array contiguous bytes.
#[derive(Debug)]
pub(crate) struct Chunk {
    code: Array<u8>,
    constants: Array<Value>,
    lines: Array<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Array::new(),
            constants: Array::new(),
            lines: Array::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: u32) {
        self.code.write(byte);
        self.lines.write(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.len() - 1
    }

    pub fn disassemble(&self, name: &str) -> String {
        let disassembler = Disassembler::new(self, name);

        disassembler.disassemble()
    }
}

#[repr(u8)]
#[derive(Debug)]
pub(crate) enum OpCode {
    Return = 1,
    AddConstant = 2,
}

impl TryFrom<u8> for OpCode {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(OpCode::Return),
            2 => Ok(OpCode::AddConstant),
            _ => Err("unknown u8 value, cannot build opcode"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Disassembler<'d> {
    chunk: &'d Chunk,
    name: &'d str,
}

/// A bytecode disassembler.
///
/// Takes a Chunk as an input and disassembles the bytecode into a human readable format.
impl<'d> Disassembler<'d> {
    pub fn new(chunk: &'d Chunk, name: &'d str) -> Self {
        Self { chunk, name }
    }

    pub fn disassemble(&self) -> String {
        println!("== {} ==", self.name);

        let mut output = String::new();
        let mut offset: usize = 0;
        while offset < self.chunk.code.len() {
            offset = self.disassemble_instruction(offset, &mut output);
        }

        output
    }

    fn disassemble_instruction(&self, offset: usize, output: &mut String) -> usize {
        output.push_str(&format!("{:0<4} ", offset));

        if offset > 0 && self.chunk.lines[offset] == self.chunk.lines[offset - 1] {
            output.push_str("   | ");
        } else {
            output.push_str(&format!("{:0>4} ", self.chunk.lines[offset]));
        }

        let opcode = match OpCode::try_from(self.chunk.code[offset]) {
            Ok(opcode) => opcode,
            Err(err) => panic!("{}", err),
        };

        match opcode {
            OpCode::Return => self.simple_instruction("OP_RETURN", offset, output),
            OpCode::AddConstant => self.constant_instruction("OP_CONSTANT", offset, output),
        }
    }

    fn simple_instruction(&self, name: &str, offset: usize, output: &mut String) -> usize {
        output.push_str(&format!("{}\n", name));
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize, output: &mut String) -> usize {
        let constant = self.chunk.code[offset + 1];
        output.push_str(&format!(
            "{:<16} {:<4} '{}'\n",
            name,
            constant,
            self.chunk.constants[constant.into()]
        ));
        offset + 2
    }
}
