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
    let mut fs = MemoryFS::new();

    fn load_directory(dir: &Path, base_path: &Path, fs: &mut MemoryFS) -> Result<(), Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path.strip_prefix(base_path).unwrap();

            if path.is_dir() {
                // Skip .git directory
                if !path.ends_with(".git") {
                    load_directory(&path, base_path, fs)?;
                }
            } else {
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
        Ok(())
    }

    load_directory(cache_dir, cache_dir, &mut fs)?;
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
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create a hash of the URL for filesystem-safe directory name
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let url_hash = format!("{:x}", hasher.finish());

    // Sanitize ref name for filesystem (replace / with -)
    let safe_ref = ref_name.replace('/', "-");

    cache_root.join(format!("{}-{}", url_hash, safe_ref))
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

    // Note: Integration tests for clone_shallow and list_tags would require
    // actual git repositories and network access, so they're omitted for now
}
