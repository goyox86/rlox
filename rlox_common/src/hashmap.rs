use std::{borrow::Borrow, collections::hash_map::DefaultHasher, hash::Hash, hash::Hasher};

use crate::raw_array::RawArray;

const MAX_LOAD: f32 = 0.75;

#[derive(Debug)]
pub struct HashMapInner<K, V>
where
    K: PartialEq + Eq + Hash,
{
    pub entries: RawArray<Entry<K, V>>,
}

#[allow(dead_code)]
impl<K, V> HashMapInner<K, V>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            entries: RawArray::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut me = Self::new();
        me.entries.grow(Some(capacity));
        me
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn find_entry<Q: ?Sized>(&self, key: &Q) -> &mut Entry<K, V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut index = self.index(key.borrow());
        let mut tombstone: Option<&mut Entry<K, V>> = None;

        loop {
            let entry = self.get_entry_mut(index);
            match entry {
                Entry::Vacant(_) => {
                    return if tombstone.is_some() {
                        tombstone.unwrap()
                    } else {
                        self.get_entry_mut(index)
                    }
                }
                Entry::Tombstone => {
                    if tombstone.is_none() {
                        tombstone = Some(entry);
                    }
                }
                Entry::Occupied(entry) => {
                    if entry.key().borrow() == key {
                        return self.get_entry_mut(index);
                    }
                }
            }

            index = (index + 1) % self.capacity();
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
            "index out of bounds: index is: {} but array capacity is: {}",
            index,
            self.capacity()
        );

        unsafe { &*self.entries.as_ptr().add(index) }
    }

    #[inline]
    fn get_entry_mut(&self, index: usize) -> &mut Entry<K, V> {
        assert!(
            index < self.entries.capacity(),
            "index out of bounds: index is: {} but array capacity is: {}",
            index,
            self.capacity()
        );

        unsafe { &mut *self.entries.as_ptr().add(index) }
    }

    pub fn grow(&mut self) {
        self.entries.grow(None);
    }
}

#[derive(Debug)]
pub struct HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    inner: HashMapInner<K, V>,
    len: usize,
}

#[allow(dead_code)]
impl<K, V> HashMap<K, V>
where
    K: Eq + Hash + std::fmt::Debug,
    V: std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            inner: HashMapInner::new(),
            len: 0,
        }
    }

    pub fn set(&mut self, key: K, value: V) -> bool {
        if self.needs_to_grow() {
            self.grow()
        }

        let entry = self.inner.find_entry(&key);

        match entry {
            Entry::Vacant(_) => {
                *entry = Entry::Occupied(OccupiedEntry { key, value });
                self.len += 1;
                true
            }
            Entry::Tombstone => {
                *entry = Entry::Occupied(OccupiedEntry { key, value });
                true
            }
            Entry::Occupied(occupied_entry) => {
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
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.is_empty() {
            return None;
        }

        match self.inner.find_entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(entry) => {
                if <dyn Borrow<Q>>::borrow(entry.key()) == key {
                    Some(entry.value())
                } else {
                    None
                }
            }
            Entry::Tombstone => None,
        }
    }

    pub fn delete(&mut self, key: K) -> bool {
        if self.is_empty() {
            return false;
        }

        let entry = self.inner.find_entry(&key);
        if entry.is_vacant() || entry.is_tombstone() {
            return false;
        }

        *entry = Entry::Tombstone;
        self.len -= 1;

        true
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn needs_to_grow(&self) -> bool {
        self.len + 1 > (self.capacity() as f32 * MAX_LOAD) as usize
    }

    #[inline]
    fn grow(&mut self) {
        if self.is_empty() {
            self.inner.entries.grow(None);
            return;
        }

        let mut new_inner: HashMapInner<K, V> = HashMapInner::with_capacity(self.capacity() * 2);
        let mut new_len = 0;
        for entry in self.inner.entries.as_slice() {
            match entry {
                Entry::Vacant(_) | Entry::Tombstone => continue,
                Entry::Occupied(occupied_entry) => {
                    let dest = new_inner.find_entry(occupied_entry.key());
                    *dest = unsafe { std::ptr::read(entry) };
                    new_len += 1;
                }
            }
        }

        std::mem::swap(&mut self.inner, &mut new_inner);
        self.len = new_len;
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
    Tombstone,
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

    pub fn is_tombstone(&self) -> bool {
        matches!(self, Self::Tombstone)
    }

    pub fn make_vacant(&mut self) {
        *self = Self::Vacant(VacantEntry);
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
        for _ in 0..100 {
            let key: usize = random();
            map.set(key, Foo { bar: 1 });
            assert_eq!(Some(&Foo { bar: 1 }), map.get(&key));
        }

        assert_eq!(100, map.len());
    }

    #[test]
    fn test_delete_empty() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        assert_eq!(true, map.is_empty());
        assert_eq!(map.delete("1"), false);
    }

    #[test]
    fn test_delete_non_empty_existing() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo { bar: 1 });
        assert_eq!(map.delete("1"), true);
        assert_eq!(map.get("1"), None);
    }

    #[test]
    fn test_delete_non_empty_non_existing() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo { bar: 1 });
        map.set("2", Foo { bar: 2 });
        assert_eq!(map.delete("0"), false);
        assert_eq!(Some(&Foo { bar: 1 }), map.get("1"));
        assert_eq!(Some(&Foo { bar: 2 }), map.get("2"));
    }

    #[test]
    fn test_delete_non_empty_many() {
        use rand::prelude::*;
        let mut keys: Vec<usize> = Vec::new();
        let mut map: HashMap<usize, Foo> = HashMap::new();
        for _ in 0..10 {
            let key: usize = random();
            keys.push(key);
            map.set(key, Foo { bar: 1 });
            assert_eq!(Some(&Foo { bar: 1 }), map.get(&key));
        }

        for key in &keys {
            map.delete(*key);
        }

        assert_eq!(0, map.len());
    }
}
