use std::fmt::{Display, Write};

use strum::FromRepr;

use crate::value::Value;
use rlox_common::Array;

/// A chunk of bytecode.
///
/// A heap allocated, dynamic array of contiguous bytes.
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

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.len() - 1
    }

    pub fn ptr(&self) -> *mut u8 {
        self.code.as_ptr()
    }

    pub fn start(&self) -> *mut u8 {
        self.ptr()
    }

    pub fn len(&self) -> usize {
        self.code.len()
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
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    JumpIfFalse,
    Jump,
    Loop,
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let me_str = match self {
            OpCode::Return => "OP_RETURN",
            OpCode::AddConstant => "OP_CONSTANT",
            OpCode::AddNil => "OP_NIL",
            OpCode::AddTrue => "OP_TRUE",
            OpCode::AddFalse => "OP_FALSE",
            OpCode::Equal => "OP_EQUAL",
            OpCode::Greater => "OP_GREATER",
            OpCode::Less => "OP_LESS",
            OpCode::Negate => "OP_NEGATE",
            OpCode::Add => "OP_ADD",
            OpCode::Substract => "OP_SUBSTRACT",
            OpCode::Multiply => "OP_MULTIPLY",
            OpCode::Divide => "OP_DIVIDE",
            OpCode::Not => "OP_NOT",
            OpCode::Print => "OP_PRINT",
            OpCode::Pop => "OP_POP",
            OpCode::DefineGlobal => "OP_DEFINE_GLOBAL",
            OpCode::GetGlobal => "OP_GET_GLOBAL",
            OpCode::SetGlobal => "OP_SET_GLOBAL",
            OpCode::GetLocal => "OP_GET_LOCAL",
            OpCode::SetLocal => "OP_SET_LOCAL",
            OpCode::JumpIfFalse => "OP_JUMP_IF_FALSE",
            OpCode::Jump => "OP_JUMP",
            OpCode::Loop => "OP_LOOP",
        };

        write!(f, "{}", me_str)
    }
}

#[derive(Debug)]
pub(crate) struct Disassembler<'d> {
    chunk: &'d Chunk,
    name: &'d str,
    offset: usize,
    output: String,
}

/// A bytecode disassembler.
///
/// Takes a Chunk as an input and disassembles the bytecode into a human readable format.
impl<'d> Disassembler<'d> {
    pub fn new(chunk: &'d Chunk, name: &'d str) -> Self {
        Self {
            chunk,
            name,
            offset: 0,
            output: String::new(),
        }
    }

    pub fn disassemble(&mut self) -> &str {
        writeln!(self.output, "== {} ==", self.name);

        while self.offset < self.chunk.code.len() {
            self.disassemble_current_instruction();
        }

        &self.output
    }

    pub fn disassemble_chunk(chunk: &'d Chunk, name: &'d str) -> String {
        Self::new(chunk, name).disassemble().to_string()
    }

    pub fn disassemble_instruction(&mut self, offset: usize) -> String {
        // This is so we can keep using the instance after we have called this function.
        let old_offset = self.offset;
        self.move_current_instruction(offset);
        let result = self.disassemble_current_instruction().to_owned();
        self.offset = old_offset;
        result
    }

    fn disassemble_current_instruction(&mut self) -> &str {
        write!(self.output, "{:0<4} ", self.offset);

        if self.offset > 0 && self.chunk.lines[self.offset] == self.chunk.lines[self.offset - 1] {
            write!(self.output, "   | ");
        } else {
            write!(self.output, "{:0>4} ", self.chunk.lines[self.offset]);
        }

        let opcode: OpCode =
            OpCode::from_repr(self.chunk.code[self.offset]).expect("error fetching opcode");

        match opcode {
            OpCode::Return => self.simple_instruction("OP_RETURN"),
            OpCode::AddConstant => self.constant_instruction("OP_CONSTANT"),
            OpCode::AddNil => self.constant_instruction("OP_NIL"),
            OpCode::AddTrue => self.constant_instruction("OP_TRUE"),
            OpCode::AddFalse => self.constant_instruction("OP_FALSE"),
            OpCode::Equal => self.constant_instruction("OP_EQUAL"),
            OpCode::Greater => self.constant_instruction("OP_GREATER"),
            OpCode::Less => self.constant_instruction("OP_LESS"),
            OpCode::Negate => self.simple_instruction("OP_NEGATE"),
            OpCode::Add => self.simple_instruction("OP_ADD"),
            OpCode::Substract => self.simple_instruction("OP_SUBSTRACT"),
            OpCode::Multiply => self.simple_instruction("OP_MULTIPLY"),
            OpCode::Divide => self.simple_instruction("OP_DIVIDE"),
            OpCode::Not => self.simple_instruction("OP_NOT"),
            OpCode::Print => self.simple_instruction("OP_PRINT"),
            OpCode::Pop => self.simple_instruction("OP_POP"),
            OpCode::DefineGlobal => self.constant_instruction("OP_DEFINE_GLOBAL"),
            OpCode::GetGlobal => self.constant_instruction("OP_GET_GLOBAL"),
            OpCode::SetGlobal => self.constant_instruction("OP_SET_GLOBAL"),
            OpCode::GetLocal => self.byte_instruction("OP_GET_LOCAL"),
            OpCode::SetLocal => self.byte_instruction("OP_SET_LOCAL"),
            OpCode::JumpIfFalse => self.jump_instruction("OP_JUMP_IF_FALSE", 1),
            OpCode::Jump => self.jump_instruction("OP_JUMP", 1),
            OpCode::Loop => self.jump_instruction("OP_LOOP", -1),
            _ => unreachable!(),
        };

        &self.output
    }

    fn simple_instruction(&mut self, name: &str) {
        writeln!(self.output, "{}", name);
        self.offset += 1;
    }

    fn constant_instruction(&mut self, name: &str) {
        let constant = self.chunk.code[self.offset + 1] as usize;

        writeln!(
            self.output,
            "{:<16} {:<4} '{}'",
            name, constant, &self.chunk.constants[constant]
        );
        self.offset += 2;
    }

    fn byte_instruction(&mut self, name: &str) {
        let slot = self.chunk.code[self.offset + 1];

        writeln!(self.output, "{:<16} {:<4}", name, slot);
        self.offset += 2;
    }

    fn jump_instruction(&mut self, name: &str, sign: i32) {
        let mut jump: u16 = (self.chunk.code[self.offset + 1] as u16) << 8;
        jump |= self.chunk.code[self.offset + 2] as u16;

        writeln!(
            self.output,
            "{:<16} {:<4} -> {}",
            name,
            self.offset,
            self.offset as i32 + 3 + (sign * jump as i32)
        );
        self.offset += 3;
    }

    fn move_current_instruction(&mut self, offset: usize) {
        assert!(offset < self.chunk.len(), "offset out of bounds.");
        self.offset = offset;
    }
}
