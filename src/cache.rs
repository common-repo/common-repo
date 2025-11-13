//! # In-Process Caching
//!
//! This module provides an in-process, thread-safe cache for storing processed
//! repositories. The primary goal of this cache is to avoid redundant processing
//! of the same repository (with the same operations) within a single run of the
//! `common-repo` application.
//!
//! ## Caching Strategy
//!
//! This in-process cache is one of two caching layers in the application:
//!
//! 1.  **Disk Cache (`.common-repo/cache`)**: This is the primary, persistent
//!     cache that stores the cloned contents of repositories on the filesystem.
//!     It is managed by the `RepositoryManager`.
//!
//! 2.  **In-Process Cache (`RepoCache`)**: This is a secondary, in-memory cache
//!     that stores the `MemoryFS` of a repository *after* its operations have
//!     been applied. This is useful in complex inheritance scenarios where the
//!     same repository might be referenced multiple times with the same set of
//!     `with:` clause operations.
//!
//! The `RepoCache` is implemented using a `HashMap` wrapped in an `Arc<Mutex>`,
//! which allows it to be shared safely across multiple threads.
//!
//! ## Key Components
//!
//! - **`RepoCache`**: The main struct that provides the caching functionality.
//!
//! - **`CacheKey`**: A struct that uniquely identifies a cached item based on
//!   the repository's URL and Git reference.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

/// Cache key combining URL and reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct CacheKey {
    /// The URL of the repository.
    pub url: String,
    /// The Git reference (e.g., branch, tag, commit hash).
    pub r#ref: String,
}

#[allow(dead_code)]
impl CacheKey {
    /// Creates a new `CacheKey` from a URL and a Git reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::cache::CacheKey;
    ///
    /// let key = CacheKey::new("https://github.com/user/repo.git", "main");
    /// assert_eq!(key.url, "https://github.com/user/repo.git");
    /// assert_eq!(key.r#ref, "main");
    ///
    /// // Keys with same values are equal
    /// let key1 = CacheKey::new("https://example.com/repo", "v1.0.0");
    /// let key2 = CacheKey::new("https://example.com/repo", "v1.0.0");
    /// assert_eq!(key1, key2);
    /// ```
    pub fn new(url: &str, r#ref: &str) -> Self {
        Self {
            url: url.to_string(),
            r#ref: r#ref.to_string(),
        }
    }
}

/// A thread-safe, in-process cache for storing processed repositories as
/// `MemoryFS` instances.
///
/// This cache is used to avoid re-processing the same repository with the same
/// operations within a single application run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RepoCache {
    cache: Arc<Mutex<HashMap<CacheKey, MemoryFS>>>,
}

#[allow(dead_code)]
impl RepoCache {
    /// Creates a new, empty `RepoCache`.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieves a `MemoryFS` from the cache. If it's not present, the
    /// `processor` closure is executed to generate it, and the result is
    /// then stored in the cache before being returned.
    ///
    /// This ensures that the expensive processing of a repository is only
    /// performed once for each unique `CacheKey`.
    pub fn get_or_process<F>(&self, key: CacheKey, processor: F) -> Result<MemoryFS>
    where
        F: FnOnce() -> Result<MemoryFS>,
    {
        // First check if we have a cached result
        {
            let cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
                context: "Cache lock".to_string(),
            })?;
            if let Some(cached) = cache.get(&key) {
                return Ok(cached.clone());
            }
        }

        // Not in cache, compute it
        let result = processor()?;

        // Store in cache
        {
            let mut cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
                context: "Cache lock".to_string(),
            })?;
            cache.insert(key, result.clone());
        }

        Ok(result)
    }

    /// Inserts a `MemoryFS` into the cache for a given `CacheKey`.
    ///
    /// If an entry for the key already exists, it will be overwritten.
    pub fn insert(&self, key: CacheKey, value: MemoryFS) -> Result<()> {
        let mut cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        cache.insert(key, value);
        Ok(())
    }

    /// Retrieves a `MemoryFS` from the cache for a given `CacheKey`, if it
    /// exists.
    pub fn get(&self, key: &CacheKey) -> Result<Option<MemoryFS>> {
        let cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        Ok(cache.get(key).cloned())
    }

    /// Checks if the cache contains an entry for the given `CacheKey`.
    pub fn contains(&self, key: &CacheKey) -> Result<bool> {
        let cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        Ok(cache.contains_key(key))
    }

    /// Removes all entries from the cache.
    pub fn clear(&self) -> Result<()> {
        let mut cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        cache.clear();
        Ok(())
    }

    /// Returns the number of entries currently in the cache.
    pub fn len(&self) -> Result<usize> {
        let cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        Ok(cache.len())
    }

    /// Returns `true` if the cache contains no entries.
    pub fn is_empty(&self) -> Result<bool> {
        let cache = self.cache.lock().map_err(|_| Error::LockPoisoned {
            context: "Cache lock".to_string(),
        })?;
        Ok(cache.is_empty())
    }
}

impl Default for RepoCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFS;

    #[test]
    fn test_cache_key() {
        let key1 = CacheKey::new("https://github.com/user/repo.git", "main");
        let key2 = CacheKey::new("https://github.com/user/repo.git", "main");
        let key3 = CacheKey::new("https://github.com/user/repo.git", "develop");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_get_or_process() {
        let cache = RepoCache::new();
        let key = CacheKey::new("https://example.com/repo.git", "v1.0.0");

        // First call should execute processor
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let result1 = cache
            .get_or_process(key.clone(), || {
                *call_count_clone.lock().unwrap() += 1;
                let mut fs = MemoryFS::new();
                fs.add_file_string("test.txt", "content").unwrap();
                Ok(fs)
            })
            .unwrap();

        assert_eq!(*call_count.lock().unwrap(), 1);
        assert!(result1.exists("test.txt"));

        // Second call should use cached result
        let call_count_clone = Arc::clone(&call_count);
        let result2 = cache
            .get_or_process(key, || {
                *call_count_clone.lock().unwrap() += 1;
                let mut fs = MemoryFS::new();
                fs.add_file_string("test2.txt", "content2").unwrap();
                Ok(fs)
            })
            .unwrap();

        assert_eq!(*call_count.lock().unwrap(), 1); // Still 1, processor not called
        assert!(result2.exists("test.txt"));
        assert!(!result2.exists("test2.txt")); // Should not have the new content
    }

    #[test]
    fn test_cache_operations() {
        let cache = RepoCache::new();
        let key = CacheKey::new("https://example.com/repo.git", "main");

        // Initially empty
        assert!(cache.is_empty().unwrap());
        assert_eq!(cache.len().unwrap(), 0);
        assert!(!cache.contains(&key).unwrap());

        // Insert
        let mut fs = MemoryFS::new();
        fs.add_file_string("file.txt", "content").unwrap();
        cache.insert(key.clone(), fs).unwrap();

        // Now has content
        assert!(!cache.is_empty().unwrap());
        assert_eq!(cache.len().unwrap(), 1);
        assert!(cache.contains(&key).unwrap());

        // Get
        let retrieved = cache.get(&key).unwrap().unwrap();
        assert!(retrieved.exists("file.txt"));

        // Clear
        cache.clear().unwrap();
        assert!(cache.is_empty().unwrap());
        assert!(!cache.contains(&key).unwrap());
    }

    #[test]
    fn test_cache_default() {
        let cache = RepoCache::default();
        assert!(cache.is_empty().unwrap());
        assert_eq!(cache.len().unwrap(), 0);
    }
}
