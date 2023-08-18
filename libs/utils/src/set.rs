use ahash::AHashSet;
use std::hash::Hash;
use std::str::FromStr;
use std::{
    fmt, fs,
    io::{self, BufRead},
    ops::{Deref, DerefMut},
};

/// A wrapper around `AHashSet` providing additional methods and functionalities.
///
/// This struct serves as a wrapper around the `AHashSet` collection, adding utility methods
/// for loading and managing the set's content from a file.
#[derive(Debug, Default, Clone)]
pub struct Set<T>(AHashSet<T>);

impl<T> Deref for Set<T> {
    type Target = AHashSet<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Set<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Set<T>
where
    T: Eq + Hash,
{
    /// Creates a new empty `Set`.
    ///
    /// Returns a new `Set` instance backed by an empty `AHashSet`.
    #[must_use]
    pub fn new() -> Self {
        Self(AHashSet::new())
    }

    /// Creates a new `Set` and loads items from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path from which to load items.
    ///
    /// # Returns
    ///
    /// Returns a new `Set` instance containing items loaded from the file.
    pub fn from_file(path: &str) -> io::Result<Self>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Debug,
    {
        let mut set = Self::new();
        set.load_from_file(path)?;
        Ok(set)
    }

    /// Loads items from a file and adds them to the `Set`.
    ///
    /// Loads items from the specified file path and inserts them into the `Set`.
    /// Each line in the file is treated as an item, and items are parsed from string representations
    /// using the `FromStr` trait implementation for the item type.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path from which to load items.
    pub fn load_from_file(&mut self, path: &str) -> io::Result<()>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Debug,
    {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap_or_default();
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let item = line.parse::<T>().unwrap();
            self.insert(item);
        }

        Ok(())
    }
}

impl<T> From<Vec<T>> for Set<T>
where
    T: Eq + Hash,
{
    fn from(vec: Vec<T>) -> Self {
        Self(vec.into_iter().collect())
    }
}
