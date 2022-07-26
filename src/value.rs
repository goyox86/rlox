use std::fmt::{self, Debug, Display};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub(crate) struct Obj {
    inner: ObjInner,
}

impl Add for Obj {
    type Output = Obj;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            inner: self.inner + rhs.inner,
        }
    }
}

impl Obj {
    pub fn from_string(string_obj: String) -> Box<Self> {
        Box::new(Self {
            inner: ObjInner::String(string_obj),
        })
    }
}

impl Display for Obj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum ObjInner {
    String(String),
}

impl Add for ObjInner {
    type Output = ObjInner;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (ObjInner::String(left), ObjInner::String(right)) => ObjInner::String(left + right),
        }
    }
}

impl Display for ObjInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjInner::String(object) => writeln!(f, "String(\"{}\")", object),
        }
    }
}

// TODO: we don't know if we need this for our implementation
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum ObjKind {
    String,
}

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Obj(Box<Obj>),
}

impl Value {
    pub fn from_string(string_obj: String) -> Self {
        Self::Obj(Obj::from_string(string_obj))
    }

    pub fn as_number(&self) -> Option<&f64> {
        match self {
            Self::Number(v) => return Some(v),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => return Some(*v),
            _ => None,
        }
    }

    pub fn nil() -> Self {
        Self::Nil
    }

    /// Returns `true` if the value is [`Number`].
    ///
    /// [`Number`]: Value::Number
    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    /// Returns `true` if the value is [`Boolean`].
    ///
    /// [`Boolean`]: Value::Boolean
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(..))
    }

    /// Returns `true` if the value is [`Nil`].
    ///
    /// [`Nil`]: Value::Nil
    #[must_use]
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    #[must_use]
    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Nil | Self::Boolean(false))
    }

    /// Returns `true` if the value is [`Obj`].
    ///
    /// [`Obj`]: Value::Obj
    #[must_use]
    pub fn is_obj(&self) -> bool {
        matches!(self, Self::Obj(..))
    }

    /// Returns `true` if the value is [`String`].
    ///
    /// [`Obj`]: Value::Obj
    #[must_use]
    pub fn is_string(&self) -> bool {
        self.is_obj()
            && matches!(
                **self.as_obj().unwrap(),
                Obj {
                    inner: ObjInner::String(..)
                }
            )
    }

    pub fn as_obj(&self) -> Option<&Box<Obj>> {
        if let Self::Obj(v) = self {
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
            Value::Obj(obj) => write!(f, "{}", *obj),
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
            (Value::Obj(ref left), Value::Obj(ref right)) => {
                Value::Obj(Box::new(*left.clone() + *right.clone()))
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
            (left, right) => panic!("unsupporte division between {} and {}", left, right),
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

#[repr(C)]
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub(crate) struct String {
    pub len: usize,
    pub chars: Vec<char>,
}

impl String {
    pub fn new(chars: &str) -> Self {
        Self {
            len: chars.len(),
            chars: chars.to_owned().chars().collect(),
        }
    }
}

impl Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.chars.iter() {
            write!(f, "{}", c)?;
        }

        Ok(())
    }
}

impl Add for String {
    type Output = String;

    fn add(self, rhs: Self) -> Self::Output {
        let mut new_chars = vec![];
        new_chars.extend_from_slice(&self.chars);
        new_chars.extend_from_slice(&rhs.chars);

        Self {
            len: new_chars.len(),
            chars: new_chars,
        }
    }
}
