use std::any::Any;
use std::{collections::LinkedList, sync::Mutex};

use rlox_common::HashMap;

use crate::object::Handle;
use crate::string::String;

pub(crate) trait Gc {
    fn collect(&mut self);
}

pub(crate) struct Heap {
    objects: Vec<Box<dyn Gc>>,
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
            Some(string_ptr) => *string_ptr,
            None => {
                self.strings.insert(string, new_string_handle);
                new_string_handle
            }
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        while let Some(mut handle) = self.objects.pop() {
            handle.collect();
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
        let s = heap.allocate(String::new("Yo!"));
        let f = heap.allocate(Function::new(None, None));
        println!("{}", *s);
    }
}
