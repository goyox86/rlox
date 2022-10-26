use std::any::Any;
use std::{collections::LinkedList, sync::Mutex};

use rlox_common::HashMap;

use crate::function::Function;
use crate::object::Handle;
use crate::string::String;

pub(crate) struct Heap {
    objects: Vec<Box<dyn Any>>,
    strings: HashMap<String, Handle<String>>,
}

impl Heap {
    pub(crate) fn new() -> Self {
        Self {
            /// List of object handles for "GC" (Not a thing yet)
            objects: Vec::new(),
            /// Interned strings
            strings: HashMap::new(),
        }
    }

    pub fn allocate<T: 'static>(&mut self, value: T) -> Handle<T> {
        let mut object_ptr = Handle::new(value);
        self.objects.push(Box::new(object_ptr.clone()));
        object_ptr
    }

    pub fn allocate_string(&mut self, string: String) -> Handle<String> {
        let new_string_handle = self.allocate(string.clone());

        match self.strings.get(&string) {
            Some(string_handle) => *string_handle,
            None => {
                self.strings.insert(string, new_string_handle);
                new_string_handle
            }
        }
    }
}

// The WAT?
impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            while let Some(mut boxed_handle) = self.objects.pop() {
                match boxed_handle.downcast_mut::<Handle<String>>() {
                    Some(string_handle) => {
                        let _ = Box::from_raw(string_handle.as_ptr());
                        continue;
                    }
                    None => {
                        if let Some(function_handle) =
                            boxed_handle.downcast_mut::<Handle<Function>>()
                        {
                            let _ = Box::from_raw(function_handle.as_ptr());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::function::Function;

    use super::*;

    #[test]
    fn store_multiple_types() {
        let mut heap = Heap::new();
        let mut s = heap.allocate(String::new("Yo!"));
        let mut f = heap.allocate(Function::new(None, None));
        let g = f;
        let h = g;

        println!("{}", *s);
        println!("{}", *f);
        println!("{}", *h);
        println!("{}", *g);
    }
}
