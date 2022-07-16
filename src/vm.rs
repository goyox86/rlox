use std::{ptr, result};

use rlox_common::{array::Array, value::Value};
use rlox_compiler::{
    bytecode::{Chunk, OpCode},
    compiler::{Compiler, CompilerError, CompilerOptions},
    scanner::Scanner,
};

#[derive(Debug)]
pub(crate) struct Options {
    pub trace_execution: bool,
    pub compiler: CompilerOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            trace_execution: false,
            compiler: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Vm {
    chunk: Option<Chunk>,
    source: Option<String>,
    ip: *mut u8,
    options: Options,
    stack: Vec<Value>,
}

impl Vm {
    pub fn new(options: Option<Options>) -> Self {
        let options = options.unwrap_or_default();

        Self {
            chunk: None,
            ip: ptr::null_mut(),
            stack: Vec::new(),
            options,
            source: None,
        }
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        self.source = Some(source);

        self.chunk = Some(self.compile()?);

        let chunk = self
            .chunk_mut()
            .ok_or(VmError::Runtime("no bytecode chunk".into()))?;

        match chunk.code.get_mut(0) {
            Some(ip) => {
                self.ip = ip;
                self.run()
            }
            None => Err(VmError::Runtime("empty bytecode chunk".into())),
        }
    }

    pub fn chunk(&self) -> Option<&Chunk> {
        self.chunk.as_ref()
    }

    pub fn chunk_mut(&mut self) -> Option<&mut Chunk> {
        self.chunk.as_mut()
    }

    pub fn compile(&mut self) -> Result<Chunk, VmError> {
        let source = self.source.as_ref().unwrap();
        let mut compiler = Compiler::new(&self.options.compiler);
        let chunk = compiler.compile(source)?;

        Ok(chunk)
    }

    fn run(&mut self) -> InterpretResult {
        debug_assert!(!self.ip.is_null());

        loop {
            if self.options.trace_execution {
                self.print_stack();
                self.dissasemble_current_instruction();
            }

            let byte: u8 = self.read_byte();
            let opcode: OpCode = OpCode::from_repr(byte).expect("cannot decode instruction");

            match opcode {
                OpCode::Return => {
                    println!("{}", self.pop());
                    return Ok(());
                }
                OpCode::AddConstant => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                OpCode::Negate => {
                    let negated = -self.pop();
                    self.push(negated);
                }
                OpCode::Add => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(left + right);
                }
                OpCode::Substract => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(left - right);
                }
                OpCode::Multiply => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(left * right);
                }
                OpCode::Divide => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(left / right);
                }
            }
        }
    }

    #[inline]
    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }

    #[inline]
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("empty stack")
    }

    #[inline]
    fn read_byte(&mut self) -> u8 {
        unsafe {
            let byte = *self.ip;
            self.ip = self.ip.add(1);
            byte
        }
    }

    #[inline]
    fn read_constant(&mut self) -> Value {
        let const_index_byte = self.read_byte();
        unsafe {
            self.chunk_mut()
                .expect("chunk expected here")
                .constants
                .get_unchecked(const_index_byte.into())
                .clone()
        }
    }

    fn dissasemble_current_instruction(&self) {
        let dissasembler =
            rlox_compiler::bytecode::Disassembler::new(&self.chunk().unwrap(), "chunk");
        let mut output = String::new();

        let (_, disassembled_instruction) = dissasembler.disassemble_instruction(
            unsafe {
                self.ip
                    .offset_from(self.chunk().expect("chunk expected here").ptr())
                    as usize
            },
            &mut output,
        );

        print!("{}", disassembled_instruction)
    }

    fn print_stack(&self) {
        for elem in &self.stack {
            println!("[ {} ]", elem);
        }
        println!("");
    }
}

#[derive(Debug)]
pub enum VmError {
    Compile(CompilerError),
    Runtime(String),
}

type InterpretResult = result::Result<(), VmError>;

impl From<CompilerError> for VmError {
    fn from(error: CompilerError) -> Self {
        VmError::Compile(error)
    }
}
