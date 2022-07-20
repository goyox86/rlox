use core::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Debug, PartialOrd)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
}

impl Value {
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
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(inner) => write!(f, "{}", inner),
            Value::Boolean(inner) => write!(f, "{}", inner),
            Value::Nil => write!(f, "nil"),
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
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
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
