use std::{borrow::Borrow, collections::hash_map::DefaultHasher, hash::Hash, hash::Hasher, ptr};

use crate::raw_array::RawArray;

const MAX_LOAD: f32 = 0.75;

#[derive(Debug, Default)]
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

    #[inline]
    pub fn find_entry_index<Q: ?Sized>(&self, key: &Q) -> usize
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash(key.borrow());
        let mut index = (hash % self.capacity() as u64) as usize;
        let mut tombstone: Option<usize> = None;

        loop {
            let entry = self.get_entry(index);
            match entry {
                Entry::Vacant => {
                    break match tombstone {
                        Some(tombstone_index) => tombstone_index,
                        None => index,
                    };
                }
                Entry::Tombstone => {
                    if tombstone.is_none() {
                        tombstone = Some(index);
                    }
                }
                Entry::Occupied(entry) => {
                    if entry.key().borrow() == key {
                        break index;
                    }
                }
            }

            index = (index + 1) % self.capacity();
        }
    }

    #[inline]
    pub fn find_entry<Q: ?Sized>(&self, key: &Q) -> &Entry<K, V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let index = self.find_entry_index(key);
        self.get_entry(index)
    }

    #[inline]
    pub fn find_entry_mut<Q: ?Sized>(&mut self, key: &Q) -> &mut Entry<K, V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let index = self.find_entry_index(key);
        self.get_entry_mut(index)
    }

    /// # Safety: [`entries.get`] is checking bounds.
    #[inline]
    fn get_entry(&self, index: usize) -> &Entry<K, V> {
        self.entries.get(index)
    }

    /// # Safety: [`entries.get_mut`] is checking bounds.
    #[inline]
    fn get_entry_mut<'a>(&'a mut self, index: usize) -> &'a mut Entry<K, V> {
        self.entries.get_mut(index)
    }

    pub fn grow(&mut self, new_capacity: Option<usize>) {
        self.entries.grow(new_capacity);
    }

    fn hash<Q: ?Sized>(&self, key: &Q) -> u64
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
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
    K: Eq + Hash,
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

        let index = self.inner.find_entry_index(&key);
        let entry = self.inner.get_entry_mut(index);

        match entry {
            Entry::Vacant => {
                entry.occupy(OccupiedEntry::new(key, value));
                self.len += 1;
                true
            }
            Entry::Tombstone => {
                entry.occupy(OccupiedEntry::new(key, value));
                true
            }
            Entry::Occupied(occupied_entry) => {
                let already_exists = occupied_entry.key() == key.borrow();
                if already_exists {
                    occupied_entry.set_value(value);
                    false
                } else {
                    entry.occupy(OccupiedEntry::new(key, value));
                    self.len += 1;
                    true
                }
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> bool {
        self.set(key, value)
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
            Entry::Vacant => None,
            Entry::Occupied(entry) => {
                if entry.key.borrow() == key {
                    Some(&entry.value)
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

        let entry = self.inner.find_entry_mut(&key);
        if entry.is_vacant() || entry.is_tombstone() {
            return false;
        }

        *entry = Entry::Tombstone;
        self.len -= 1;

        true
    }

    pub fn remove(&mut self, key: K) -> bool {
        self.delete(key)
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

    pub fn iter(&'_ self) -> Iter<'_, K, V> {
        Iter { map: self, at: 0 }
    }

    pub fn iter_mut(&'_ mut self) -> IterMut<'_, K, V> {
        IterMut { map: self, at: 0 }
    }

    #[inline]
    fn needs_to_grow(&self) -> bool {
        self.len + 1 > (self.capacity() as f32 * MAX_LOAD) as usize
    }

    #[inline]
    fn grow(&mut self) {
        if self.capacity() == 0 {
            // Capacity by default for an empty RawArray is 8 elements.
            self.inner.grow(None);
            return;
        }

        let mut new_inner: HashMapInner<K, V> = HashMapInner::with_capacity(self.capacity() * 2);
        let mut new_len = 0;
        for entry in self.inner.entries.as_slice() {
            match entry {
                Entry::Vacant | Entry::Tombstone => continue,
                Entry::Occupied(occupied_entry) => {
                    let index = new_inner.find_entry_index(occupied_entry.key());
                    let dest = new_inner.get_entry_mut(index);
                    unsafe { std::ptr::write(dest, std::ptr::read(entry)) };
                    new_len += 1;
                }
            }
        }

        std::mem::swap(&mut self.inner, &mut new_inner);
        self.len = new_len;
    }
}

pub struct Iter<'a, K, V>
where
    K: Hash + Eq,
{
    map: &'a HashMap<K, V>,
    at: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Hash + Eq,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.map.capacity() - 1 {
            return None;
        }

        let mut result = None;

        for entry in self.map.inner.entries.as_slice()[self.at..].iter() {
            self.at += 1;
            match entry {
                Entry::Vacant | Entry::Tombstone => {
                    continue;
                }
                Entry::Occupied(occupied_entry) => {
                    result = Some((occupied_entry.key(), occupied_entry.value()));
                    break;
                }
            }
        }

        result
    }
}

pub struct IterMut<'a, K, V>
where
    K: Hash + Eq,
{
    map: &'a mut HashMap<K, V>,
    at: usize,
}

impl<'a, K, V: 'a> Iterator for IterMut<'a, K, V>
where
    K: Hash + Eq,
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.map.inner.get_entry_mut(self.at).as_occupied_mut();
            self.at += 1;

            match entry {
                Some(occupied_entry) => unsafe {
                    let occupied_entry = occupied_entry.as_ptr();
                    break Some((&(*occupied_entry).key, &mut (*occupied_entry).value));
                },
                None => {
                    if self.at == self.map.capacity() {
                        break None;
                    }
                }
            }
        }
    }
}

impl<K, V> Default for HashMap<K, V>
where
    K: Hash + Eq + Default,
    V: Default,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
            len: Default::default(),
        }
    }
}

