//! High-level repository management with caching
//!
//! This module provides the main interface for fetching and caching repositories,
//! abstracting away the details of git operations and filesystem management.

use crate::error::Result;
use crate::filesystem::MemoryFS;
use std::path::{Path, PathBuf};

/// Trait for git operations - allows mocking in tests
#[allow(dead_code)]
pub trait GitOperations: Send + Sync {
    /// Clone a repository at a specific ref
    fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()>;

    /// List tags from a remote repository
    fn list_tags(&self, url: &str) -> Result<Vec<String>>;
}

/// Trait for cache operations - allows mocking in tests
#[allow(dead_code)]
pub trait CacheOperations: Send + Sync {
    /// Check if a cached repository exists
    fn exists(&self, cache_path: &Path) -> bool;

    /// Get the cache path for a repository
    fn get_cache_path(&self, url: &str, ref_name: &str) -> PathBuf;

    /// Get the cache path for a repository with optional sub-path
    fn get_cache_path_with_path(&self, url: &str, ref_name: &str, _path: Option<&str>) -> PathBuf {
        // Default implementation: ignore path for backward compatibility
        self.get_cache_path(url, ref_name)
    }

    /// Load a repository from cache into MemoryFS
    fn load_from_cache(&self, cache_path: &Path) -> Result<MemoryFS>;

    /// Load a repository from cache with optional path filtering
    fn load_from_cache_with_path(
        &self,
        cache_path: &Path,
        _path: Option<&str>,
    ) -> Result<MemoryFS> {
        // Default implementation: ignore path and load full repository
        self.load_from_cache(cache_path)
    }

    /// Save a MemoryFS to cache
    fn save_to_cache(&self, cache_path: &Path, fs: &MemoryFS) -> Result<()>;
}

/// Default implementation using real git and filesystem operations
#[allow(dead_code)]
pub struct DefaultGitOperations;

impl GitOperations for DefaultGitOperations {
    fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()> {
        crate::git::clone_shallow(url, ref_name, target_dir)
    }

    fn list_tags(&self, url: &str) -> Result<Vec<String>> {
        crate::git::list_tags(url)
    }
}

/// Default implementation using real cache operations
#[allow(dead_code)]
pub struct DefaultCacheOperations {
    cache_root: PathBuf,
}

impl DefaultCacheOperations {
    #[allow(dead_code)]
    pub fn new(cache_root: PathBuf) -> Self {
        Self { cache_root }
    }
}

impl CacheOperations for DefaultCacheOperations {
    fn exists(&self, cache_path: &Path) -> bool {
        cache_path.exists() && cache_path.is_dir()
    }

    fn get_cache_path(&self, url: &str, ref_name: &str) -> PathBuf {
        crate::git::url_to_cache_path(&self.cache_root, url, ref_name)
    }

    fn get_cache_path_with_path(&self, url: &str, ref_name: &str, path: Option<&str>) -> PathBuf {
        crate::git::url_to_cache_path_with_path(&self.cache_root, url, ref_name, path)
    }

    fn load_from_cache(&self, cache_path: &Path) -> Result<MemoryFS> {
        crate::git::load_from_cache(cache_path)
    }

    fn load_from_cache_with_path(&self, cache_path: &Path, path: Option<&str>) -> Result<MemoryFS> {
        crate::git::load_from_cache_with_path(cache_path, path)
    }

    fn save_to_cache(&self, cache_path: &Path, fs: &MemoryFS) -> Result<()> {
        crate::git::save_to_cache(cache_path, fs)
    }
}

/// Repository manager that handles cloning, caching, and loading
#[allow(dead_code)]
pub struct RepositoryManager {
    git_ops: Box<dyn GitOperations>,
    cache_ops: Box<dyn CacheOperations>,
}

