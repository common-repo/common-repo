//! In-process caching of processed repositories

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

/// Cache key combining URL and reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct CacheKey {
    pub url: String,
    pub r#ref: String,
}

#[allow(dead_code)]
impl CacheKey {
    pub fn new(url: &str, r#ref: &str) -> Self {
        Self {
            url: url.to_string(),
            r#ref: r#ref.to_string(),
        }
    }
}

/// In-process cache for processed repositories
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RepoCache {
    cache: Arc<Mutex<HashMap<CacheKey, MemoryFS>>>,
}

#[allow(dead_code)]
impl RepoCache {
    /// Create a new empty repository cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a cached repository, or compute and cache it if not present
    pub fn get_or_process<F>(&self, key: CacheKey, processor: F) -> Result<MemoryFS>
    where
        F: FnOnce() -> Result<MemoryFS>,
    {
        // First check if we have a cached result
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
            if let Some(cached) = cache.get(&key) {
                return Ok(cached.clone());
            }
        }

        // Not in cache, compute it
        let result = processor()?;

        // Store in cache
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
            cache.insert(key, result.clone());
        }

        Ok(result)
    }

    /// Manually insert a value into the cache
    pub fn insert(&self, key: CacheKey, value: MemoryFS) -> Result<()> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
        cache.insert(key, value);
        Ok(())
    }

    /// Get a value from cache without computing
    pub fn get(&self, key: &CacheKey) -> Result<Option<MemoryFS>> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
        Ok(cache.get(key).cloned())
    }

    /// Check if a key exists in cache
    pub fn contains(&self, key: &CacheKey) -> Result<bool> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
        Ok(cache.contains_key(key))
    }

    /// Clear all cached entries
    pub fn clear(&self) -> Result<()> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
        cache.clear();
        Ok(())
    }

    /// Get the number of cached entries
    pub fn len(&self) -> Result<usize> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
        Ok(cache.len())
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> Result<bool> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| Error::Generic("Cache lock poisoned".to_string()))?;
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
