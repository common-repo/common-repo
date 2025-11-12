use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;
use crate::filesystem::{File, MemoryFS};
use semver::Version;

/// Clone a repository at a specific ref using shallow clone
///
/// This uses the system git command, which automatically handles:
/// - SSH keys from ~/.ssh/
/// - Git credential helpers
/// - Personal access tokens
/// - Any authentication configured in ~/.gitconfig
#[allow(dead_code)]
pub fn clone_shallow(url: &str, ref_name: &str, target_dir: &Path) -> Result<(), Error> {
    // Remove target directory if it exists (git won't clone into existing non-empty dir)
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)?;
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    // Execute git clone --depth=1 --branch <ref> <url> <target_dir>
    let output = Command::new("git")
        .args(["clone", "--depth=1", "--branch", ref_name, url])
        .arg(target_dir)
        .output()
        .map_err(|e| Error::GitClone {
            url: url.to_string(),
            r#ref: ref_name.to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Provide helpful error message for common auth failures
        let message = if stderr.contains("Authentication failed")
            || stderr.contains("Permission denied")
            || stderr.contains("Could not read from remote repository")
        {
            format!(
                "Authentication failed. Make sure you have access to the repository.\n\
                For private repos, ensure you have:\n\
                - SSH key added to ssh-agent\n\
                - Git credentials configured\n\
                - Personal access token set up\n\
                Error: {}",
                stderr
            )
        } else {
            stderr.to_string()
        };

        return Err(Error::GitClone {
            url: url.to_string(),
            r#ref: ref_name.to_string(),
            message,
        });
    }

    Ok(())
}

/// Load a cached repository into MemoryFS
#[allow(dead_code)]
pub fn load_from_cache(cache_dir: &Path) -> Result<MemoryFS, Error> {
    load_from_cache_with_path(cache_dir, None)
}

/// Load a cached repository into MemoryFS with optional path filtering
///
/// When a path is specified, only files under that sub-directory are loaded,
/// and the specified path becomes the effective filesystem root.
#[allow(dead_code)]
pub fn load_from_cache_with_path(cache_dir: &Path, path: Option<&str>) -> Result<MemoryFS, Error> {
    let mut fs = MemoryFS::new();

    // Normalize path: remove leading/trailing slashes, handle empty/None
    let filter_path = path
        .filter(|p| !p.is_empty() && *p != "." && *p != "/")
        .map(|p| PathBuf::from(p.trim_matches('/')));

    fn load_directory(
        dir: &Path,
        base_path: &Path,
        fs: &mut MemoryFS,
        filter_path: Option<&PathBuf>,
    ) -> Result<(), Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path.strip_prefix(base_path).unwrap();

            if path.is_dir() {
                // Skip .git directory
                if !path.ends_with(".git") {
                    load_directory(&path, base_path, fs, filter_path)?;
                }
            } else {
                // Apply path filtering if specified
                if let Some(filter) = filter_path {
                    // Check if this file is under the filter path
                    if !relative_path.starts_with(filter) {
                        continue;
                    }
                    // Remap the path to be relative to the filter path
                    let remapped_path = relative_path.strip_prefix(filter).unwrap_or(relative_path);
                    if remapped_path.as_os_str().is_empty() {
                        continue; // Skip the root directory itself
                    }

                    // Read file content
                    let content = fs::read(&path)?;
                    let metadata = entry.metadata()?;

                    // Create File with basic metadata
                    let file = File {
                        content,
                        permissions: 0o644, // Default permissions, TODO: Check actual permissions
                        modified_time: metadata
                            .modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    };

                    fs.add_file(remapped_path, file)?;
                } else {
                    // No filtering - load all files as-is
                    // Read file content
                    let content = fs::read(&path)?;
                    let metadata = entry.metadata()?;

                    // Create File with basic metadata
                    let file = File {
                        content,
                        permissions: 0o644, // Default permissions, TODO: Check actual permissions
                        modified_time: metadata
                            .modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    };

                    fs.add_file(relative_path, file)?;
                }
            }
        }
        Ok(())
    }

    load_directory(cache_dir, cache_dir, &mut fs, filter_path.as_ref())?;
    Ok(fs)
}

/// Save repository to cache directory
#[allow(dead_code)]
pub fn save_to_cache(cache_dir: &Path, fs: &MemoryFS) -> Result<(), Error> {
    // Create cache directory if it doesn't exist
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
    }

    // Write all files from MemoryFS to cache directory
    for (path, file) in fs.files() {
        let full_path = cache_dir.join(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, &file.content)?;

        // TODO: Set file permissions based on file.permissions
        // Currently we don't set permissions, just write the content
    }

    Ok(())
}

