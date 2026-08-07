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
//!     Tag parsing goes through [`crate::git::parse_semver_tag`], and
//!     [`find_latest_version`] is the single selection path shared with the
//!     `init` and `add` commands, so every code path agrees on which tag is
//!     latest.
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
//! ## Filtering
//!
//! The [`check_updates_filtered`] function allows filtering which repositories
//! are checked using glob patterns. Patterns match against the repository URL
//! (with scheme stripped) combined with any optional path. This is used by the
//! `--filter` flag in the update command.
//!
//! ## `UpdateInfo`
//!
//! The results of the update check are returned in a `Vec<UpdateInfo>`, where
//! each `UpdateInfo` struct contains detailed information about the updates
//! available for a single repository.

use crate::config::{RepoOp, Schema};
use crate::error::Result;
use crate::git::parse_semver_tag;
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
    let result = check_updates_filtered(config, repo_manager, &[])?;
    Ok(result.updates)
}

/// Result of a filtered update check, containing both the updates and filter statistics.
#[derive(Debug)]
pub struct FilteredUpdateResult {
    /// The list of update information for repositories that matched the filter.
    pub updates: Vec<UpdateInfo>,
    /// The number of repositories that were excluded by the filter.
    pub filtered_out_count: usize,
}

/// Checks inherited repositories for newer versions, with optional filtering.
///
/// This variant of [`check_updates`] allows filtering which repositories are
/// checked using glob patterns. Patterns are matched against the repository URL
/// (with scheme stripped) combined with any optional path. Multiple patterns
/// use OR logic (any match passes).
///
/// # Arguments
///
/// * `config` - The configuration schema containing repo operations
/// * `repo_manager` - The repository manager for fetching tag information
/// * `filters` - Glob patterns to filter which repos to check (empty = all)
///
/// # Returns
///
/// A [`FilteredUpdateResult`] containing the updates for matching repos and
/// the count of how many repos were excluded by the filter.
pub fn check_updates_filtered(
    config: &Schema,
    repo_manager: &RepositoryManager,
    filters: &[String],
) -> Result<FilteredUpdateResult> {
    let mut results = Vec::new();
    let mut filtered_out_count = 0;

    // Get all inherited repos from the configuration
    let inherited_repos = collect_inherited_repos(config);

    for repo in inherited_repos {
        // Apply filter if any patterns specified
        if !filters.is_empty() && !matches_filter(&repo, filters) {
            filtered_out_count += 1;
            continue;
        }

        let update_info = check_repo_updates(&repo, repo_manager)?;
        results.push(update_info);
    }

    Ok(FilteredUpdateResult {
        updates: results,
        filtered_out_count,
    })
}

/// Build the match target string for a repo (url without scheme + path).
fn build_match_target(repo: &RepoOp) -> String {
    let base = crate::path::strip_url_scheme(&repo.url);
    match &repo.path {
        Some(path) => format!("{}/{}", base, path.trim_matches('/')),
        None => base.to_string(),
    }
}

/// Check if a repo matches any of the given filter patterns.
fn matches_filter(repo: &RepoOp, filters: &[String]) -> bool {
    let target = build_match_target(repo);
    filters
        .iter()
        .any(|pattern| crate::path::glob_match(pattern, &target).unwrap_or(false))
}

