//! Default values for common-repo configuration.
//!
//! This module provides centralized default values used across commands,
//! ensuring consistency and avoiding duplication.

use std::path::PathBuf;

/// Returns the default cache root directory.
///
/// Uses the platform-appropriate cache directory:
/// - Linux: `~/.cache/common-repo` (XDG Base Directory)
/// - macOS: `~/Library/Caches/common-repo`
/// - Windows: `{FOLDERID_LocalAppData}\common-repo`
///
/// Falls back to `.common-repo-cache` in the current directory if the
/// platform cache directory cannot be determined.
///
/// This can be overridden by the `--cache-root` CLI flag or the
/// `COMMON_REPO_CACHE` environment variable.
pub fn default_cache_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".common-repo-cache"))
        .join("common-repo")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cache_root_returns_path() {
        let cache_root = default_cache_root();
        // Should end with "common-repo"
        assert!(cache_root.ends_with("common-repo"));
    }

    #[test]
    fn test_default_cache_root_is_absolute_or_fallback() {
        let cache_root = default_cache_root();
        // Either absolute (normal case) or relative fallback
        assert!(
            cache_root.is_absolute() || cache_root.starts_with(".common-repo-cache"),
            "Expected absolute path or fallback, got: {:?}",
            cache_root
        );
    }
}
