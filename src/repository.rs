//! # High-Level Repository Management
//!
//! This module provides the `RepositoryManager`, a high-level interface for
//! fetching and caching Git repositories. It is designed to abstract away the
//! underlying details of Git operations and filesystem management, providing a
//! clean and consistent API for the rest of the application.
//!
//! ## Design
//!
//! The `RepositoryManager` is built around a trait-based design that separates
//! the logic of repository management from the concrete implementation of Git
//! and cache operations. This is achieved through two key traits:
//!
//! - **`GitOperations`**: Defines the interface for Git-related actions, such as
//!   cloning a repository and listing its tags.
//!
//! - **`CacheOperations`**: Defines the interface for cache-related actions, such
//!   as checking for the existence of a cached item, generating cache paths,
//!   and loading from or saving to the cache.
//!
//! This design allows for the underlying implementations to be swapped out,
//! which is particularly useful for testing. In the main application,
//! `DefaultGitOperations` and `DefaultCacheOperations` are used, which wrap the
//! actual `git` command and filesystem operations. In tests, these can be
//! replaced with mock implementations to simulate various scenarios without
//! performing real Git operations or touching the filesystem.

use crate::error::Result;
use crate::filesystem::MemoryFS;
use std::path::{Path, PathBuf};

/// Trait for git operations - allows mocking in tests
#[allow(dead_code)]
pub trait GitOperations: Send + Sync {
    /// Clones a repository at a specific Git reference (branch, tag, or commit).
    ///
    /// This is expected to be a shallow clone to optimize for speed and disk
    /// space.
    fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()>;

    /// Retrieves a list of all tags from a remote repository.
    fn list_tags(&self, url: &str) -> Result<Vec<String>>;
}

/// A trait that defines the interface for cache operations.
///
/// This allows for the caching logic to be mocked in tests, enabling the
/// simulation of cache hits and misses without actual filesystem interaction.
#[allow(dead_code)]
pub trait CacheOperations: Send + Sync {
    /// Check if a cached repository exists
    fn exists(&self, cache_path: &Path) -> bool;

    /// Get the cache path for a repository
    fn get_cache_path(&self, url: &str, ref_name: &str) -> PathBuf;

    /// Generates the cache path for a repository, optionally including a
    /// sub-path to ensure uniqueness.
    fn get_cache_path_with_path(&self, url: &str, ref_name: &str, _path: Option<&str>) -> PathBuf {
        // Default implementation: ignore path for backward compatibility
        self.get_cache_path(url, ref_name)
    }

    /// Load a repository from cache into MemoryFS
    fn load_from_cache(&self, cache_path: &Path) -> Result<MemoryFS>;

    /// Loads a repository from the cache into a `MemoryFS`, with an option to
    /// filter by a sub-path.
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

/// The default implementation of `GitOperations`, which uses the system's
/// `git` command to perform real Git operations.
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

/// The default implementation of `CacheOperations`, which interacts with the
/// host filesystem to manage the repository cache.
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

/// The main entry point for managing repositories.
///
/// This struct orchestrates the Git and cache operations to provide a simple,
/// high-level API for fetching repositories.
#[allow(dead_code)]
pub struct RepositoryManager {
    git_ops: Box<dyn GitOperations>,
    cache_ops: Box<dyn CacheOperations>,
}

impl RepositoryManager {
    /// Creates a new `RepositoryManager` with the default Git and cache
    /// operations, using the specified `cache_root` for the on-disk cache.
    #[allow(dead_code)]
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            git_ops: Box::new(DefaultGitOperations),
            cache_ops: Box::new(DefaultCacheOperations::new(cache_root)),
        }
    }

    /// Creates a `RepositoryManager` with custom `GitOperations` and
    /// `CacheOperations` implementations.
    ///
    /// This is primarily used for testing to inject mock operations.
    #[cfg(test)]
    pub fn with_operations(
        git_ops: Box<dyn GitOperations>,
        cache_ops: Box<dyn CacheOperations>,
    ) -> Self {
        Self { git_ops, cache_ops }
    }

    /// Fetches a repository, using the cache if a valid entry is available.
    ///
    /// This method will:
    /// 1.  Check if the repository is already in the on-disk cache.
    /// 2.  If not, it will perform a shallow clone of the repository into the
    ///     cache directory.
    /// 3.  Finally, it will load the repository's contents from the cache into
    ///     a `MemoryFS`.
    #[allow(dead_code)]
    pub fn fetch_repository(&self, url: &str, ref_name: &str) -> Result<MemoryFS> {
        self.fetch_repository_with_path(url, ref_name, None)
    }

    /// Fetches a repository with optional sub-path filtering, using the cache
    /// if a valid entry is available.
    ///
    /// This method extends `fetch_repository` by allowing a sub-path within the
    /// repository to be specified. If a path is provided, only the contents of
    /// that path will be loaded, and the paths in the resulting `MemoryFS` will
    /// be relative to that sub-path.
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

    /// Fetches a repository, bypassing any existing cache entries.
    ///
    /// This method will always perform a fresh, shallow clone of the repository
    /// and will overwrite any existing entry in the on-disk cache.
    #[allow(dead_code)]
    pub fn fetch_repository_fresh(&self, url: &str, ref_name: &str) -> Result<MemoryFS> {
        self.fetch_repository_fresh_with_path(url, ref_name, None)
    }

    /// Fetches a repository with optional sub-path filtering, bypassing any
    /// existing cache entries.
    ///
    /// This method is the "fresh" equivalent of `fetch_repository_with_path`,
    /// ensuring that the latest version of the repository is cloned.
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

    /// Checks if a repository is present in the on-disk cache.
    #[allow(dead_code)]
    pub fn is_cached(&self, url: &str, ref_name: &str) -> bool {
        self.is_cached_with_path(url, ref_name, None)
    }

    /// Checks if a repository with an optional sub-path is present in the
    /// on-disk cache.
    #[allow(dead_code)]
    pub fn is_cached_with_path(&self, url: &str, ref_name: &str, path: Option<&str>) -> bool {
        let cache_path = self.cache_ops.get_cache_path_with_path(url, ref_name, path);
        self.cache_ops.exists(&cache_path)
    }

    /// Retrieves a list of all available tags for a remote repository.
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