impl RepositoryManager {
    /// Create a new repository manager with default operations
    #[allow(dead_code)]
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            git_ops: Box::new(DefaultGitOperations),
            cache_ops: Box::new(DefaultCacheOperations::new(cache_root)),
        }
    }

    /// Create a repository manager with custom operations (for testing)
    #[cfg(test)]
    pub fn with_operations(
        git_ops: Box<dyn GitOperations>,
        cache_ops: Box<dyn CacheOperations>,
    ) -> Self {
        Self { git_ops, cache_ops }
    }

    /// Fetch a repository, using cache if available
    ///
    /// This will:
    /// 1. Check if the repository is already cached
    /// 2. If not, clone it to the cache directory
    /// 3. Load the repository from cache into a MemoryFS
    #[allow(dead_code)]
    pub fn fetch_repository(&self, url: &str, ref_name: &str) -> Result<MemoryFS> {
        self.fetch_repository_with_path(url, ref_name, None)
    }

    /// Fetch a repository with optional sub-path filtering, using cache if available
    ///
    /// This will:
    /// 1. Check if the repository is already cached
    /// 2. If not, clone it to the cache directory
    /// 3. Load the repository from cache with path filtering into a MemoryFS
    #[allow(dead_code)]
    pub fn fetch_repository_with_path(
        &self,
        url: &str,
        ref_name: &str,
        path: Option<&str>,
    ) -> Result<MemoryFS> {
        let cache_path = self.cache_ops.get_cache_path_with_path(url, ref_name, path);

        // Check if already cached
        if !self.cache_ops.exists(&cache_path) {
            // Clone to cache
            self.git_ops.clone_shallow(url, ref_name, &cache_path)?;
        }

        // Load from cache with path filtering
        self.cache_ops.load_from_cache_with_path(&cache_path, path)
    }

    /// Force fetch a repository, bypassing cache
    ///
    /// This will always clone fresh and update the cache
    #[allow(dead_code)]
    pub fn fetch_repository_fresh(&self, url: &str, ref_name: &str) -> Result<MemoryFS> {
        self.fetch_repository_fresh_with_path(url, ref_name, None)
    }

    /// Force fetch a repository with optional sub-path filtering, bypassing cache
    ///
    /// This will always clone fresh and update the cache
    #[allow(dead_code)]
    pub fn fetch_repository_fresh_with_path(
        &self,
        url: &str,
        ref_name: &str,
        path: Option<&str>,
    ) -> Result<MemoryFS> {
        let cache_path = self.cache_ops.get_cache_path_with_path(url, ref_name, path);

        // Always clone fresh
        self.git_ops.clone_shallow(url, ref_name, &cache_path)?;

        // Load from cache with path filtering
        self.cache_ops.load_from_cache_with_path(&cache_path, path)
    }

    /// Check if a repository is cached
    #[allow(dead_code)]
    pub fn is_cached(&self, url: &str, ref_name: &str) -> bool {
        self.is_cached_with_path(url, ref_name, None)
    }

    /// Check if a repository with optional sub-path is cached
    #[allow(dead_code)]
    pub fn is_cached_with_path(&self, url: &str, ref_name: &str, path: Option<&str>) -> bool {
        let cache_path = self.cache_ops.get_cache_path_with_path(url, ref_name, path);
        self.cache_ops.exists(&cache_path)
    }

    /// List available tags for a repository
    #[allow(dead_code)]
    pub fn list_repository_tags(&self, url: &str) -> Result<Vec<String>> {
        self.git_ops.list_tags(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock git operations for testing
    struct MockGitOperations {
        clone_calls: Arc<Mutex<Vec<(String, String, PathBuf)>>>,
        should_fail: bool,
        error_message: String,
        tags: Vec<String>,
    }

    impl MockGitOperations {
        fn new() -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                should_fail: false,
                error_message: String::new(),
                tags: vec!["v1.0.0".to_string(), "v2.0.0".to_string()],
            }
        }

        fn with_error(message: String) -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                should_fail: true,
                error_message: message,
                tags: vec![],
            }
        }
    }

    impl GitOperations for MockGitOperations {
        fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()> {
            self.clone_calls.lock().unwrap().push((
                url.to_string(),
                ref_name.to_string(),
                target_dir.to_path_buf(),
            ));
            if self.should_fail {
                Err(crate::error::Error::GitClone {
                    url: url.to_string(),
                    r#ref: ref_name.to_string(),
                    message: self.error_message.clone(),
                })
            } else {
                Ok(())
            }
        }

        fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
            Ok(self.tags.clone())
        }
    }

    /// Mock cache operations for testing
    struct MockCacheOperations {
        cached_repos: Arc<Mutex<Vec<PathBuf>>>,
        cache_root: PathBuf,
        filesystem: MemoryFS,
    }

    impl MockCacheOperations {
        fn new() -> Self {
            Self {
                cached_repos: Arc::new(Mutex::new(Vec::new())),
                cache_root: PathBuf::from("/mock/cache"),
                filesystem: MemoryFS::new(),
            }
        }

        fn with_cached(paths: Vec<PathBuf>) -> Self {
            Self {
                cached_repos: Arc::new(Mutex::new(paths)),
                cache_root: PathBuf::from("/mock/cache"),
                filesystem: MemoryFS::new(),
            }
        }

        #[allow(dead_code)]
        fn with_filesystem(filesystem: MemoryFS) -> Self {
            Self {
                cached_repos: Arc::new(Mutex::new(Vec::new())),
                cache_root: PathBuf::from("/mock/cache"),
                filesystem,
            }
        }
    }

    impl CacheOperations for MockCacheOperations {
        fn exists(&self, cache_path: &Path) -> bool {
            self.cached_repos
                .lock()
                .unwrap()
                .contains(&cache_path.to_path_buf())
        }

        fn get_cache_path(&self, url: &str, ref_name: &str) -> PathBuf {
            // Simple mock implementation
            self.cache_root
                .join(format!("{}-{}", url.replace(['/', ':'], "-"), ref_name))
        }

        fn get_cache_path_with_path(
            &self,
            url: &str,
            ref_name: &str,
            path: Option<&str>,
        ) -> PathBuf {
            // Mock implementation that includes path in cache key
            let base_key = format!("{}-{}", url.replace(['/', ':'], "-"), ref_name);
            let cache_key = if let Some(path) = path {
                let safe_path = path.replace(['/', '.'], "-");
                format!("{}-path-{}", base_key, safe_path)
            } else {
                base_key
            };
            self.cache_root.join(cache_key)
        }

        fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
            Ok(self.filesystem.clone())
        }

        fn load_from_cache_with_path(
            &self,
            _cache_path: &Path,
            path: Option<&str>,
        ) -> Result<MemoryFS> {
            if let Some(path_filter) = path {
                // Apply path filtering to the stored filesystem
                let mut filtered_fs = MemoryFS::new();
                let filter_prefix = format!("{}/", path_filter.trim_matches('/'));

                for (file_path, file) in self.filesystem.files() {
                    if file_path.starts_with(&filter_prefix) {
                        // Calculate the relative path from the filter
                        let relative_path =
                            file_path.strip_prefix(&filter_prefix).unwrap_or(file_path);

                        // Skip empty paths (directories themselves)
                        if relative_path.as_os_str().is_empty() {
                            continue;
                        }

                        filtered_fs.add_file(relative_path, file.clone())?;
                    }
                }

                Ok(filtered_fs)
            } else {
                // No path filter - return full filesystem
                Ok(self.filesystem.clone())
            }
        }

        fn save_to_cache(&self, cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
            self.cached_repos
                .lock()
                .unwrap()
                .push(cache_path.to_path_buf());
            Ok(())
        }
    }

    #[test]
    fn test_fetch_repository_not_cached() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = manager.fetch_repository("https://github.com/test/repo", "main");
        assert!(result.is_ok());

        // Verify clone was called
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "https://github.com/test/repo");
        assert_eq!(calls[0].1, "main");
    }

    #[test]
    fn test_fetch_repository_already_cached() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();

        // Pre-populate cache
        let cache_path = PathBuf::from("/mock/cache/https---github.com-test-repo-main");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = manager.fetch_repository("https://github.com/test/repo", "main");
        assert!(result.is_ok());

        // Verify clone was NOT called
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_fetch_repository_fresh_always_clones() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();

        // Pre-populate cache
        let cache_path = PathBuf::from("/mock/cache/https---github.com-test-repo-main");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = manager.fetch_repository_fresh("https://github.com/test/repo", "main");
        assert!(result.is_ok());

        // Verify clone WAS called even though cached
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_is_cached() {
        let git_ops = Box::new(MockGitOperations::new());
        let cache_path = PathBuf::from("/mock/cache/https---github.com-test-repo-main");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        assert!(manager.is_cached("https://github.com/test/repo", "main"));
        assert!(!manager.is_cached("https://github.com/test/repo", "develop"));
    }

    #[test]
    fn test_list_repository_tags() {
        let git_ops = Box::new(MockGitOperations::new());
        let cache_ops = Box::new(MockCacheOperations::new());

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let tags = manager
            .list_repository_tags("https://github.com/test/repo")
            .unwrap();
        assert_eq!(tags, vec!["v1.0.0", "v2.0.0"]);
    }

    #[test]
    fn test_clone_error_propagates() {
        let git_ops = Box::new(MockGitOperations::with_error("Network error".to_string()));
        let cache_ops = Box::new(MockCacheOperations::new());

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = manager.fetch_repository("https://github.com/test/repo", "main");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network error"));
    }

    #[test]
    fn test_fetch_repository_with_path_not_cached() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result =
            manager.fetch_repository_with_path("https://github.com/test/repo", "main", Some("uv"));
        assert!(result.is_ok());

        // Verify clone was called
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "https://github.com/test/repo");
        assert_eq!(calls[0].1, "main");
    }

    #[test]
    fn test_fetch_repository_with_path_already_cached() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();

        // Pre-populate cache with path-specific cache key
        let cache_path = PathBuf::from("/mock/cache/https---github.com-test-repo-main-path-uv");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result =
            manager.fetch_repository_with_path("https://github.com/test/repo", "main", Some("uv"));
        assert!(result.is_ok());

        // Verify clone was NOT called
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_fetch_repository_fresh_with_path_always_clones() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();

        // Pre-populate cache
        let cache_path = PathBuf::from("/mock/cache/https---github.com-test-repo-main-path-uv");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = manager.fetch_repository_fresh_with_path(
            "https://github.com/test/repo",
            "main",
            Some("uv"),
        );
        assert!(result.is_ok());

        // Verify clone WAS called even though cached
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_is_cached_with_path() {
        let git_ops = Box::new(MockGitOperations::new());

        // Cache full repository
        let cache_path_full = PathBuf::from("/mock/cache/https---github.com-test-repo-main");
        // Cache path-filtered repository
        let cache_path_uv = PathBuf::from("/mock/cache/https---github.com-test-repo-main-path-uv");

        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![
            cache_path_full.clone(),
            cache_path_uv.clone(),
        ]));

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        // Test full repository
        assert!(manager.is_cached("https://github.com/test/repo", "main"));
        assert!(manager.is_cached_with_path("https://github.com/test/repo", "main", None));

        // Test path-filtered repository
        assert!(manager.is_cached_with_path("https://github.com/test/repo", "main", Some("uv")));
        assert!(!manager.is_cached_with_path(
            "https://github.com/test/repo",
            "main",
            Some("django")
        ));
    }

    #[test]
    fn test_backward_compatibility_without_path() {
        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());

        let manager = RepositoryManager::with_operations(git_ops, cache_ops);

        // Test that old method still works
        let result = manager.fetch_repository("https://github.com/test/repo", "main");
        assert!(result.is_ok());

        // Test that is_cached still works
        assert!(!manager.is_cached("https://github.com/test/repo", "main"));

        // Verify clone was called
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
    }
}
