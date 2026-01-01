//! # Version Detection and Update Checking
//!
//! This module provides the core functionality for detecting when inherited
//! repositories have newer versions available. It is used by the `check` and
//! `update` subcommands to inform the user about available updates.
//!
//! ## Process
//!
//! 1.  **Repository Collection**: The process begins by collecting all the `repo`
//!     operations from the configuration, including any nested `repo` operations
//!     within `with:` clauses.
//!
//! 2.  **Tag Fetching**: For each repository, it queries the remote Git repository
//!     to get a list of all available tags.
//!
//! 3.  **Semantic Version Filtering**: The list of tags is filtered to include
//!     only those that conform to the semantic versioning (semver) specification.
//!
//! 4.  **Version Comparison**: If the current `ref` for a repository is also a
//!     valid semantic version, it is compared against the latest available
//!     semver tag.
//!
//! 5.  **Update Categorization**: Any available updates are categorized as either:
//!     - **Breaking Changes**: If the major version number has increased (e.g.,
//!       `v1.2.3` to `v2.0.0`).
//!     - **Compatible Updates**: If the minor or patch version number has
//!       increased (e.g., `v1.2.3` to `v1.3.0` or `v1.2.4`).
//!
//! ## `UpdateInfo`
//!
//! The results of the update check are returned in a `Vec<UpdateInfo>`, where
//! each `UpdateInfo` struct contains detailed information about the updates
//! available for a single repository.

use crate::config::{RepoOp, Schema};
use crate::error::Result;
use crate::repository::RepositoryManager;
use semver::Version;

/// Information about available updates for a repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The URL of the repository that was checked.
    pub url: String,
    /// The current Git reference (e.g., tag, branch) being used for the
    /// repository.
    pub current_ref: String,
    /// The latest semantic version tag found for the repository, if any.
    pub latest_version: Option<String>,
    /// A flag indicating whether the latest version includes breaking changes
    /// (i.e., a major version bump).
    pub breaking_changes: bool,
    /// A flag indicating whether the latest version is a compatible update
    /// (i.e., a minor or patch version bump).
    pub compatible_updates: bool,
    /// A list of all the tags from the repository that were identified as valid
    /// semantic versions.
    pub available_versions: Vec<String>,
}

/// Checks all inherited repositories in a configuration for newer versions.
///
/// This function serves as the main entry point for the update-checking
/// process. It collects all the `repo` operations from the given `Schema` and
/// then checks each one for available updates.
pub fn check_updates(config: &Schema, repo_manager: &RepositoryManager) -> Result<Vec<UpdateInfo>> {
    let mut results = Vec::new();

    // Get all inherited repos from the configuration
    let inherited_repos = collect_inherited_repos(config);

    for repo in inherited_repos {
        let update_info = check_repo_updates(&repo, repo_manager)?;
        results.push(update_info);
    }

    Ok(results)
}

/// Check for updates for a single repository
pub fn check_repo_updates(repo: &RepoOp, repo_manager: &RepositoryManager) -> Result<UpdateInfo> {
    let current_ref = &repo.r#ref;

    // List all tags for this repository
    let tags = repo_manager.list_repository_tags(&repo.url)?;

    // Filter to semantic version tags only
    let semver_tags = filter_semver_tags(&tags);

    // Parse current ref if it's a semantic version
    let current_version = extract_semver_from_ref(current_ref);

    let mut latest_version = None;
    let mut breaking_changes = false;
    let mut compatible_updates = false;

    if let Some(current_ver_str) = current_version {
        if let Ok(current_ver) = Version::parse(&current_ver_str) {
            // Find the latest version
            if let Some((latest_tag, latest_ver)) = find_latest_version(&semver_tags) {
                latest_version = Some(latest_tag.clone());

                // Compare versions
                match latest_ver.cmp(&current_ver) {
                    std::cmp::Ordering::Greater => {
                        // Latest is newer, check if it's a breaking change
                        if latest_ver.major > current_ver.major {
                            breaking_changes = true;
                        } else {
                            compatible_updates = true;
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        // Same version, no updates
                    }
                    std::cmp::Ordering::Less => {
                        // Current is newer than latest? This shouldn't happen
                        // but we'll treat it as no updates
                    }
                }
            }
        }
    }

    Ok(UpdateInfo {
        url: repo.url.clone(),
        current_ref: current_ref.clone(),
        latest_version,
        breaking_changes,
        compatible_updates,
        available_versions: semver_tags,
    })
}

/// Compare two refs to determine update relationship
pub fn compare_refs(current: &str, available: &[String]) -> Result<(bool, bool)> {
    let current_version = extract_semver_from_ref(current).and_then(|v| Version::parse(&v).ok());

    if let Some(current_ver) = current_version {
        if let Some((_, latest_ver)) = find_latest_version(available) {
            match latest_ver.cmp(&current_ver) {
                std::cmp::Ordering::Greater => {
                    let breaking = latest_ver.major > current_ver.major;
                    let compatible = !breaking;
                    Ok((breaking, compatible))
                }
                _ => Ok((false, false)),
            }
        } else {
            Ok((false, false))
        }
    } else {
        // Current ref is not a semantic version, can't compare
        Ok((false, false))
    }
}

/// Filter git tags to semantic versions only
pub fn filter_semver_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .filter_map(|tag| {
            extract_semver_from_ref(tag)
                .and_then(|v| Version::parse(&v).ok())
                .map(|_| tag.clone())
        })
        .collect()
}

