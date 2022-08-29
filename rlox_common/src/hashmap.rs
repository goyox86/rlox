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

        let mut index = (self.hash(key.borrow()) % self.capacity() as u64) as usize;

        loop {
            let entry = self.find_entry(index);
            match entry {
                Entry::Vacant => {
                    self.set_entry(index, Entry::Occupied { key, value });
                    self.len += 1;
                    break true;
                }
                Entry::Occupied {
                    key: ekey,
                    value: _evalue,
                } => {
                    if ekey == key.borrow() {
                        self.set_entry(index, Entry::new(key, value));
                        break true;
                    } else {
                        index = (index + 1) % self.capacity();
                    }
                }
            }
        }
    }

    pub fn get(&mut self, key: K) -> Option<&V> {
        if self.is_empty() {
            return None;
        }

        let mut index = (self.hash(key.borrow()) % self.capacity() as u64) as usize;

        loop {
            let entry = self.find_entry(index);
            match entry {
                Entry::Vacant => {
                    break None;
                }
                Entry::Occupied {
                    key: ekey,
                    value: evalue,
                } => {
                    if ekey == key.borrow() {
                        break Some(evalue);
                    } else {
                        index = (index + 1) % self.capacity();
                    }
                }
            }
        }
    }

    fn hash(&self, key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
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

    /// Returns `true` if the entry is [`Occupied`].
    ///
    /// [`Occupied`]: Entry::Occupied
    #[must_use]
    pub fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied { .. })
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
    fn test_set_empty() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        assert_eq!(true, map.is_empty());
        map.set("1", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 2 }), map.get("1"));
        assert_eq!(1, map.len());
    }

    #[test]
    fn test_set_non_empty_same_key() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo { bar: 1 });
        map.set("1", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 2 }), map.get("1"));
        assert_eq!(1, map.len());
    }

    #[test]
    fn test_set_non_empty_diff_key() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo { bar: 1 });
        map.set("2", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 1 }), map.get("1"));
        assert_eq!(Some(&Foo { bar: 2 }), map.get("2"));
        assert_eq!(2, map.len());
    }

    #[test]
    fn test_set_non_empty_many() {
        use rand::prelude::*;

        let mut map: HashMap<usize, Foo> = HashMap::new();
        for _ in 0..10_000 {
            let key: usize = random();
            map.set(key, Foo { bar: 1 });
            assert_eq!(Some(&Foo { bar: 1 }), map.get(key));
        }

        assert_eq!(10_000, 10_000);
    }
}
