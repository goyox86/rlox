use std::{borrow::Borrow, collections::hash_map::DefaultHasher, hash::Hash, hash::Hasher};

use crate::raw_array::RawArray;

const MAX_LOAD: f32 = 0.75;
#[derive(Debug)]
pub struct HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    entries: RawArray<Entry<K, V>>,
    len: usize,
}

#[allow(dead_code)]
impl<K, V> HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: K, value: V) -> bool {
        if self.needs_to_grow() {
            self.entries.grow(None);
        }

        let index = (self.hash(&key) as usize) % self.entries.capacity();

        if self.find_entry(index).is_vacant() {
            self.set_entry(index, Entry::Occupied { key, value });
            self.len += 1;
            return true;
        }

        while let Entry::Occupied {
            key: ekey,
            value: _,
        } = &mut self.find_entry(index)
        {
            if ekey == &key {
                self.set_entry(index, Entry::Occupied { key, value });
                break;
            }
        }

        true
    }

    pub fn get(&mut self, key: K) -> Option<&V> {
        let index = (self.hash(&key) as usize) % self.entries.capacity();

        match self.find_entry(index) {
            Entry::Vacant => None,
            Entry::Occupied { key: ekey, value } => {
                return if ekey == key.borrow() {
                    Some(&value)
                } else {
                    None
                }
            }
        }
    }

    fn hash(&self, key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    #[inline]
    fn find_entry(&self, index: usize) -> &Entry<K, V> {
        assert!(
            index < self.entries.capacity(),
            "index out of bounds on buckets array"
        );

        unsafe { &*self.entries.as_ptr().add(index) }
    }

    #[inline]
    pub fn set_entry(&mut self, index: usize, value: Entry<K, V>) {
        if self.needs_to_grow() {
            self.entries.grow(None);
        }

        assert!(
            index < self.entries.capacity(),
            "index out of bounds on buckets array"
        );

        unsafe { self.entries.as_ptr().add(index).write(value) };
    }

    pub(crate) fn needs_to_grow(&self) -> bool {
        self.len + 1 > (self.entries.capacity() as f32 * MAX_LOAD) as usize
    }
}

#[derive(Debug)]
pub enum Entry<K, V> {
    Vacant,
    Occupied { key: K, value: V },
}

impl<K, V> Default for HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    fn default() -> Self {
        let entries: RawArray<Entry<K, V>> = RawArray::new();

        Self { entries, len: 0 }
    }
}

impl<K, V> Entry<K, V> {
    fn new(key: K, value: V) -> Entry<K, V> {
        Self::Occupied { key, value }
    }

    /// Returns `true` if the entry is [`Vacant`].
    ///
    /// [`Vacant`]: Entry::Vacant
    #[must_use]
    pub(crate) fn is_vacant(&self) -> bool {
        matches!(self, Self::Vacant)
    }
}

impl<K, V> Default for Entry<K, V> {
    fn default() -> Self {
        Self::Vacant
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[allow(dead_code)]
    #[derive(Debug, PartialEq)]
    struct Foo {
        bar: usize,
    }

    #[test]
    fn test_foo() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo { bar: 1 });
        map.set("1", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 2 }), map.get(&"1"))
    }
}