/// Extract semantic version string from a git reference (tag or ref)
fn extract_semver_from_ref(ref_str: &str) -> Option<String> {
    // Common patterns: v1.2.3, 1.2.3, refs/tags/v1.2.3, refs/tags/1.2.3

    // Strip refs/tags/ prefix if present
    let tag = ref_str.strip_prefix("refs/tags/").unwrap_or(ref_str);

    // Try to extract semantic version
    if let Some(version_str) = tag.strip_prefix('v') {
        // Has 'v' prefix
        Version::parse(version_str)
            .ok()
            .map(|_| version_str.to_string())
    } else {
        // No 'v' prefix
        Version::parse(tag).ok().map(|_| tag.to_string())
    }
}

/// Find the latest version from a list of semantic version tags
fn find_latest_version(tags: &[String]) -> Option<(String, Version)> {
    let mut latest: Option<(String, Version)> = None;

    for tag in tags {
        if let Some(version_str) = extract_semver_from_ref(tag) {
            if let Ok(version) = Version::parse(&version_str) {
                if let Some((_, ref mut latest_ver)) = latest {
                    if version > *latest_ver {
                        latest = Some((tag.clone(), version));
                    }
                } else {
                    latest = Some((tag.clone(), version));
                }
            }
        }
    }

    latest
}

/// Collect all inherited repositories from a configuration
fn collect_inherited_repos(config: &Schema) -> Vec<RepoOp> {
    let mut repos = Vec::new();

    // Recursively collect repos from operations
    collect_repos_from_operations(config, &mut repos);

    repos
}

fn collect_repos_from_operations(operations: &[crate::config::Operation], repos: &mut Vec<RepoOp>) {
    for op in operations {
        if let crate::config::Operation::Repo { repo } = op {
            repos.push(repo.clone());
            // Also collect from with clause
            collect_repos_from_operations(&repo.with, repos);
        }
        // Other operations might contain nested repos, but for now we only handle direct repo ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RepoOp;

    #[test]
    fn test_extract_semver_from_ref() {
        assert_eq!(extract_semver_from_ref("v1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_semver_from_ref("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(
            extract_semver_from_ref("refs/tags/v1.2.3"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            extract_semver_from_ref("refs/tags/1.2.3"),
            Some("1.2.3".to_string())
        );
        assert_eq!(extract_semver_from_ref("main"), None);
        assert_eq!(extract_semver_from_ref("v1.2"), None); // Invalid semver
    }

    #[test]
    fn test_filter_semver_tags() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v1.1.0".to_string(),
            "main".to_string(),
            "v2.0.0".to_string(),
            "invalid".to_string(),
        ];

        let filtered = filter_semver_tags(&tags);
        assert_eq!(filtered, vec!["v1.0.0", "v1.1.0", "v2.0.0"]);
    }

    #[test]
    fn test_find_latest_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v1.1.0".to_string(),
            "v2.0.0".to_string(),
            "v1.5.0".to_string(),
        ];

        let (latest_tag, latest_ver) = find_latest_version(&tags).unwrap();
        assert_eq!(latest_tag, "v2.0.0");
        assert_eq!(latest_ver, Version::parse("2.0.0").unwrap());
    }

    #[test]
    fn test_compare_refs_breaking_change() {
        let available = vec!["v2.0.0".to_string(), "v1.5.0".to_string()];
        let (breaking, compatible) = compare_refs("v1.0.0", &available).unwrap();
        assert!(breaking);
        assert!(!compatible);
    }

    #[test]
    fn test_compare_refs_compatible_update() {
        let available = vec!["v1.5.0".to_string(), "v1.1.0".to_string()];
        let (breaking, compatible) = compare_refs("v1.0.0", &available).unwrap();
        assert!(!breaking);
        assert!(compatible);
    }

    #[test]
    fn test_compare_refs_no_update() {
        let available = vec!["v1.0.0".to_string(), "v0.9.0".to_string()];
        let (breaking, compatible) = compare_refs("v1.0.0", &available).unwrap();
        assert!(!breaking);
        assert!(!compatible);
    }

    #[test]
    fn test_compare_refs_non_semver() {
        let available = vec!["v1.0.0".to_string()];
        let (breaking, compatible) = compare_refs("main", &available).unwrap();
        assert!(!breaking);
        assert!(!compatible);
    }

    #[test]
    fn test_collect_inherited_repos() {
        let config: Schema = vec![
            crate::config::Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/org/repo1.git".to_string(),
                    path: None,
                    r#ref: "v1.0.0".to_string(),
                    with: vec![crate::config::Operation::Repo {
                        repo: RepoOp {
                            url: "https://github.com/org/repo2.git".to_string(),
                            path: None,
                            r#ref: "main".to_string(),
                            with: vec![],
                        },
                    }],
                },
            },
            crate::config::Operation::Include {
                include: crate::config::IncludeOp {
                    patterns: vec!["*.md".to_string()],
                },
            },
        ];

        let repos = collect_inherited_repos(&config);
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].url, "https://github.com/org/repo1.git");
        assert_eq!(repos[1].url, "https://github.com/org/repo2.git");
    }
}
