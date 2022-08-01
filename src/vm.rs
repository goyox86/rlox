use std::collections::LinkedList;
use std::rc::Rc;
use std::string::String;
use std::sync::Mutex;
use std::{fmt::Display, ptr, result};

use rlox_common::{Array, Stack};

use crate::bytecode::{Chunk, Disassembler, OpCode};
use crate::compiler::{Compiler, CompilerError, CompilerOptions};
use crate::value::{Obj, ObjPointer, String as LoxString, Value};

pub(crate) static mut HEAP: Option<Vec<*mut Obj>> = None;

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

pub(crate) fn allocate_object(object: Obj) -> ObjPointer {
    let mut object = ObjPointer::new(object);
    unsafe {
        if let Some(heap) = &mut HEAP {
            heap.push(object.as_ptr());
        } else {
            let mut heap = Vec::new();
            heap.push(object.as_ptr());
            HEAP = Some(heap);
        }

        object
    }
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

        let result = run(self);
        self.free_objects();
        result
    }

    pub fn compile(&mut self) -> Result<Chunk, VmError> {
        let source = self.source.as_ref().unwrap().clone();
        let mut compiler = Compiler::new(None);
        let chunk = compiler.compile(&source)?;

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

    pub fn free_objects(&mut self) {
        unsafe {
            if let Some(heap) = &mut HEAP {
                while let Some(object) = heap.pop() {
                    let object = unsafe { Box::from_raw(object) };
                    drop(object);
                }
            }
        }
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
        let dissasembler = Disassembler::new(self.chunk.as_ref().unwrap(), "chunk");
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
                if let (Some(left), Some(right)) = (vm.stack.peek(1), vm.stack.peek(0)) {
                    if (left.is_number() && right.is_number())
                        || (left.is_string() && right.is_string())
                    {
                        let right = vm.pop();
                        let left = vm.pop();
                        vm.push(left + right);
                    } else {
                        return vm.runtime_error("operands must be two numbers of two strings.");
                    }
                }
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
                vm.push(Value::from(value.is_falsey()))
            }
            OpCode::Equal => {
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::from(left == right))
            }
            OpCode::Greater => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::from(left > right));
            }
            OpCode::Less => {
                vm.check_both_number()?;
                let right = vm.pop();
                let left = vm.pop();
                vm.push(Value::from(left < right));
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

enum OperandsCheck {
    Pass,
    LeftWrongType,
    RightWrongType,
    Missing,
}
