use std::collections::LinkedList;
use std::fmt::{self, Debug, Display};
use std::mem::forget;
use std::ops::{Add, Deref, DerefMut, Div, Mul, Neg, Sub};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Mutex;

use crate::object::{Obj, ObjPointer, String};
use crate::vm;

// TODO: we don't know if we need this for our implementation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub enum ObjKind {
    String,
}

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Obj(ObjPointer),
}

impl Value {
    pub fn from_string(string: String) -> Self {
        Self::Obj(ObjPointer::new(Obj::from_string(string)))
    }

    pub fn from_obj(object_ptr: ObjPointer) -> Self {
        Self::Obj(object_ptr)
    }

    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn r#true() -> Self {
        Self::Boolean(true)
    }

    pub fn r#false() -> Self {
        Self::Boolean(false)
    }

    /// Returns `true` if the value is [`Number`].
    ///
    /// [`Number`]: Value::Number
    #[must_use]
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    /// Returns `true` if the value is [`Boolean`].
    ///
    /// [`Boolean`]: Value::Boolean
    #[must_use]
    #[inline]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(..))
    }

    /// Returns `true` if the value is [`Nil`].
    ///
    /// [`Nil`]: Value::Nil
    #[must_use]
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    #[must_use]
    #[inline]
    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Nil | Self::Boolean(false))
    }

    /// Returns `true` if the value is [`Obj`].
    ///
    /// [`Obj`]: Value::Obj
    #[must_use]
    #[inline]
    pub fn is_obj(&self) -> bool {
        matches!(self, Self::Obj(..))
    }

    #[must_use]
    #[inline]
    pub fn is_string(&self) -> bool {
        self.is_obj() && matches!(**self.as_obj().unwrap(), Obj::String(..))
    }

    #[must_use]
    #[inline]
    pub fn as_obj(&self) -> Option<&ObjPointer> {
        if let Self::Obj(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn as_number(&self) -> Option<&f64> {
        match self {
            Self::Number(v) => Some(v),
            _ => None,
        }
    }

    #[must_use]
    #[inline]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(inner) => write!(f, "{}", inner),
            Value::Boolean(inner) => write!(f, "{}", inner),
            Value::Nil => write!(f, "nil"),
            Value::Obj(obj) => write!(f, "{}", **obj),
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
            Value::Obj(_) => panic!("unsupported integer negation for objects"),
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
            (Value::Obj(left), Value::Obj(right)) => {
                let new_obj = vm::allocate_object(&*left + &*right);
                Value::Obj(new_obj)
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
            (Self::Obj(left), Self::Obj(right)) => left == right,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::Boolean(left), Self::Boolean(right)) => left.partial_cmp(right),
            (Self::Obj(left), Self::Obj(right)) => left.partial_cmp(right),
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
