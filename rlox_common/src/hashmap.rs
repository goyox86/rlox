use std::{
    borrow::Borrow, collections::hash_map::DefaultHasher, fmt::Debug, hash::Hash, hash::Hasher, ptr,
};

use crate::raw_array::RawArray;

const MAX_LOAD: f32 = 0.75;

#[derive(Debug, Default)]
struct HashMapInner<K, V>
where
    K: PartialEq + Eq + Hash,
{
    pub entries: RawArray<Entry<K, V>>,
}

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
        let mut entries: RawArray<Entry<K, V>> = RawArray::with_capacity(capacity);
        // We need all entries by default to be Entry::Vacant.
        entries.as_mut_slice().iter_mut().for_each(|entry| {
            unsafe { ptr::write(entry, Entry::Vacant) };
        });

        Self { entries }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    #[inline]
    fn find_entry_index<Q: ?Sized>(&self, key: &Q) -> usize
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
                    break tombstone.map_or_else(|| index, |tombstone_index| tombstone_index)
                }
                Entry::Tombstone => {
                    if tombstone.is_none() {
                        tombstone = Some(index);
                    }
                }
                Entry::Occupied(entry) => {
                    if entry.key.borrow() == key {
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
        self.get_entry(self.find_entry_index(key))
    }

    #[inline]
    pub fn find_entry_mut<Q: ?Sized>(&mut self, key: &Q) -> &mut Entry<K, V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get_entry_mut(self.find_entry_index(key))
    }

    /// # Safety: [`entries.get`] is checking bounds.
    #[inline]
    fn get_entry(&self, index: usize) -> &Entry<K, V> {
        self.entries.get(index)
    }

    /// # Safety: [`entries.get_mut`] is checking bounds.
    #[inline]
    fn get_entry_mut(&mut self, index: usize) -> &mut Entry<K, V> {
        self.entries.get_mut(index)
    }

    fn hash<Q>(&self, key: &Q) -> u64
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
        Q: ?Sized,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

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
                let already_exists = occupied_entry.key == key;
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

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get(key).is_some()
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

    pub fn delete<Q: ?Sized>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.is_empty() {
            return false;
        }

        let entry = self.inner.find_entry_mut(key);
        if entry.is_vacant() || entry.is_tombstone() {
            return false;
        }

        *entry = Entry::Tombstone;
        self.len -= 1;

        true
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
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

    #[inline]
    fn needs_to_grow(&self) -> bool {
        self.len + 1 > (self.capacity() as f32 * MAX_LOAD) as usize
    }

    #[inline]
    fn grow(&mut self) {
        let new_capacity = if self.capacity() == 0 {
            8
        } else {
            self.capacity() * 2
        };

        let mut new_inner: HashMapInner<K, V> = HashMapInner::with_capacity(new_capacity);
        let mut new_len = 0;
        for entry in self.inner.entries.as_slice() {
            match entry {
                Entry::Vacant | Entry::Tombstone => continue,
                Entry::Occupied(occupied_entry) => {
                    let index = new_inner.find_entry_index(&occupied_entry.key);
                    let dest = new_inner.get_entry_mut(index);
                    unsafe { ptr::write(dest, ptr::read(entry)) };
                    new_len += 1;
                }
            }
        }

        self.inner = new_inner;
        self.len = new_len;
    }

    pub fn iter(&'_ self) -> Iter<'_, K, V> {
        Iter { map: self, at: 0 }
    }

    pub fn iter_mut(&'_ mut self) -> IterMut<'_, K, V> {
        IterMut { map: self, at: 0 }
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
        if self.map.is_empty() {
            return None;
        }

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
                    result = Some((&occupied_entry.key, &occupied_entry.value));
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
        if self.map.is_empty() {
            return None;
        }

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
            inner: HashMapInner::new(),
            len: 0,
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

    #[inline]
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
    use fake::{Dummy, Fake, Faker};

    #[allow(dead_code)]
    #[derive(Clone, Debug, Dummy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    fn create_map<K, V>(quantity: usize) -> (Vec<(K, V)>, HashMap<K, V>)
    where
        K: Eq + Hash + Dummy<Faker> + Clone,
        V: Dummy<Faker> + Clone,
    {
        let mut map: HashMap<K, V> = HashMap::new();
        let mut pairs: Vec<(K, V)> = Vec::new();
        for _ in 0..quantity {
            let key: K = Faker.fake();
            let value: V = Faker.fake();
            map.set(key.clone(), value.clone());
            pairs.push((key, value))
        }

        (pairs, map)
    }

    #[test]
    fn test_set_empty() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        assert_eq!(true, map.is_empty());
        let result = map.set("1", Foo::new(2));
        assert_eq!(Some(&Foo { bar: 2 }), map.get("1"));
        assert_eq!(1, map.len());
        assert_eq!(true, result);
    }

    #[test]
    fn test_set_non_empty_same_key() {
        let mut map: HashMap<String, Foo> = HashMap::new();
        let result1 = map.set("1".into(), Foo::new(1));
        let result2 = map.set("1".into(), Foo::new(2));
        assert_eq!(Some(&Foo::new(2)), map.get("1"));
        assert_eq!(1, map.len());
        assert_eq!(true, result1);
        assert_eq!(false, result2);
    }

    #[test]
    fn test_set_non_empty_diff_key() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo::new(1));
        map.set("2", Foo::new(2));
        assert_eq!(Some(&Foo::new(1)), map.get("1"));
        assert_eq!(Some(&Foo::new(2)), map.get("2"));
        assert_eq!(2, map.len());
    }

    #[test]
    fn test_set_non_empty_many() {
        let (keys_values, map) = create_map::<usize, Foo>(100);
        for (key, value) in keys_values.iter() {
            assert_eq!(Some(value), map.get(key));
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
        map.set("1", Foo::new(1));
        assert_eq!(map.delete("1"), true);
        assert_eq!(map.get("1"), None);
    }

    #[test]
    fn test_delete_non_empty_non_existing() {
        let mut map: HashMap<&str, Foo> = HashMap::new();
        map.set("1", Foo::new(1));
        map.set("2", Foo::new(2));
        assert_eq!(map.delete("0"), false);
        assert_eq!(Some(&Foo::new(1)), map.get("1"));
        assert_eq!(Some(&Foo::new(2)), map.get("2"));
    }

    #[test]
    fn test_delete_non_empty_many() {
        let (keys_values, mut map) = create_map::<String, Foo>(1000);
        for (key, value) in keys_values.iter() {
            assert_eq!(Some(value), map.get(key));
        }
        assert_eq!(1000, map.len());

        for (key, _) in &keys_values {
            assert_eq!(true, map.delete(key));
        }
        assert_eq!(0, map.len());
    }

    #[test]
    fn test_iter_empty() {
        let map: HashMap<usize, String> = HashMap::new();
        assert_eq!(map.iter().next(), None);
    }

    #[test]
    fn test_iter_mut_empty() {
        let mut map: HashMap<usize, String> = HashMap::new();
        assert_eq!(map.iter_mut().next(), None);
    }

    #[test]
    fn test_iter() {
        let mut map: HashMap<usize, String> = HashMap::new();
        map.set(1, "1".into());
        map.set(2, "2".into());
        map.set(3, "3".into());

        let mut iter_entries = map.iter();
        // Order does not matter, for these particular set of entries they come like this.
        assert_eq!(iter_entries.next(), Some((&1, &"1".into())));
        assert_eq!(iter_entries.next(), Some((&3, &"3".into())));
        assert_eq!(iter_entries.next(), Some((&2, &"2".into())));
        assert_eq!(iter_entries.next(), None);
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
