use std::{collections::hash_map::DefaultHasher, hash::Hash, hash::Hasher};

use crate::Array;

const MAX_LOAD: f32 = 0.75;

#[derive(Debug)]
pub(crate) struct HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    entries: Array<Entry<K, V>>,
}

impl<K, V> HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    fn set(&mut self, key: K, value: V) -> bool {
        unimplemented!();
    }

    fn find_entry(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let mut idx = (hasher.finish() % self.entries.capacity() as u64) as usize;
        let result = idx;

        let result = loop {
            if let Some(entry) = self.entries.get(idx as usize) {
                if entry.key == *key {
                    break idx;
                }
            }

            idx = (idx + 1) % self.entries.capacity();
        };

        result
    }
}

#[derive(Debug)]
pub(crate) struct Entry<K, V> {
    key: K,
    value: V,
}

impl<K, V> Default for HashMap<K, V>
where
    K: PartialEq + Eq + Hash,
{
    fn default() -> Self {
        Self {
            entries: Default::default(),
        }
    }
}

impl<K, V> Entry<K, V> {
    fn new(key: K, value: V) -> Entry<K, V> {
        todo!()
    }
}
