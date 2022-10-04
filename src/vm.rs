use std::collections::LinkedList;
use std::ops::Deref;
use std::rc::Rc;
use std::string::String;
use std::sync::Mutex;
use std::{fmt::Display, ptr, result};

use once_cell::sync::OnceCell;

use crate::bytecode::{Chunk, Disassembler, OpCode};
use crate::compiler::{Compiler, CompilerError, CompilerOptions};
use crate::object::{ManagedPtr, Object};
use crate::string::String as LoxString;
use crate::value::Value;
use rlox_common::{Array, HashMap, Stack};

pub fn heap() -> &'static Mutex<LinkedList<ManagedPtr<Object>>> {
    static HEAP: OnceCell<Mutex<LinkedList<ManagedPtr<Object>>>> = OnceCell::new();
    HEAP.get_or_init(|| {
        let mut heap = LinkedList::new();
        Mutex::new(heap)
    })
}

pub fn strings() -> &'static Mutex<HashMap<LoxString, ManagedPtr<Object>>> {
    static HEAP: OnceCell<Mutex<HashMap<LoxString, ManagedPtr<Object>>>> = OnceCell::new();
    HEAP.get_or_init(|| {
        let mut heap = HashMap::new();
        Mutex::new(heap)
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum VmError {
    Compile(CompilerError),
    Runtime(RuntimeError),
}

impl VmError {
    pub fn runtime(msg: &str, line: usize) -> Self {
        Self::Runtime(RuntimeError {
            msg: msg.to_string(),
            line,
        })
    }
}

impl From<CompilerError> for VmError {
    fn from(error: CompilerError) -> Self {
        VmError::Compile(error)
    }
}

impl From<RuntimeError> for VmError {
    fn from(error: RuntimeError) -> Self {
        VmError::Runtime(error)
    }
}

type InterpretResult = result::Result<Value, VmError>;

impl Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::Compile(error) => {
                write!(f, "[line: {}] compile error: {}", error.line(), error.msg())
            }
            VmError::Runtime(error) => {
                write!(f, "[line: {}] runtime error: {}", error.line(), error.msg())
            }
        }?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RuntimeError {
    msg: String,
    line: usize,
}

impl RuntimeError {
    pub fn msg(&self) -> &str {
        self.msg.as_ref()
    }

    pub fn line(&self) -> usize {
        self.line
    }
}

#[derive(Debug, Default)]
pub(crate) struct VmOptions {
    pub trace_execution: bool,
    pub compiler: CompilerOptions,
}

pub(crate) struct Vm {
    chunk: Option<Chunk>,
    source: Option<String>,
    ip: *mut u8,
    options: VmOptions,
    stack: Stack<Value>,
    globals: HashMap<LoxString, Value>,
    last: Value,
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
            globals: HashMap::new(),
            last: Value::Nil,
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
        let source = self.source.as_ref().unwrap().clone();
        let mut compiler = Compiler::new(Some(&self.options.compiler));
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
    fn read_short(&mut self) -> u16 {
        unsafe {
            let bytes = [*self.ip, *self.ip.add(1)];
            self.ip = self.ip.add(2);

            u16::from_ne_bytes(bytes)
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
    fn read_string(&mut self) -> LoxString {
        let string = self.read_constant();
        let string = string.as_obj().unwrap().as_string().unwrap();
        string.clone()
    }

    #[inline]
    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }

    #[inline]
    fn pop(&mut self) -> Value {
        let value = self.stack.pop().expect("empty stack");
        self.last = value;
        value
    }

    #[inline]
    fn peek(&mut self, distance: usize) -> Result<Value, RuntimeError> {
        match self.stack.peek(distance) {
            Some(value) => Ok(value.clone()),
            None => Err(RuntimeError {
                msg: format!("no value at distance: {} in the stack.", distance),
                line: self.current_line(),
            }),
        }
    }

    #[inline]
    fn reset_stack(&mut self) {
        self.stack.reset();
    }

    #[inline]
    fn current_instruction_offset(&self) -> usize {
        unsafe {
            self.ip
                .offset_from(self.chunk.as_ref().expect("chunk expected here.").ptr())
                as usize
        }
    }

    fn current_line(&self) -> usize {
        let instruction = self.current_instruction_offset();

        self.chunk.as_ref().unwrap().lines[instruction]
    }

    pub fn free_objects(&mut self) {
        let mut heap = heap().lock().unwrap();
        while let Some(mut object_ptr) = heap.pop_back() {
            let object = unsafe { Box::from_raw(object_ptr.as_ptr()) };
            drop(object);
        }
    }

    fn print_stack(&self) {
        println!("{}", self.stack);
    }

    fn dissasemble_current_instruction(&mut self) {
        let mut dissasembler = Disassembler::new(self.chunk.as_ref().unwrap(), "chunk");
        let mut output = String::new();

        let disassembled_instruction =
            dissasembler.disassemble_instruction(self.current_instruction_offset());

        print!("{}", disassembled_instruction)
    }

    #[inline]
    fn check_both_number(&mut self) -> Result<(), RuntimeError> {
        if let (Some(left), Some(right)) = (self.stack.peek(1), self.stack.peek(0)) {
            if !left.is_number() || !right.is_number() {
                return self.runtime_error("operand must be a number.");
            }
        } else {
            return self.runtime_error("missing operand.");
        }

        Ok(())
    }

    #[inline]
    fn check_number(&mut self) -> Result<(), RuntimeError> {
        if !self.stack.peek(0).unwrap().is_number() {
            return self.runtime_error("operand must be a number.");
        }

        Ok(())
    }

    fn vm_error(&mut self, message: &str) -> InterpretResult {
        let instruction = self.current_instruction_offset();

        self.reset_stack();

        Err(VmError::runtime(
            message,
            self.chunk.as_ref().unwrap().lines[instruction],
        ))
    }

    fn runtime_error(&mut self, message: &str) -> Result<(), RuntimeError> {
        let instruction = self.current_instruction_offset();

        self.reset_stack();

        Err(RuntimeError {
            msg: message.to_string(),
            line: self.chunk.as_ref().unwrap().lines[instruction],
        })
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        self.free_objects();
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
        let opcode: OpCode =
            OpCode::from_repr(byte).expect("internal error: cannot decode instruction.");

        match opcode {
            OpCode::Return => return Ok(vm.last),
            OpCode::AddConstant => {
                let constant = vm.read_constant();
                vm.push(constant);
            }
            OpCode::Negate => {
                vm.check_number()?;
                let negated = -vm.pop();
                vm.push(negated);
            }
            OpCode::Add => op_add(vm)?,
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
            OpCode::AddTrue => vm.push(Value::r#true()),
            OpCode::AddFalse => vm.push(Value::r#false()),
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
            OpCode::Print => println!("{}", vm.pop()),
            OpCode::Pop => {
                vm.pop();
            }
            OpCode::DefineGlobal => {
                let name = vm.read_string();
                let value = vm.peek(0)?;
                vm.globals.insert(name, value);
                vm.pop();
            }
            OpCode::GetGlobal => {
                let name = vm.read_string();
                match vm.globals.get(&name) {
                    Some(value) => vm.push(value.clone()),
                    None => return vm.vm_error(&format!("undefined variable '{}'.", name)),
                };
            }
            OpCode::SetGlobal => {
                let name = vm.read_string();
                let value = vm.peek(0)?;
                if vm.globals.insert(name.clone(), value) {
                    vm.globals.remove(&name);
                    return vm.vm_error(&format!("undefined variable '{}'.", name));
                }
            }
            OpCode::GetLocal => {
                let slot = vm.read_byte();
                vm.push(vm.stack[slot as usize].clone());
            }
            OpCode::SetLocal => {
                let slot = vm.read_byte();
                vm.stack[slot as usize] = vm.peek(0)?;
            }
            OpCode::JumpIfFalse => {
                let offset = vm.read_short();
                if vm.peek(0)?.is_falsey() {
                    unsafe { vm.ip = vm.ip.add(offset.into()) };
                }
            }
            OpCode::Jump => {
                let offset = vm.read_short();
                unsafe { vm.ip = vm.ip.add(offset.into()) };
            }
            OpCode::Loop => {
                let offset = vm.read_short();
                unsafe { vm.ip = vm.ip.sub(offset.into()) };
            }
        }
    }
}

#[inline(always)]
fn op_add(vm: &mut Vm) -> Result<(), RuntimeError> {
    let (left, right) = (vm.peek(1)?, vm.peek(0)?);
    if (left.is_number() && right.is_number()) || (left.is_string() && right.is_string()) {
        let right = vm.pop();
        let left = vm.pop();
        vm.push(left + right);
        Ok(())
    } else {
        vm.runtime_error("operands must be two numbers of two strings.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_add_two_numbers() {
        let mut vm = Vm::new(None);
        assert_eq!(
            Value::from(2.0),
            vm.interpret("1 + 1;".to_string()).unwrap()
        );
    }

    #[test]
    fn op_add_two_strings() {
        let mut vm = Vm::new(None);
        assert_eq!(
            Value::from("hello world!"),
            vm.interpret("\"hello\" + \" world!\";".to_string())
                .unwrap()
        );
    }

    #[test]
    fn op_add_type_both_number_error() {
        let mut vm = Vm::new(None);

        let expected_error = Err(VmError::Runtime(RuntimeError {
            msg: "operands must be two numbers of two strings.".into(),
            line: 1,
        }));

        assert_eq!(expected_error, vm.interpret("1 + \"1\";".to_string()));
    }

    #[test]
    fn op_add_type_both_number_error_2() {
        let mut vm = Vm::new(None);

        let expected_error = Err(VmError::Runtime(RuntimeError {
            msg: "operands must be two numbers of two strings.".into(),
            line: 1,
        }));

        assert_eq!(expected_error, vm.interpret("1 + nil;".to_string()));
    }

    #[test]
    fn undefinded_local_error() {
        let mut vm = Vm::new(None);

        let expected_error = Err(VmError::Runtime(RuntimeError {
            msg: "undefined variable 'a'.".into(),
            line: 1,
        }));

        assert_eq!(expected_error, vm.interpret("{ print a; }".to_string()));
    }
}