impl<K, V> Drop for HashMap<K, V>
where
    K: Hash + Eq,
{
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(self.inner.entries.as_mut_slice()) };
    }
}

#[derive(Debug)]
pub struct OccupiedEntry<K: Hash + Eq, V> {
    key: K,
    value: V,
}

impl<K, V> OccupiedEntry<K, V>
where
    K: Hash + Eq,
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    #[inline]
    pub fn key(&self) -> &K {
        &self.key
    }

    #[inline]
    pub fn value(&self) -> &V {
        &self.value
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    #[inline]
    pub fn set_value(&mut self, value: V) {
        self.value = value;
    }

    pub fn as_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }
}

#[derive(Debug)]
pub enum Entry<K, V>
where
    K: Hash + Eq,
{
    Vacant,
    Occupied(OccupiedEntry<K, V>),
    Tombstone,
}

impl<K, V> Entry<K, V>
where
    K: Hash + Eq,
{
    #[inline]
    pub fn is_vacant(&self) -> bool {
        matches!(self, Self::Vacant)
    }

    #[inline]
    pub fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied { .. })
    }

    pub fn is_tombstone(&self) -> bool {
        matches!(self, Self::Tombstone)
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

    #[inline]
    pub fn occupy(&mut self, occupied_entry: OccupiedEntry<K, V>) {
        *self = Self::Occupied(occupied_entry);
    }
}

impl<K, V> Default for Entry<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self::Vacant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct Foo {
        bar: usize,
    }

    impl Foo {
        pub fn new(bar: usize) -> Self {
            Self { bar }
        }
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
        for _ in 0..1000 {
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

    #[test]
    fn test_iter() {
        let mut map: HashMap<usize, Foo> = HashMap::new();
        let (foo0, foo1, foo2) = (Foo::new(0), Foo::new(1), Foo::new(2));
        map.set(2, foo2.clone());
        map.set(1, foo1.clone());
        map.set(0, foo0.clone());

        let mut entries: Vec<(&usize, &Foo)> = vec![];
        for entry in map.iter() {
            dbg!(entry);
            entries.push(entry);
        }

        let expected_entries = vec![(&0, &foo0), (&1, &foo1), (&2, &foo2)];
        entries.sort();
        assert_eq!(expected_entries, entries);
    }

    #[test]
    fn test_iter_mut() {
        let mut map: HashMap<usize, String> = HashMap::new();
        map.set(2, "Hello".into());
        map.set(1, "darkness".into());
        map.set(0, "into".to_string());

        for (key, value) in map.iter_mut() {
            if *key == 2 {
                *value = "I've come to talk with you again".to_string();
            }
            if *key == 1 {
                *value = "Because a vision".to_string();
            }
            if *key == 0 {
                *value = "softly creeping".to_string();
            }
        }

        assert_eq!(
            Some(&"I've come to talk with you again".into()),
            map.get(&2)
        );
        assert_eq!(Some(&"Because a vision".into()), map.get(&1));
        assert_eq!(Some(&"softly creeping".into()), map.get(&0));
    }
}
