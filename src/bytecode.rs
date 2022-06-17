use crate::value::Value;
use rlox_common::array::Array;

/// A chunk of bytecode.
///
/// A heap allocated, dynamic array contiguous bytes.
#[derive(Debug)]
pub(crate) struct Chunk {
    pub code: Array<u8>,
    pub constants: Array<Value>,
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

    pub fn ptr(&self) -> *mut u8 {
        self.code.ptr()
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.len() - 1
    }
}

#[repr(u8)]
#[derive(Debug)]
pub(crate) enum OpCode {
    Return = 1,
    AddConstant = 2,
    Negate = 3,
    Add = 4,
    Substract = 5,
    Multiply = 6,
    Divide = 7,
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> u8 {
        match value {
            OpCode::Return => 1,
            OpCode::AddConstant => 2,
            OpCode::Negate => 3,
            OpCode::Add => 4,
            OpCode::Substract => 5,
            OpCode::Multiply => 6,
            OpCode::Divide => 7,
        }
    }
}

impl From<u8> for OpCode {
    fn from(byte: u8) -> Self {
        match byte {
            1 => OpCode::Return,
            2 => OpCode::AddConstant,
            3 => OpCode::Negate,
            4 => OpCode::Add,
            5 => OpCode::Substract,
            6 => OpCode::Multiply,
            7 => OpCode::Divide,
            _ => panic!("unimplemented conversion u8 -> opcode"),
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
            (offset, _) = self.disassemble_instruction(offset, &mut output);
        }

        output
    }

    pub fn disassemble_instruction<'output>(
        &self,
        offset: usize,
        output: &'output mut String,
    ) -> (usize, &'output String) {
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

        let offset = match opcode {
            OpCode::Return => self.simple_instruction("OP_RETURN", offset, output),
            OpCode::AddConstant => self.constant_instruction("OP_CONSTANT", offset, output),
            OpCode::Negate => self.simple_instruction("OP_NEGATE", offset, output),
            OpCode::Add => self.simple_instruction("OP_ADD", offset, output),
            OpCode::Substract => self.simple_instruction("OP_SUBSTRACT", offset, output),
            OpCode::Multiply => self.simple_instruction("OP_MULTIPLY", offset, output),
            OpCode::Divide => self.simple_instruction("OP_DIVIDE", offset, output),
            _ => self.simple_instruction("OP_UNKNOWN", offset, output),
        };

        (offset, output)
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
