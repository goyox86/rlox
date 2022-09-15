use core::fmt;
use std::{
    borrow::Borrow,
    fmt::Display,
    ops::{Add, Deref, DerefMut},
    ptr::NonNull,
    string::String as RustString,
};

use crate::{
    value::Value,
    vm::{heap, strings},
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ManagedPtr<T> {
    pub raw: NonNull<T>,
}

impl<T: Clone> Copy for ManagedPtr<T> {}
// TODO: Hey, not sure about this
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
pub enum Object {
    String(String),
}

impl Object {
    pub fn allocate(value: Object) -> ManagedPtr<Object> {
        let mut object_ptr = ManagedPtr::new(value);
        let mut heap = heap().lock().unwrap();
        heap.push_back(object_ptr);
        object_ptr
    }

    pub fn allocate_string(string: String) -> ManagedPtr<Object> {
        let mut strings = strings().lock().unwrap();
        match strings.get(&string) {
            Some(string_ptr) => *string_ptr,
            None => {
                let string_ptr = Object::allocate(Object::String(string.clone()));
                strings.insert(string, string_ptr);
                string_ptr
            }
        }
    }

    pub fn from_string(string: &str) -> Self {
        Self::String(String::new(string))
    }

    #[must_use]
    pub(crate) fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    pub(crate) fn as_string(&self) -> Option<&String> {
        let Self::String(string) = self;
        Some(string)
    }
}

impl Add for &Object {
    type Output = Object;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::String(left), Object::String(right)) => Object::String(left + right),
        }
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::String(object) => write!(f, "String(\"{}\")", object),
        }
    }
}

impl From<Object> for Value {
    fn from(obj: Object) -> Self {
        Self::Obj(Object::allocate(obj))
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

impl Add for &String {
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
