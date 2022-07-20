use std::{fmt::Display, ptr, result};

use rlox_common::{array::Array, stack::Stack, value::Value};
use rlox_compiler::{
    bytecode::{Chunk, OpCode},
    compiler::{Compiler, CompilerError, CompilerOptions},
    scanner::Scanner,
};

#[derive(Debug)]
pub(crate) struct VmOptions {
    pub trace_execution: bool,
    pub compiler: CompilerOptions,
}

impl Default for VmOptions {
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
    options: VmOptions,
    stack: Stack<Value>,
}

impl Vm {
    pub fn new(options: Option<VmOptions>) -> Self {
        let options = options.unwrap_or_default();

        Self {
            chunk: None,
            ip: ptr::null_mut(),
            stack: Stack::new(),
            options,
            source: None,
        }
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        self.source = Some(source);
        let chunk = self.compile()?;
        let ip_start = chunk.start();
        self.chunk = Some(chunk);
        self.ip = ip_start;

        run(self)
    }

    pub fn compile(&mut self) -> Result<Chunk, VmError> {
        let source = self.source.as_ref().unwrap();
        let mut compiler = Compiler::new(&self.options.compiler);
        let chunk = compiler.compile(source)?;

        Ok(chunk)
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
                .as_ref()
                .expect("chunk expected here")
                .constants
                .get_unchecked(const_index_byte.into())
                .clone()
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
    fn reset_stack(&mut self) {
        self.stack.reset();
    }

    #[inline]
    fn current_instruction_offset(&self) -> usize {
        unsafe {
            self.ip
                .offset_from(self.chunk.as_ref().expect("chunk expected here").ptr())
                as usize
        }
    }

    fn print_stack(&self) {
        println!("{}", self.stack);
    }

    fn dissasemble_current_instruction(&mut self) {
        let dissasembler =
            rlox_compiler::bytecode::Disassembler::new(self.chunk.as_ref().unwrap(), "chunk");
        let mut output = String::new();

        let (_, disassembled_instruction) =
            dissasembler.disassemble_instruction(self.current_instruction_offset(), &mut output);

        print!("{}", disassembled_instruction)
    }

    #[inline(always)]
    fn check_both_number(&mut self) -> InterpretResult {
        if let (Some(left), Some(right)) = (self.stack.peek(1), self.stack.peek(0)) {
            if !left.is_number() || !right.is_number() {
                return self.runtime_error("operand must be a number.");
            }
        } else {
            return self.runtime_error("missing operand.");
        }

        Ok(())
    }

    #[inline(always)]
    fn check_number(&mut self) -> InterpretResult {
        if !self.stack.peek(0).unwrap().is_number() {
            return self.runtime_error("operand must be a number.");
        }

        Ok(())
    }

    fn runtime_error(&mut self, message: &str) -> InterpretResult {
        let instruction = self.current_instruction_offset();

        self.reset_stack();

        Err(VmError::runtime(
            message,
            self.chunk.as_ref().unwrap().lines[instruction],
        ))
    }
}

fn run(vm: &mut Vm) -> InterpretResult {
    debug_assert!(!vm.ip.is_null());

    // vm.reset_stack();

    loop {
        if vm.options.trace_execution {
            vm.print_stack();
            vm.dissasemble_current_instruction();
        }

        let byte: u8 = vm.read_byte();
        let opcode: OpCode = OpCode::from_repr(byte).expect("cannot decode instruction");

        match opcode {
            OpCode::Return => {
                println!("{}", vm.pop());
                return Ok(());
            }
            OpCode::AddConstant => {
                let constant = vm.read_constant();
                vm.push(constant);
            }
            OpCode::Negate => {
                vm.check_number()?;
                let negated = -vm.pop();
                vm.push(negated);
            }
            OpCode::Add => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(left + right);
            }
            OpCode::Substract => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(left - right);
            }
            OpCode::Multiply => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(left * right);
            }
            OpCode::Divide => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(left / right);
            }
            OpCode::AddNil => vm.push(Value::Nil),
            OpCode::AddTrue => vm.push(Value::Boolean(true)),
            OpCode::AddFalse => vm.push(Value::Boolean(false)),
            OpCode::Not => {
                let value = vm.pop();
                vm.push(Value::Boolean(value.is_falsey()))
            }
            OpCode::Equal => {
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::Boolean(left == right))
            }
            OpCode::Greater => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::Boolean(left > right));
            }
            OpCode::Less => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::Boolean(left < right));
            }
        }
    }
}

type InterpretResult = result::Result<(), VmError>;

#[derive(Debug)]
pub enum VmError {
    Compile(CompilerError),
    Runtime(String, usize),
}

impl VmError {
    pub fn runtime(message: &str, line: usize) -> Self {
        Self::Runtime(message.into(), line)
    }
}

impl From<CompilerError> for VmError {
    fn from(error: CompilerError) -> Self {
        VmError::Compile(error)
    }
}
