use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;
use crate::filesystem::{File, MemoryFS};
use semver::Version;

/// Clone a repository at a specific ref using shallow clone
#[allow(dead_code)]
pub fn clone_shallow(url: &str, ref_name: &str, target_dir: &Path) -> Result<(), Error> {
    // Create target directory if it doesn't exist
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)?;
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
        return Err(Error::GitClone {
            url: url.to_string(),
            r#ref: ref_name.to_string(),
            message: stderr.to_string(),
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

    // Note: Integration tests for clone_shallow and list_tags would require
    // actual git repositories and network access, so they're omitted for now
}
