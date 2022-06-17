use std::{
    alloc::{alloc, dealloc, Layout},
    marker::PhantomData,
    ops::{Index, IndexMut},
    ptr::{self, NonNull},
};

#[derive(Debug, PartialEq)]
pub struct Array<T> {
    count: usize,
    capacity: usize,
    ptr: NonNull<T>,
    pub _marker: PhantomData<T>,
}

impl<T> Array<T> {
    pub fn new() -> Self {
        Self {
            count: 0,
            capacity: 0,
            ptr: NonNull::dangling(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if self.needs_to_grow() {
            self.grow_array();
        }

        unsafe { self.ptr.as_ptr().add(self.count).write(value) };
        self.count += 1;
    }

    pub fn write(&mut self, value: T) {
        self.push(value)
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        unsafe {
            self.count -= 1;
            let value: T = ptr::read(self.ptr.as_ptr().add(self.count));
            Some(value)
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            len: 0,
            array: self,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            len: 0,
            array: self,
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            unsafe { Some(&*self.ptr().add(index)) }
        } else {
            None
        }
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        &*self.ptr().add(index)
    }

    #[inline]
    pub fn get_mut<'a>(&mut self, index: usize) -> Option<&'a mut T> {
        if index < self.len() {
            unsafe { Some(&mut *self.ptr().add(index)) }
        } else {
            None
        }
    }

    #[inline]
    pub fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    fn current_memory(&self) -> Layout {
        Layout::array::<T>(self.capacity()).expect("failed to obtain memory layout")
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    fn grow_capacity(&mut self) -> usize {
        if self.capacity < 8 {
            8
        } else {
            self.capacity * 2
        }
    }

    fn needs_to_grow(&self) -> bool {
        self.capacity < self.count + 1
    }

    fn grow_array(&mut self) {
        let old_ptr = self.ptr;
        let old_capacity = self.capacity();
        let old_layout = self.current_memory();
        self.capacity = self.grow_capacity();
        let new_layout = self.current_memory();

        unsafe {
            let new_ptr = alloc(new_layout);
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

impl<T> Index<usize> for Array<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "index out of bounds: len is {} but index is {}",
                self.len(),
                index
            )
        })
    }
}

impl<T> IndexMut<usize> for Array<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.len();
        self.get_mut(index)
            .unwrap_or_else(|| panic!("index out of bounds: len is {} but index is {}", len, index))
    }
}

impl<T> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.count)) };
    }
}

pub struct Iter<'a, T> {
    array: &'a Array<T>,
    len: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.array.get(self.len);
        self.len += 1;
        next
    }
}

pub struct IterMut<'a, T: 'a> {
    array: &'a mut Array<T>,
    len: usize,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        let next = self.array.get_mut(self.len);
        self.len += 1;
        next
    }
}

unsafe impl<T> Send for Array<T> {}

#[cfg(test)]
mod tests {

    use super::*;

    #[allow(dead_code)]
    #[derive(Debug, PartialEq)]
    struct Foo {
        bar: usize,
        baz: String,
    }

    impl Foo {
        fn new(bar: usize, baz: String) -> Self {
            Self { bar, baz }
        }
    }

    #[test]
    fn test_push_when_its_empty() {
        let mut array: Array<Foo> = Array::new();
        array.push(Foo::new(1, "lol".to_string()));

        assert_eq!(Some(Foo::new(1, "lol".to_string())), array.pop());
    }

    #[test]
    fn test_push_when_its_not_empty() {
        let mut array: Array<Foo> = Array::new();
        array.push(Foo::new(1, "lol".to_string()));

        array.push(Foo::new(2, "wat".to_string()));
        assert_eq!(Some(Foo::new(2, "wat".to_string())), array.pop());
    }

    #[test]
    fn test_push_when_its_not_empty_lots() {
        let mut array: Array<Foo> = Array::new();
        // Forcing to cause resizes
        for i in 0..128 {
            array.push(Foo::new(i, format!("I am {}", i)));
        }

        array.push(Foo::new(2, "wat".to_string()));
        assert_eq!(Some(Foo::new(2, "wat".to_string())), array.pop());
    }

    #[test]
    fn test_is_empty_when_is_empty() {
        let array: Array<Foo> = Array::new();

        assert_eq!(true, array.is_empty())
    }

    #[test]
    fn test_is_empty_when_is_not_empty() {
        let mut array: Array<Foo> = Array::new();
        array.push(Foo::new(1, "lol".to_string()));

        assert_eq!(false, array.is_empty())
    }

    #[test]
    fn test_pop_none_when_its_empty() {
        let mut array: Array<usize> = Array::new();
        assert_eq!(true, array.is_empty());
        assert_eq!(array.pop(), None);
    }

    #[test]
    fn test_pop_when_not_empty() {
        let mut array: Array<Foo> = Array::new();
        array.push(Foo::new(1, "lol".to_string()));

        assert_eq!(Some(Foo::new(1, "lol".to_string())), array.pop());
    }

    #[test]
    #[should_panic(expected = "index out of bounds: len is 0 but index is 2")]
    fn test_index_when_empty() {
        let array: Array<String> = Array::new();

        assert_eq!(&"Yooo".to_string(), &array[2]) // let _ = deque[0];
    }

    #[test]
    fn test_index_when_not_empty() {
        let mut array: Array<String> = Array::new();
        array.push("Yooo".to_string());

        assert_eq!(&"Yooo".to_string(), &array[0]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: len is 0 but index is 1")]
    fn test_index_mut_when_empty() {
        let mut array: Array<String> = Array::new();
        let a = &mut array[1];

        *a = "swapped".to_string();
    }

    #[test]
    fn test_index_mut_when_not_empty() {
        let mut array: Array<String> = Array::new();
        array.push("Yooo".to_string());
        let a = &mut array[0];
        *a = "swapped".to_string();

        assert_eq!(&"swapped".to_string(), &mut array[0]) // let _ = deque[0];
    }

    #[test]
    fn test_default() {
        let array: Array<String> = Array::new();
        assert_eq!(Array::default(), array); // let _ = deque[0];
    }

    #[test]
    fn test_drop() {
        use drop_tracker::DropTracker;
        let mut tracker = DropTracker::new();

        let mut array = Array::new();

        array.push(tracker.track(1));
        array.push(tracker.track(2));
        array.push(tracker.track(3));

        // Assert that all elements in the vector are alive
        tracker
            .all_alive(1..=3)
            .expect("expected all elements to be alive");

        // Once the vector is dropped, all items should be dropped with it
        drop(array);
        tracker
            .all_dropped(1..=3)
            .expect("expected all elements to be dropped");
    }
}