/// Check for updates for a single repository
pub fn check_repo_updates(repo: &RepoOp, repo_manager: &RepositoryManager) -> Result<UpdateInfo> {
    if repo.is_local() {
        return Ok(UpdateInfo {
            url: repo.url.clone(),
            current_ref: String::new(),
            latest_version: None,
            breaking_changes: false,
            compatible_updates: false,
            available_versions: vec![],
        });
    }

    let current_ref = repo.r#ref.as_deref().unwrap_or("");

    // List all tags for this repository
    let tags = repo_manager.list_repository_tags(&repo.url)?;

    // Filter to semantic version tags only
    let semver_tags = filter_semver_tags(&tags);

    // Parse current ref if it's a semantic version
    let current_version = parse_semver_tag(current_ref);

    let mut latest_version = None;
    let mut breaking_changes = false;
    let mut compatible_updates = false;

    if let Some(current_ver) = current_version {
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

    Ok(UpdateInfo {
        url: repo.url.clone(),
        current_ref: current_ref.to_string(),
        latest_version,
        breaking_changes,
        compatible_updates,
        available_versions: semver_tags,
    })
}

/// Compare two refs to determine update relationship
pub fn compare_refs(current: &str, available: &[String]) -> Result<(bool, bool)> {
    let current_version = parse_semver_tag(current);

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
        .filter(|tag| parse_semver_tag(tag).is_some())
        .cloned()
        .collect()
}

/// Find the latest semantic version from a list of git tags.
///
/// Tags are parsed with [`crate::git::parse_semver_tag`], so every tag format
/// that function understands (`v1.2.3`, `1.2.3`, `component-v1.2.3`,
/// `refs/tags/v1.2.3`) is considered here. Tags that are not semantic versions
/// are ignored, and `None` is returned when no tag parses.
///
/// The returned tuple pairs the original tag string with its parsed version, so
/// callers can pin a config to the tag as it exists on the remote.
///
/// # Examples
///
/// ```
/// use common_repo::version::find_latest_version;
///
/// let tags = vec!["v1.0.0".to_string(), "v2.0.0".to_string()];
/// let (tag, version) = find_latest_version(&tags).unwrap();
/// assert_eq!(tag, "v2.0.0");
/// assert_eq!(version.major, 2);
/// ```
pub fn find_latest_version(tags: &[String]) -> Option<(String, Version)> {
    let mut latest: Option<(String, Version)> = None;

    for tag in tags {
        if let Some(version) = parse_semver_tag(tag) {
            if let Some((_, ref latest_ver)) = latest {
                if version > *latest_ver {
                    latest = Some((tag.clone(), version));
                }
            } else {
                latest = Some((tag.clone(), version));
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
        match op {
            crate::config::Operation::Repo { repo } => {
                repos.push(repo.clone());
                collect_repos_from_operations(&repo.with, repos);
            }
            crate::config::Operation::Self_ { self_ } => {
                collect_repos_from_operations(&self_.operations, repos);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RepoOp;

    #[test]
    fn test_collect_repos_includes_self_block_repos() {
        use crate::config::{Operation, SelfOp};

        let config = vec![
            Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/example/source".to_string(),
                    r#ref: Some("v1.0.0".to_string()),
                    path: None,
                    with: vec![],
                },
            },
            Operation::Self_ {
                self_: SelfOp {
                    operations: vec![Operation::Repo {
                        repo: RepoOp {
                            url: "https://github.com/example/tooling".to_string(),
                            r#ref: Some("v2.0.0".to_string()),
                            path: None,
                            with: vec![],
                        },
                    }],
                },
            },
        ];

        let repos = collect_inherited_repos(&config);
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].url, "https://github.com/example/source");
        assert_eq!(repos[1].url, "https://github.com/example/tooling");
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
    fn test_filter_semver_tags_component_prefixed() {
        let tags = vec![
            "release-v2.0.0".to_string(),
            "refs/tags/v1.0.0".to_string(),
            "main".to_string(),
        ];

        let filtered = filter_semver_tags(&tags);
        assert_eq!(filtered, vec!["release-v2.0.0", "refs/tags/v1.0.0"]);
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
    fn test_find_latest_version_empty() {
        let tags: Vec<String> = vec![];
        assert!(find_latest_version(&tags).is_none());
    }

    #[test]
    fn test_find_latest_version_no_semver() {
        let tags = vec!["main".to_string(), "develop".to_string()];
        assert!(find_latest_version(&tags).is_none());
    }

    #[test]
    fn test_find_latest_version_zero_major() {
        // 0.x.x versions are valid semver and should be returned
        let tags = vec![
            "v0.1.0".to_string(),
            "v0.5.0".to_string(),
            "v0.2.3".to_string(),
        ];

        let (latest_tag, latest_ver) = find_latest_version(&tags).unwrap();
        assert_eq!(latest_tag, "v0.5.0");
        assert_eq!(latest_ver, Version::parse("0.5.0").unwrap());
    }

    #[test]
    fn test_find_latest_version_mixed_zero_and_stable() {
        // Stable versions should be preferred over 0.x.x
        let tags = vec![
            "v0.9.9".to_string(),
            "v1.0.0".to_string(),
            "v0.10.0".to_string(),
        ];

        let (latest_tag, latest_ver) = find_latest_version(&tags).unwrap();
        assert_eq!(latest_tag, "v1.0.0");
        assert_eq!(latest_ver, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn test_find_latest_version_component_prefixed_tag() {
        let tags = vec!["v1.0.0".to_string(), "release-v2.0.0".to_string()];

        let (latest_tag, latest_ver) = find_latest_version(&tags).unwrap();
        assert_eq!(latest_tag, "release-v2.0.0");
        assert_eq!(latest_ver, Version::parse("2.0.0").unwrap());
    }

    #[test]
    fn test_find_latest_version_fully_qualified_ref() {
        let tags = vec![
            "refs/tags/v1.2.3".to_string(),
            "refs/tags/1.0.0".to_string(),
        ];

        let (latest_tag, latest_ver) = find_latest_version(&tags).unwrap();
        assert_eq!(latest_tag, "refs/tags/v1.2.3");
        assert_eq!(latest_ver, Version::parse("1.2.3").unwrap());
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
                    r#ref: Some("v1.0.0".to_string()),
                    with: vec![crate::config::Operation::Repo {
                        repo: RepoOp {
                            url: "https://github.com/org/repo2.git".to_string(),
                            path: None,
                            r#ref: Some("main".to_string()),
                            with: vec![],
                        },
                    }],
                },
            },
            crate::config::Operation::Include {
                include: crate::config::IncludeOp {
                    patterns: vec!["*.md".to_string()],
                    if_exists: crate::config::IfExists::Overwrite,
                },
                if_exists: crate::config::IfExists::Overwrite,
            },
        ];

        let repos = collect_inherited_repos(&config);
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].url, "https://github.com/org/repo1.git");
        assert_eq!(repos[1].url, "https://github.com/org/repo2.git");
    }

    #[test]
    fn test_build_match_target_url_only() {
        let repo = RepoOp {
            url: "https://github.com/org/repo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert_eq!(build_match_target(&repo), "github.com/org/repo");
    }

    #[test]
    fn test_build_match_target_url_with_path() {
        let repo = RepoOp {
            url: "https://github.com/org/monorepo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: Some("configs/eslint".to_string()),
            with: vec![],
        };
        assert_eq!(
            build_match_target(&repo),
            "github.com/org/monorepo/configs/eslint"
        );
    }

    #[test]
    fn test_build_match_target_strips_various_schemes() {
        // Test https
        let repo = RepoOp {
            url: "https://github.com/org/repo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert_eq!(build_match_target(&repo), "github.com/org/repo");

        // Test git://
        let repo = RepoOp {
            url: "git://gitlab.com/org/repo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert_eq!(build_match_target(&repo), "gitlab.com/org/repo");

        // Test ssh://
        let repo = RepoOp {
            url: "ssh://git@github.com/org/repo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert_eq!(build_match_target(&repo), "git@github.com/org/repo");
    }

    #[test]
    fn test_build_match_target_trims_path_slashes() {
        let repo = RepoOp {
            url: "https://github.com/org/repo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: Some("/configs/eslint/".to_string()),
            with: vec![],
        };
        assert_eq!(
            build_match_target(&repo),
            "github.com/org/repo/configs/eslint"
        );
    }

    #[test]
    fn test_matches_filter_single_pattern() {
        let repo = RepoOp {
            url: "https://github.com/org/ci-templates".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };

        // Exact match
        assert!(matches_filter(
            &repo,
            &["github.com/org/ci-templates".to_string()]
        ));

        // Wildcard match
        assert!(matches_filter(&repo, &["github.com/org/*".to_string()]));
        assert!(matches_filter(&repo, &["*/*/ci-*".to_string()]));

        // No match
        assert!(!matches_filter(&repo, &["gitlab.com/*".to_string()]));
        assert!(!matches_filter(&repo, &["*/*/linter-*".to_string()]));
    }

    #[test]
    fn test_matches_filter_multiple_patterns_or_logic() {
        let repo = RepoOp {
            url: "https://github.com/org/ci-templates".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };

        // First pattern matches
        assert!(matches_filter(
            &repo,
            &["*/*/ci-*".to_string(), "*/*/linter-*".to_string()]
        ));

        // Second pattern matches
        let repo2 = RepoOp {
            url: "https://github.com/org/linter-config".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert!(matches_filter(
            &repo2,
            &["*/*/ci-*".to_string(), "*/*/linter-*".to_string()]
        ));

        // Neither pattern matches
        let repo3 = RepoOp {
            url: "https://github.com/org/utils".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: None,
            with: vec![],
        };
        assert!(!matches_filter(
            &repo3,
            &["*/*/ci-*".to_string(), "*/*/linter-*".to_string()]
        ));
    }

    #[test]
    fn test_matches_filter_with_path() {
        let repo = RepoOp {
            url: "https://github.com/org/monorepo".to_string(),
            r#ref: Some("v1.0.0".to_string()),
            path: Some("configs/eslint".to_string()),
            with: vec![],
        };

        // Match full path
        assert!(matches_filter(
            &repo,
            &["github.com/org/monorepo/configs/eslint".to_string()]
        ));

        // Match with wildcard in path
        assert!(matches_filter(
            &repo,
            &["github.com/org/monorepo/configs/*".to_string()]
        ));

        // Match with ** for any depth
        assert!(matches_filter(
            &repo,
            &["github.com/org/monorepo/**".to_string()]
        ));
    }

    #[test]
    fn check_repo_updates_skips_local() {
        use crate::config::RepoOp;
        use std::sync::{Arc, Mutex};

        struct CountingGit {
            tag_calls: Arc<Mutex<u32>>,
        }
        impl crate::repository::GitOperations for CountingGit {
            fn clone_shallow(
                &self,
                _: &str,
                _: &str,
                _: &std::path::Path,
            ) -> crate::error::Result<()> {
                Ok(())
            }
            fn list_tags(&self, _: &str) -> crate::error::Result<Vec<String>> {
                *self.tag_calls.lock().unwrap() += 1;
                Ok(vec![])
            }
        }
        struct NullCache;
        impl crate::repository::CacheOperations for NullCache {
            fn exists(&self, _: &std::path::Path) -> bool {
                false
            }
            fn get_cache_path(&self, _: &str, _: &str) -> std::path::PathBuf {
                std::path::PathBuf::new()
            }
            fn load_from_cache(
                &self,
                _: &std::path::Path,
            ) -> crate::error::Result<crate::filesystem::MemoryFS> {
                Ok(crate::filesystem::MemoryFS::new())
            }
            fn save_to_cache(
                &self,
                _: &std::path::Path,
                _: &crate::filesystem::MemoryFS,
            ) -> crate::error::Result<()> {
                Ok(())
            }
        }

        let calls = Arc::new(Mutex::new(0));
        let git = Box::new(CountingGit {
            tag_calls: calls.clone(),
        });
        let cache = Box::new(NullCache);
        let manager = crate::repository::RepositoryManager::with_operations(git, cache);

        let repo = RepoOp {
            url: "./local".to_string(),
            r#ref: None,
            path: None,
            with: vec![],
        };
        let info = check_repo_updates(&repo, &manager).unwrap();
        assert_eq!(info.url, "./local");
        assert_eq!(info.current_ref, "");
        assert!(info.latest_version.is_none());
        assert!(!info.breaking_changes);
        assert!(!info.compatible_updates);
        assert!(info.available_versions.is_empty());
        assert_eq!(*calls.lock().unwrap(), 0, "list_tags must not be called");
    }
}
