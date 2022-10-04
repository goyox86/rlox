use std::{
    borrow::Borrow,
    fmt::{self, Debug, Display},
    ops::{Add, Deref, DerefMut},
    ptr::NonNull,
    string::String as RustString,
};

use crate::{
    string::String,
    value::Value,
    vm::{heap, strings},
};

/// A copyable pointer to values in the Lox heap.
#[derive(Clone, Eq, PartialOrd, Ord)]
pub struct ManagedPtr<T> {
    pub raw: NonNull<T>,
}

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

impl<T: PartialEq> PartialEq for ManagedPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
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

impl<T: Clone> Copy for ManagedPtr<T> {}
unsafe impl<T> Sync for ManagedPtr<T> where T: Sync {}
unsafe impl<T> Send for ManagedPtr<T> where T: Send {}

impl<T: Debug> Debug for ManagedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagedPtr")
            .field("raw", &self.raw)
            .field("value", unsafe { self.raw.as_ref() })
            .finish()
    }
}

/// A Lox object allocated in the Lox heap.
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
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    pub fn as_string(&self) -> Option<&String> {
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
            Object::String(object) => write!(f, "{}", object),
        }
    }
}

impl From<Object> for Value {
    fn from(obj: Object) -> Self {
        Self::Obj(Object::allocate(obj))
    }
}

impl From<String> for Value {
    fn from(string_obj: String) -> Self {
        Self::Obj(Object::allocate_string(string_obj))
    }
}
