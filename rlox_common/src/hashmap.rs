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
    K: Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: K, value: V) -> bool {
        if self.needs_to_grow() {
            self.entries.grow(None);
        }

        let entry = self.find_entry(&key);
        if entry.is_vacant() {
            *entry = Entry::Occupied(OccupiedEntry { key, value });
            self.len += 1;
            true
        } else {
            let occupied_entry = entry
                .as_occupied_mut()
                .expect("entry at this pount must be occupied");
            let already_exists = occupied_entry.key() == key.borrow();
            if already_exists {
                occupied_entry.set_value(value);
                false
            } else {
                *entry = Entry::occupied(key, value);
                self.len += 1;
                true
            }
        }
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.is_empty() {
            return None;
        }

        match self.find_entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(entry) => {
                if <dyn Borrow<Q>>::borrow(entry.key()) == key {
                    Some(entry.value())
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn find_entry<Q: ?Sized>(&self, key: &Q) -> &mut Entry<K, V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut index = self.index(key.borrow());

        loop {
            let entry = self.get_entry(index);
            match entry {
                Entry::Vacant(_) => break self.get_entry_mut(index),
                Entry::Occupied(entry) => {
                    if entry.key().borrow() == key {
                        break self.get_entry_mut(index);
                    } else {
                        index = (index + 1) % self.capacity();
                    }
                }
            }
        }
    }

    #[inline]
    fn index<Q: ?Sized>(&self, key: &Q) -> usize
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        (hash % self.capacity() as u64) as usize
    }

    #[inline]
    fn get_entry(&self, index: usize) -> &Entry<K, V> {
        assert!(
            index < self.entries.capacity(),
            "index out of bounds on buckets array"
        );

        unsafe { &*self.entries.as_ptr().add(index) }
    }

    #[inline]
    fn get_entry_mut(&self, index: usize) -> &mut Entry<K, V> {
        assert!(
            index < self.entries.capacity(),
            "index out of bounds on buckets array"
        );

        unsafe { &mut *self.entries.as_ptr().add(index) }
    }

    #[inline]
    fn needs_to_grow(&self) -> bool {
        self.len + 1 > (self.entries.capacity() as f32 * MAX_LOAD) as usize
    }
}

#[derive(Debug)]
pub struct OccupiedEntry<K, V> {
    key: K,
    value: V,
}

impl<K, V> OccupiedEntry<K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn set_value(&mut self, value: V) {
        self.value = value;
    }
}

#[derive(Debug)]
pub struct VacantEntry;

#[derive(Debug)]
pub enum Entry<K, V> {
    Vacant(VacantEntry),
    Occupied(OccupiedEntry<K, V>),
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
    fn occupied(key: K, value: V) -> Entry<K, V> {
        Self::Occupied(OccupiedEntry { key, value })
    }

    #[inline]
    pub fn is_vacant(&self) -> bool {
        matches!(self, Self::Vacant(_))
    }

    #[inline]
    pub fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied { .. })
    }

    #[inline]
    pub fn as_occupied(&self) -> Option<&OccupiedEntry<K, V>> {
        if let Self::Occupied(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_occupied_mut(&mut self) -> Option<&mut OccupiedEntry<K, V>> {
        if let Self::Occupied(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<K, V> Default for Entry<K, V> {
    fn default() -> Self {
        Self::Vacant(VacantEntry)
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

    impl Drop for Foo {
        fn drop(&mut self) {}
    }

    #[test]
    fn test_set_empty() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        assert_eq!(true, map.is_empty());
        let result = map.set("1", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 2 }), map.get("1"));
        assert_eq!(1, map.len());
        assert_eq!(true, result);
    }

    #[test]
    fn test_set_non_empty_same_key() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        let result1 = map.set("1", Foo { bar: 1 });
        let result2 = map.set("1", Foo { bar: 2 });
        assert_eq!(Some(&Foo { bar: 2 }), map.get("1"));
        assert_eq!(1, map.len());
        assert_eq!(true, result1);
        assert_eq!(false, result2);
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
            assert_eq!(Some(&Foo { bar: 1 }), map.get(&key));
        }

        assert_eq!(10_000, 10_000);
    }
}
