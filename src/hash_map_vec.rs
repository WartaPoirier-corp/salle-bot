use std::collections::hash_map::{Entry, HashMap};
use std::iter::FromIterator;

pub struct HashMapVec<K, V>(pub HashMap<K, Vec<V>>);

impl<K, V> HashMapVec<K, V> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<K: std::cmp::Eq + std::hash::Hash, V> HashMapVec<K, V> {
    fn insert(&mut self, key: K, value: V) {
        let mut entry = self.0.entry(key);

        let cell = match entry {
            Entry::Occupied(ref mut entry) => entry.get_mut(),
            Entry::Vacant(entry) => entry.insert(Vec::new()),
        };

        cell.push(value);
    }
}

impl<K: std::cmp::Eq + std::hash::Hash, V> FromIterator<(K, V)> for HashMapVec<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut ret = Self::new();
        iter.into_iter().for_each(|(k, v)| ret.insert(k, v));
        ret
    }
}
