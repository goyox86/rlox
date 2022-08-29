use std::{
    alloc::{alloc, alloc_zeroed, dealloc, Layout},
    marker::PhantomData,
    ptr::{self, NonNull},
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct RawArray<T> {
    capacity: usize,
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> RawArray<T> {
    pub fn new() -> Self {
        let ptr = NonNull::dangling();

        Self {
            capacity: 0,
            ptr,
            _marker: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut me = Self::new();
        me.grow(Some(capacity));
        me
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    #[inline]
    fn layout_for(&self, capacity: usize) -> Layout {
        Layout::array::<T>(capacity).expect("failed to obtain memory layout")
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    fn grow_capacity(&mut self) -> usize {
        if self.capacity < 8 {
            8
        } else {
            self.capacity * 2
        }
    }

    pub fn grow(&mut self, capacity: Option<usize>) {
        let old_ptr = self.ptr;
        let old_layout = self.layout_for(self.capacity);
        let old_capacity = self.capacity;
        self.capacity = capacity.unwrap_or(self.grow_capacity());
        let new_layout = self.layout_for(self.capacity);

        unsafe {
            // TODO: Check if there is a way of getting this without zeroing.
            let new_ptr = alloc_zeroed(new_layout);
            self.ptr = NonNull::new_unchecked(new_ptr.cast());
            ptr::copy(
                old_ptr.as_ptr() as *const T,
                self.ptr.as_ptr().cast(),
                old_capacity,
            );

            if old_capacity > 0 {
                dealloc(old_ptr.as_ptr().cast(), old_layout);
            }
        }
    }
}

impl<T> Default for RawArray<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Drop for RawArray<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

unsafe impl<T> Send for RawArray<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(Debug, PartialEq)]
    struct Foo {
        bar: usize,
        baz: String,
    }

    #[test]
    fn test_init() {
        let array: RawArray<Foo> = RawArray::new();
    }
}