/// Convert URL and ref to cache path
#[allow(dead_code)]
pub fn url_to_cache_path(cache_root: &Path, url: &str, ref_name: &str) -> PathBuf {
    url_to_cache_path_with_path(cache_root, url, ref_name, None)
}

/// Convert URL, ref, and optional sub-path to cache path
#[allow(dead_code)]
pub fn url_to_cache_path_with_path(
    cache_root: &Path,
    url: &str,
    ref_name: &str,
    path: Option<&str>,
) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create a hash of the URL for filesystem-safe directory name
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let url_hash = format!("{:x}", hasher.finish());

    // Sanitize ref name for filesystem (replace / with -)
    let safe_ref = ref_name.replace('/', "-");

    // Include path in cache key if present and non-empty
    let cache_key = if let Some(path) = path {
        let normalized_path = path.trim_matches('/').trim();
        if normalized_path.is_empty() || normalized_path == "." {
            format!("{}-{}", url_hash, safe_ref)
        } else {
            let safe_path = normalized_path.replace(['/', '.'], "-");
            format!("{}-{}-path-{}", url_hash, safe_ref, safe_path)
        }
    } else {
        format!("{}-{}", url_hash, safe_ref)
    };

    cache_root.join(cache_key)
}

/// List all tags from a remote repository
#[allow(dead_code)]
pub fn list_tags(url: &str) -> Result<Vec<String>, Error> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .output()
        .map_err(|e| Error::GitCommand {
            command: "ls-remote --tags".to_string(),
            url: url.to_string(),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::GitCommand {
            command: "ls-remote --tags".to_string(),
            url: url.to_string(),
            stderr: stderr.to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tags: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            // Git ls-remote output format: <hash>\t<ref>
            // Tags look like: refs/tags/v1.0.0
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 2 {
                let ref_name = parts[1];
                ref_name
                    .strip_prefix("refs/tags/")
                    .map(|tag| tag.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(tags)
}

/// Parse a tag string into a semantic version
#[allow(dead_code)]
pub fn parse_semver_tag(tag: &str) -> Option<Version> {
    // Common tag formats: v1.0.0, 1.0.0, v1.0, 1.0
    let version_str = if let Some(stripped) = tag.strip_prefix('v') {
        stripped
    } else {
        tag
    };

    Version::parse(version_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_url_to_cache_path() {
        let cache_root = PathBuf::from("/tmp/cache");
        let url = "https://github.com/example/repo.git";
        let ref_name = "v1.0.0";

        let cache_path = url_to_cache_path(&cache_root, url, ref_name);
        assert!(cache_path.starts_with(&cache_root));
        assert!(cache_path.to_string_lossy().contains("v1.0.0"));
    }

    #[test]
    fn test_url_to_cache_path_with_slashes() {
        let cache_root = PathBuf::from("/tmp/cache");
        let url = "https://github.com/example/repo.git";
        let ref_name = "feature/some-branch";

        let cache_path = url_to_cache_path(&cache_root, url, ref_name);
        assert!(cache_path.starts_with(&cache_root));
        // Slashes should be replaced with dashes
        assert!(cache_path.to_string_lossy().contains("feature-some-branch"));
    }

    #[test]
    fn test_url_to_cache_path_different_urls() {
        let cache_root = PathBuf::from("/tmp/cache");

        let path1 = url_to_cache_path(&cache_root, "https://github.com/user1/repo.git", "main");
        let path2 = url_to_cache_path(&cache_root, "https://github.com/user2/repo.git", "main");

        // Different URLs should produce different paths
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_url_to_cache_path_with_path() {
        let cache_root = PathBuf::from("/tmp/cache");
        let url = "https://github.com/example/repo.git";
        let ref_name = "main";

        let path_no_filter = url_to_cache_path_with_path(&cache_root, url, ref_name, None);
        let path_with_filter = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("uv"));

        // Paths should be different when path filter is applied
        assert_ne!(path_no_filter, path_with_filter);

        // Both should start with cache root
        assert!(path_no_filter.starts_with(&cache_root));
        assert!(path_with_filter.starts_with(&cache_root));

        // Path with filter should contain the path information
        assert!(path_with_filter.to_string_lossy().contains("path-uv"));
    }

    #[test]
    fn test_url_to_cache_path_with_path_normalization() {
        let cache_root = PathBuf::from("/tmp/cache");
        let url = "https://github.com/example/repo.git";
        let ref_name = "main";

        let path1 = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("uv"));
        let path2 = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("uv/"));
        let path3 = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("/uv/"));

        // All should produce the same path (normalized)
        assert_eq!(path1, path2);
        assert_eq!(path2, path3);
    }

    #[test]
    fn test_url_to_cache_path_with_path_empty() {
        let cache_root = PathBuf::from("/tmp/cache");
        let url = "https://github.com/example/repo.git";
        let ref_name = "main";

        let path_none = url_to_cache_path_with_path(&cache_root, url, ref_name, None);
        let path_empty = url_to_cache_path_with_path(&cache_root, url, ref_name, Some(""));
        let path_dot = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("."));
        let path_slash = url_to_cache_path_with_path(&cache_root, url, ref_name, Some("/"));

        // All empty/None path variations should produce the same path as no path
        assert_eq!(path_none, path_empty);
        assert_eq!(path_empty, path_dot);
        assert_eq!(path_dot, path_slash);
    }

    #[test]
    fn test_parse_semver_tag() {
        assert_eq!(
            parse_semver_tag("v1.0.0"),
            Some(Version::parse("1.0.0").unwrap())
        );
        assert_eq!(
            parse_semver_tag("1.0.0"),
            Some(Version::parse("1.0.0").unwrap())
        );
        assert_eq!(parse_semver_tag("invalid"), None);
    }

    #[test]
    fn test_parse_semver_tag_variations() {
        // Test various valid semver formats
        assert_eq!(
            parse_semver_tag("v2.1.3-alpha"),
            Some(Version::parse("2.1.3-alpha").unwrap())
        );
        assert_eq!(
            parse_semver_tag("3.0.0-beta.1"),
            Some(Version::parse("3.0.0-beta.1").unwrap())
        );
        // Note: semver crate requires patch version, so "1.0" is invalid
        assert_eq!(parse_semver_tag("v1.0"), None);
        assert_eq!(parse_semver_tag("2.0"), None);

        // Test invalid formats
        assert_eq!(parse_semver_tag("not-a-version"), None);
        assert_eq!(parse_semver_tag("v"), None);
        assert_eq!(parse_semver_tag(""), None);
        assert_eq!(parse_semver_tag("v1.0.0.0"), None); // Too many components
    }

    #[test]
    fn test_load_from_cache_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let fs = load_from_cache(cache_dir).unwrap();
        assert!(fs.is_empty());
    }

    #[test]
    fn test_load_from_cache_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create some test files
        fs::create_dir_all(cache_dir.join("subdir")).unwrap();
        fs::write(cache_dir.join("file1.txt"), b"content1").unwrap();
        fs::write(cache_dir.join("file2.txt"), b"content2").unwrap();
        fs::write(cache_dir.join("subdir/file3.txt"), b"content3").unwrap();

        let fs = load_from_cache(cache_dir).unwrap();

        assert_eq!(fs.len(), 3);
        assert!(fs.exists("file1.txt"));
        assert!(fs.exists("file2.txt"));
        assert!(fs.exists("subdir/file3.txt"));

        let file1 = fs.get_file("file1.txt").unwrap();
        assert_eq!(file1.content, b"content1");

        let file3 = fs.get_file("subdir/file3.txt").unwrap();
        assert_eq!(file3.content, b"content3");
    }

    #[test]
    fn test_load_from_cache_skips_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create files including a .git directory that should be skipped
        fs::create_dir_all(cache_dir.join(".git")).unwrap();
        fs::create_dir_all(cache_dir.join("src")).unwrap();
        fs::write(cache_dir.join("README.md"), b"readme").unwrap();
        fs::write(cache_dir.join("src/main.rs"), b"code").unwrap();
        fs::write(cache_dir.join(".git/config"), b"git config").unwrap();

        let fs = load_from_cache(cache_dir).unwrap();

        // Should contain files but not .git directory contents
        assert_eq!(fs.len(), 2);
        assert!(fs.exists("README.md"));
        assert!(fs.exists("src/main.rs"));
        assert!(!fs.exists(".git/config"));
    }

    #[test]
    fn test_save_to_cache_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create a MemoryFS with some files
        let mut fs = MemoryFS::new();
        fs.add_file_string("file1.txt", "content1").unwrap();
        fs.add_file_string("subdir/file2.txt", "content2").unwrap();

        // Save to cache
        save_to_cache(cache_dir, &fs).unwrap();

        // Load back and verify
        let loaded_fs = load_from_cache(cache_dir).unwrap();

        assert_eq!(loaded_fs.len(), 2);
        assert!(loaded_fs.exists("file1.txt"));
        assert!(loaded_fs.exists("subdir/file2.txt"));

        let file1 = loaded_fs.get_file("file1.txt").unwrap();
        assert_eq!(
            String::from_utf8(file1.content.clone()).unwrap(),
            "content1"
        );

        let file2 = loaded_fs.get_file("subdir/file2.txt").unwrap();
        assert_eq!(
            String::from_utf8(file2.content.clone()).unwrap(),
            "content2"
        );
    }

    #[test]
    fn test_save_to_cache_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let mut fs = MemoryFS::new();
        fs.add_file_string("deep/nested/path/file.txt", "content")
            .unwrap();

        save_to_cache(cache_dir, &fs).unwrap();

        // Check that the directory structure was created
        assert!(cache_dir.join("deep/nested/path/file.txt").exists());
        let content = fs::read_to_string(cache_dir.join("deep/nested/path/file.txt")).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn test_save_to_cache_overwrites_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create initial file
        fs::write(cache_dir.join("file.txt"), b"old content").unwrap();

        let mut fs = MemoryFS::new();
        fs.add_file_string("file.txt", "new content").unwrap();

        save_to_cache(cache_dir, &fs).unwrap();

        let content = fs::read_to_string(cache_dir.join("file.txt")).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_load_from_cache_with_path_no_filter() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create test directory structure
        fs::create_dir_all(cache_dir.join("subdir")).unwrap();
        fs::write(cache_dir.join("file1.txt"), b"content1").unwrap();
        fs::write(cache_dir.join("subdir/file2.txt"), b"content2").unwrap();

        // Load without path filter
        let fs = load_from_cache_with_path(cache_dir, None).unwrap();

        assert_eq!(fs.len(), 2);
        assert!(fs.exists("file1.txt"));
        assert!(fs.exists("subdir/file2.txt"));
    }

    #[test]
    fn test_load_from_cache_with_path_filter() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create test directory structure
        fs::create_dir_all(cache_dir.join("uv")).unwrap();
        fs::create_dir_all(cache_dir.join("django")).unwrap();
        fs::write(cache_dir.join("README.md"), b"root readme").unwrap();
        fs::write(cache_dir.join("uv/main.py"), b"uv main").unwrap();
        fs::write(cache_dir.join("uv/lib.py"), b"uv lib").unwrap();
        fs::write(cache_dir.join("django/models.py"), b"django models").unwrap();

        // Load with path filter "uv"
        let fs = load_from_cache_with_path(cache_dir, Some("uv")).unwrap();

        // Should only contain files from uv directory, with paths relative to uv/
        assert_eq!(fs.len(), 2);
        assert!(fs.exists("main.py"));
        assert!(fs.exists("lib.py"));
        assert!(!fs.exists("README.md"));
        assert!(!fs.exists("django/models.py"));

        let main_content = fs.get_file("main.py").unwrap();
        assert_eq!(main_content.content, b"uv main");
    }

    #[test]
    fn test_load_from_cache_with_path_filter_nested() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create nested directory structure
        fs::create_dir_all(cache_dir.join("src").join("python").join("uv")).unwrap();
        fs::create_dir_all(cache_dir.join("src").join("python").join("django")).unwrap();
        fs::write(
            cache_dir
                .join("src")
                .join("python")
                .join("uv")
                .join("main.py"),
            b"uv main",
        )
        .unwrap();
        fs::write(
            cache_dir
                .join("src")
                .join("python")
                .join("uv")
                .join("utils.py"),
            b"uv utils",
        )
        .unwrap();
        fs::write(
            cache_dir
                .join("src")
                .join("python")
                .join("django")
                .join("app.py"),
            b"django app",
        )
        .unwrap();

        // Load with path filter "src/python/uv"
        let fs = load_from_cache_with_path(cache_dir, Some("src/python/uv")).unwrap();

        // Should only contain files from src/python/uv directory
        assert_eq!(fs.len(), 2);
        assert!(fs.exists("main.py"));
        assert!(fs.exists("utils.py"));
        assert!(!fs.exists("src/python/django/app.py"));
    }

    #[test]
    fn test_load_from_cache_with_path_empty_filter() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create test files
        fs::write(cache_dir.join("file1.txt"), b"content1").unwrap();
        fs::write(cache_dir.join("file2.txt"), b"content2").unwrap();

        // Load with empty path filter - should behave like no filter
        let fs1 = load_from_cache_with_path(cache_dir, None).unwrap();
        let fs2 = load_from_cache_with_path(cache_dir, Some("")).unwrap();
        let fs3 = load_from_cache_with_path(cache_dir, Some(".")).unwrap();
        let fs4 = load_from_cache_with_path(cache_dir, Some("/")).unwrap();

        // All should be equivalent to loading without filter
        assert_eq!(fs1.len(), fs2.len());
        assert_eq!(fs2.len(), fs3.len());
        assert_eq!(fs3.len(), fs4.len());
        assert_eq!(fs1.len(), 2);
    }

    #[test]
    fn test_load_from_cache_with_path_nonexistent_filter() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create some files
        fs::write(cache_dir.join("file1.txt"), b"content1").unwrap();

        // Load with path filter for nonexistent directory
        let fs = load_from_cache_with_path(cache_dir, Some("nonexistent")).unwrap();

        // Should return empty filesystem since no files match the filter
        assert_eq!(fs.len(), 0);
    }

    // Note: Integration tests for clone_shallow and list_tags would require
    // actual git repositories and network access, so they're omitted for now
}
