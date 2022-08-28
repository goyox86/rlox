use std::fmt::Write;

use strum::FromRepr;

use crate::value::Value;
use rlox_common::Array;

/// A chunk of bytecode.
///
/// A heap allocated, dynamic array contiguous bytes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Chunk {
    pub code: Array<u8>,
    pub constants: Array<Value>,
    pub lines: Array<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Array::new(),
            constants: Array::new(),
            lines: Array::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.write(byte);
        self.lines.write(line);
    }

    pub fn ptr(&self) -> *mut u8 {
        self.code.as_ptr()
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.len() - 1
    }

    pub fn start(&self) -> *mut u8 {
        self.ptr()
    }
}

#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub(crate) enum OpCode {
    Return,
    AddConstant,
    AddNil,
    AddTrue,
    AddFalse,
    Equal,
    Greater,
    Less,
    Negate,
    Add,
    Substract,
    Multiply,
    Divide,
    Not,
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
    pub(crate) fn new(chunk: &'d Chunk, name: &'d str) -> Self {
        Self { chunk, name }
    }

    pub fn disassemble(&self) -> String {
        println!("== {} ==", self.name);

        let mut output = String::new();
        let mut offset: usize = 0;
        while offset < self.chunk.code.len() {
            (offset, _) = self.disassemble_instruction(offset, &mut output);
        }

        output
    }

    pub(crate) fn disassemble_chunk(chunk: &'d Chunk, name: &'d str) -> String {
        Self::new(chunk, name).disassemble()
    }

    pub fn disassemble_instruction<'output>(
        &self,
        offset: usize,
        output: &'output mut String,
    ) -> (usize, &'output String) {
        write!(output, "{:0<4} ", offset);

        if offset > 0 && self.chunk.lines[offset] == self.chunk.lines[offset - 1] {
            write!(output, "   | ");
        } else {
            write!(output, "{:0>4} ", self.chunk.lines[offset]);
        }

        let opcode: OpCode =
            OpCode::from_repr(self.chunk.code[offset]).expect("error fetching opcode");

        let offset = match opcode {
            OpCode::Return => self.simple_instruction("OP_RETURN", offset, output),
            OpCode::AddConstant => self.constant_instruction("OP_CONSTANT", offset, output),
            OpCode::AddNil => self.constant_instruction("OP_NIL", offset, output),
            OpCode::AddTrue => self.constant_instruction("OP_TRUE", offset, output),
            OpCode::AddFalse => self.constant_instruction("OP_FALSE", offset, output),
            OpCode::Equal => self.constant_instruction("OP_EQUAL", offset, output),
            OpCode::Greater => self.constant_instruction("OP_GREATER", offset, output),
            OpCode::Less => self.constant_instruction("OP_LESS", offset, output),
            OpCode::Negate => self.simple_instruction("OP_NEGATE", offset, output),
            OpCode::Add => self.simple_instruction("OP_ADD", offset, output),
            OpCode::Substract => self.simple_instruction("OP_SUBSTRACT", offset, output),
            OpCode::Multiply => self.simple_instruction("OP_MULTIPLY", offset, output),
            OpCode::Divide => self.simple_instruction("OP_DIVIDE", offset, output),
            OpCode::Not => self.simple_instruction("OP_NOT", offset, output),
            _ => unreachable!(),
        };

        (offset, output)
    }

    fn simple_instruction(&self, name: &str, offset: usize, output: &mut String) -> usize {
        writeln!(output, "{}", name);
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize, output: &mut String) -> usize {
        let constant = self.chunk.code[offset + 1];

        writeln!(
            output,
            "{:<16} {:<4} '{}'",
            name, constant, &self.chunk.constants[constant as usize]
        );
        offset + 2
    }
}
