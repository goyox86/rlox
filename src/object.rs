use core::fmt;
use std::{
    fmt::Display,
    ops::{Add, Deref, DerefMut},
    ptr::NonNull,
    string::String as RustString,
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ManagedPtr<T> {
    pub raw: NonNull<T>,
}

impl<T: Clone> Copy for ManagedPtr<T> {}

// TODO: Hey, not sure bout theze
unsafe impl<T> Sync for ManagedPtr<T> where T: Sync {}
unsafe impl<T> Send for ManagedPtr<T> where T: Send {}

impl<T> ManagedPtr<T> {
    pub(crate) fn new(object: T) -> Self {
        let boxed = Box::into_raw(Box::new(object));
        // Safety: object is always valid value, into_raw promises a well-aligned non-null pointer.
        unsafe {
            Self {
                raw: NonNull::new_unchecked(boxed),
            }
        }
    }

    pub fn as_ptr(&mut self) -> *mut T {
        self.raw.as_ptr()
    }
}

impl<T> Deref for ManagedPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl<T> DerefMut for ManagedPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Obj {
    String(String),
}

impl Obj {
    pub fn from_string(string: String) -> Self {
        Self::String(string)
    }

    #[must_use]
    pub(crate) fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    pub(crate) fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Add for Obj {
    type Output = Obj;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Obj::String(left), Obj::String(right)) => Obj::String(left + right),
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct String {
    inner: std::string::String,
}

impl String {
    pub fn new(chars: &str) -> Self {
        Self {
            inner: RustString::from(chars),
        }
    }
}

impl Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        Ok(())
    }
}

impl Add for String {
    type Output = String;

    fn add(self, rhs: Self) -> Self::Output {
        String {
            inner: format!("{}{}", self.inner, rhs.inner),
        }
    }
}

impl Deref for String {
    type Target = RustString;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
