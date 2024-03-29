use std::collections::LinkedList;
use std::fmt::{self, Debug, Display};
use std::mem::forget;
use std::ops::{Add, Deref, DerefMut, Div, Mul, Neg, Sub};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Mutex;

use crate::string::String;
use crate::vm::{self, HEAP};
use crate::{function::Function, object::Handle};

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    String(Handle<String>),
    Function(Handle<Function>),
}

impl Value {
    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn r#true() -> Self {
        Self::Boolean(true)
    }

    pub fn r#false() -> Self {
        Self::Boolean(false)
    }

    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    #[inline]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(..))
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    #[inline]
    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Nil | Self::Boolean(false))
    }

    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    #[inline]
    pub fn as_number(&self) -> Option<&f64> {
        match self {
            Self::Number(v) => Some(v),
            _ => None,
        }
    }

    #[inline]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&Handle<String>> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(inner) => write!(f, "{}", inner),
            Value::Boolean(inner) => write!(f, "{}", inner),
            Value::Nil => write!(f, "nil"),
            Value::String(obj) => write!(f, "{}", **obj),
            Value::Function(function) => write!(f, "{}", **function),
        }
    }
}

impl Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Self::Number(number) => Self::Number(-number),
            Value::Boolean(_) => panic!("unsupported integer negation for booleans"),
            Value::Nil => panic!("unsupported integer negation for Nil"),
            Value::String(_) => panic!("unsupported integer negation for string objects"),
            Value::Function(_) => panic!("unsupported integer negation for function objects"),
        }
    }
}

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(number), Value::Number(rhs_number)) => {
                Value::Number(number + rhs_number)
            }
            (Value::String(left), Value::String(right)) => {
                let new_obj = &*left + &*right;
                Value::from(new_obj)
            }
            (left, right) => panic!("unsupported addition between {} and {}", left, right),
        }
    }
}

impl Sub for Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(number), Value::Number(rhs_number)) => {
                Value::Number(number - rhs_number)
            }
            (left, right) => panic!("unsupported substraction between {} and {}", left, right),
        }
    }
}

impl Div for Value {
    type Output = Value;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(number), Value::Number(rhs_number)) => {
                Value::Number(number / rhs_number)
            }
            (left, right) => panic!("unsupported division between {} and {}", left, right),
        }
    }
}

impl Mul for Value {
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(number), Value::Number(rhs_number)) => {
                Value::Number(number * rhs_number)
            }
            (left, right) => panic!("unsupported multiplication between {} and {}", left, right),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::Boolean(left), Self::Boolean(right)) => left.partial_cmp(right),
            (Self::String(left), Self::String(right)) => left.partial_cmp(right),
            (left, right) => left.partial_cmp(right),
        }
    }
}

impl Eq for Value {}

impl From<f64> for Value {
    fn from(inner: f64) -> Self {
        Self::Number(inner)
    }
}

impl From<bool> for Value {
    fn from(inner: bool) -> Self {
        Self::Boolean(inner)
    }
}

impl From<String> for Value {
    fn from(string_obj: String) -> Self {
        let string_handle = HEAP.with(|heap| heap.borrow_mut().allocate_string(string_obj));
        Self::String(string_handle)
    }
}

impl From<&str> for Value {
    fn from(string: &str) -> Self {
        let string_handle =
            HEAP.with(|heap| heap.borrow_mut().allocate_string(String::new(string)));
        Self::String(string_handle)
    }
}
