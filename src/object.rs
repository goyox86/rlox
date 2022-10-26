use std::{
    borrow::Borrow,
    fmt::{self, Debug, Display},
    ops::{Add, Deref, DerefMut},
    ptr::NonNull,
    string::String as RustString,
};

use crate::{string::String, value::Value, vm::HEAP};

/// A pointer to values in the Lox heap.
pub struct Handle<T> {
    raw: NonNull<T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(object: T) -> Self {
        let boxed = Box::into_raw(Box::new(object));
        // Safety: object is always valid value, into_raw promises a well-aligned non-null pointer.
        unsafe {
            Self {
                raw: NonNull::new_unchecked(boxed),
            }
        }
    }

    pub unsafe fn as_ptr(&mut self) -> *mut T {
        self.raw.as_ptr()
    }
}

impl<T: PartialEq> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.as_ref() }
    }
}

impl<T> DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.as_mut() }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self { raw: self.raw }
    }
}

impl<T: Clone> Copy for Handle<T> {}
unsafe impl<T> Sync for Handle<T> where T: Sync {}
unsafe impl<T> Send for Handle<T> where T: Send {}

impl<T: Debug> Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("raw", &self.raw)
            .field("value", unsafe { self.raw.as_ref() })
            .finish()
    }
}
