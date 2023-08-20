use ahash::RandomState;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

/// A multi-map data structure that associates multiple values with a single key.
///
/// The `MultiMap` struct is implemented using a `HashMap` where each key is associated
/// with a vector containing multiple values. This allows for efficient storage and retrieval
/// of multiple values associated with the same key.
///
/// # Examples
///
/// ```
/// use ts_utils::MultiMap;
///
/// let mut multi_map = MultiMap::new();
///
/// multi_map.insert("fruit", "apple");
/// multi_map.insert("fruit", "banana");
/// multi_map.insert("color", "red");
///
/// assert_eq!(multi_map.get("fruit"), Some(&vec!["apple", "banana"]));
/// assert_eq!(multi_map.get("color"), Some(&vec!["red"]));
///```
/// # Type Parameters
///
/// - `K`: The type of the keys.
/// - `V`: The type of the values.
/// - `S`: The hash state builder used for hashing keys. Defaults to `RandomState`.
#[derive(Clone)]
pub struct MultiMap<K, V, S = RandomState> {
    inner: HashMap<K, Vec<V>, S>,
}

impl<K, V> MultiMap<K, V, RandomState>
where
    K: Eq + Hash,
{
    /// Creates a new, empty `MultiMap`.
    #[must_use]
    pub fn new() -> MultiMap<K, V> {
        Self::with_capacity(0)
    }

    /// Creates a new, empty `MultiMap` with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> MultiMap<K, V> {
        Self {
            inner: HashMap::with_capacity_and_hasher(capacity, RandomState::default()),
        }
    }
}

impl<K, V, S> MultiMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Creates a new `MultiMap` with the specified capacity and hash builder.
    #[must_use]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> MultiMap<K, V, S> {
        Self {
            inner: HashMap::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    /// Creates a new `MultiMap` with the specified hash builder.
    pub fn with_hasher(hash_builder: S) -> MultiMap<K, V, S> {
        Self::with_capacity_and_hasher(0, hash_builder)
    }

    /// Inserts a value associated with a key into the `MultiMap`.
    pub fn insert(&mut self, key: K, value: V) {
        self.inner.entry(key).or_default().push(value);
    }

    /// Removes a key and its associated values from the `MultiMap`, if present.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Vec<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key)
    }
}

impl<K, V, S> std::ops::Deref for MultiMap<K, V, S> {
    type Target = HashMap<K, Vec<V>, S>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

struct MultiMapVisitor<K, V, S> {
    marker: PhantomData<MultiMap<K, V, S>>,
}

impl<K, V, S> MultiMapVisitor<K, V, S>
where
    K: Hash + Eq,
{
    fn new() -> Self {
        MultiMapVisitor {
            marker: PhantomData,
        }
    }
}

impl<'a, K, V, S> serde::de::Visitor<'a> for MultiMapVisitor<K, V, S>
where
    K: Deserialize<'a> + Eq + Hash,
    V: Deserialize<'a>,
    S: BuildHasher + Default,
{
    type Value = MultiMap<K, V, S>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'a>,
    {
        let mut multi_map = MultiMap::with_hasher(S::default());
        while let Some((key, value)) = map.next_entry()? {
            multi_map.insert(key, value);
        }

        Ok(multi_map)
    }
}

impl<K, V, BS> Serialize for MultiMap<K, V, BS>
where
    K: Serialize + Eq + Hash,
    V: Serialize,
    BS: BuildHasher,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'a, K, V, S> Deserialize<'a> for MultiMap<K, V, S>
where
    K: Deserialize<'a> + Eq + Hash,
    V: Deserialize<'a>,
    S: BuildHasher + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_map(MultiMapVisitor::<K, V, S>::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query;

    #[test]
    fn test_with_capacity_and_hasher() {
        let custom_hash_builder = RandomState::new();
        let multi_map: MultiMap<i32, &str> =
            MultiMap::with_capacity_and_hasher(20, custom_hash_builder);

        assert!(multi_map.inner.is_empty());
        assert!(multi_map.inner.capacity() >= 20);
    }

    #[test]
    fn test_with_hasher() {
        let custom_hash_builder = RandomState::new();
        let multi_map: MultiMap<i32, &str> = MultiMap::with_hasher(custom_hash_builder);

        // Assert that the created map is empty and has the default initial capacity.
        assert!(multi_map.inner.is_empty());
        assert_eq!(multi_map.inner.capacity(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut multi_map = MultiMap::new();
        multi_map.insert("fruit", "apple");
        multi_map.insert("fruit", "banana");

        assert_eq!(multi_map.get("fruit"), Some(&vec!["apple", "banana"]));
        assert_eq!(multi_map.get("color"), None);
    }

    #[test]
    fn test_remove() {
        let mut multi_map = MultiMap::new();
        multi_map.insert("fruit", "apple");
        multi_map.insert("fruit", "banana");

        // Remove a key and assert the removed values.
        let removed_values = multi_map.remove("fruit");
        assert_eq!(removed_values, Some(vec!["apple", "banana"]));
        assert_eq!(multi_map.get("fruit"), None);
    }

    #[test]
    fn test_deserialize() {
        let query = "fruit=apple&fruit=banana&color=red&color=blue";
        let multi_map: MultiMap<&str, &str> = query::from_bytes(query.as_bytes()).unwrap();

        assert_eq!(multi_map.get("fruit"), Some(&vec!["apple", "banana"]));
        assert_eq!(multi_map.get("color"), Some(&vec!["red", "blue"]));
        assert_eq!(multi_map.get("size"), None);
    }
}
