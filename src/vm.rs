use std::{ptr, result};

use crate::{
    bytecode::{self, Chunk, OpCode},
    value::Value,
};

#[derive(Debug)]
pub(crate) struct Options {
    pub trace_execution: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            trace_execution: false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Vm {
    chunk: Chunk,
    ip: *mut u8,
    options: Options,
    stack: Vec<Value>,
}

impl Vm {
    pub fn new(chunk: Chunk, options: Option<Options>) -> Self {
        println!("{:?}", options);

        Self {
            chunk,
            ip: ptr::null_mut(),
            stack: Vec::new(),
            options: options.unwrap_or_default(),
        }
    }

    pub fn interpret(&mut self) -> InterpretResult {
        match self.chunk.code.get_mut(0) {
            Some(ip) => {
                self.ip = ip;
                self.run()
            }
            None => Err(Error::Runtime("empty bytecode chunk".into())),
        }
    }

    fn run(&mut self) -> InterpretResult {
        debug_assert!(!self.ip.is_null());

        loop {
            if self.options.trace_execution {
                self.print_stack();
                self.dissasemble_current_instruction();
            }

            let byte = self.read_byte();

            match OpCode::from(byte) {
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
            self.chunk
                .constants
                .get_unchecked(const_index_byte.into())
                .clone()
        }
    }

    fn dissasemble_current_instruction(&self) {
        let dissasembler = bytecode::Disassembler::new(&self.chunk, "chunk");
        let mut output = String::new();

        let (_, disassembled_instruction) = dissasembler.disassemble_instruction(
            unsafe { self.ip.offset_from(self.chunk.ptr()) as usize },
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

pub enum Error {
    Compile(String),
    Runtime(String),
}

type InterpretResult = result::Result<(), Error>;
