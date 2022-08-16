use core::fmt;
use std::{
    fmt::Display,
    ops::{Add, Deref, DerefMut},
    ptr::NonNull,
};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub(crate) struct ObjPointer {
    pub raw: NonNull<Obj>,
}

impl ObjPointer {
    pub(crate) fn new(object: Obj) -> Self {
        let boxed = Box::into_raw(Box::new(object));
        // Safety: object is always valid value, into_raw promises a well-aligned non-null pointer.
        unsafe {
            Self {
                raw: NonNull::new_unchecked(boxed),
            }
        }
    }

    pub fn as_ptr(&mut self) -> *mut Obj {
        self.raw.as_ptr()
    }
}

impl Deref for ObjPointer {
    type Target = Obj;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl DerefMut for ObjPointer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub(crate) enum Obj {
    String(String),
}

impl Obj {
    pub fn from_string(string: String) -> Self {
        Self::String(string)
    }
}

impl Add for &Obj {
    type Output = Obj;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Obj::String(ref left), Obj::String(ref right)) => Obj::String(left + right),
        }
    }
}

impl Display for Obj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Obj::String(object) => write!(f, "String(\"{}\")", object),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
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

impl Add for &String {
    type Output = String;

    fn add(self, rhs: Self) -> Self::Output {
        let mut new_chars = vec![];
        new_chars.extend_from_slice(&self.chars);
        new_chars.extend_from_slice(&rhs.chars);

        String {
            len: new_chars.len(),
            chars: new_chars,
        }
    }
}
