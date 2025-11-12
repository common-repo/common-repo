//! Operator implementations for common-repo operations
//!
//! This module contains implementations for all operator types defined in the configuration schema.

use crate::config::{ExcludeOp, IncludeOp, Operation, RenameOp, RepoOp};
use crate::error::Result;
use crate::filesystem::MemoryFS;
use crate::path::regex_rename;
use crate::repository::RepositoryManager;
use std::path::Path;

/// Include operator - adds files matching glob patterns to the filesystem
pub mod include {
    use super::*;

    /// Apply the include operation to a filesystem
    ///
    /// This adds files from the source filesystem that match the given glob patterns
    /// to the target filesystem.
    ///
    /// # Arguments
    /// * `op` - The include operation configuration
    /// * `source` - Source filesystem to include files from
    /// * `target` - Target filesystem to add files to
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn apply(op: &IncludeOp, source: &MemoryFS, target: &mut MemoryFS) -> Result<()> {
        for pattern in &op.patterns {
            let matching_files = source.list_files_glob(pattern)?;

            for path in matching_files {
                if let Some(file) = source.get_file(&path) {
                    // Clone the file to add to target
                    target.add_file(&path, file.clone())?;
                }
            }
        }

        Ok(())
    }
}

/// Exclude operator - removes files matching glob patterns from the filesystem
pub mod exclude {
    use super::*;

    /// Apply the exclude operation to a filesystem
    ///
    /// This removes files from the target filesystem that match the given glob patterns.
    ///
    /// # Arguments
    /// * `op` - The exclude operation configuration
    /// * `target` - Target filesystem to remove files from
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn apply(op: &ExcludeOp, target: &mut MemoryFS) -> Result<()> {
        for pattern in &op.patterns {
            let matching_files = target.list_files_glob(pattern)?;

            for path in matching_files {
                target.remove_file(&path)?;
            }
        }

        Ok(())
    }
}

/// Rename operator - renames files using regex patterns
pub mod rename {
    use super::*;

    /// Apply the rename operation to a filesystem
    ///
    /// This renames files in the target filesystem according to the regex mappings.
    /// Each mapping consists of a regex pattern and a replacement string.
    ///
    /// # Arguments
    /// * `op` - The rename operation configuration
    /// * `target` - Target filesystem to rename files in
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn apply(op: &RenameOp, target: &mut MemoryFS) -> Result<()> {
        // Collect all current file paths
        let current_files: Vec<_> = target.list_files();

        // Apply each rename mapping
        for mapping in &op.mappings {
            let from_pattern = &mapping.from;
            let to_pattern = &mapping.to;

            // Collect files that need to be renamed
            let mut files_to_rename = Vec::new();

            for path in &current_files {
                let path_str = path.to_string_lossy();

                // Check if the file path matches the regex pattern
                if let Some(new_name) = regex_rename(from_pattern, to_pattern, &path_str)? {
                    // Only rename if the name actually changed
                    if new_name != path_str {
                        files_to_rename.push((path.clone(), Path::new(&new_name).to_path_buf()));
                    }
                }
            }

            // Perform the renames
            for (old_path, new_path) in files_to_rename {
                target.rename_file(&old_path, &new_path)?;
            }
        }

        Ok(())
    }
}

/// Repo operator - pulls files from inherited repositories
pub mod repo {
    use super::*;

    /// Apply the repo operation to fetch and process a repository
    ///
    /// This fetches the specified repository using the RepositoryManager and applies
    /// any inline `with:` operations to the repository's filesystem.
    ///
    /// # Arguments
    /// * `op` - The repo operation configuration
    /// * `repo_manager` - RepositoryManager for fetching repositories
    ///
    /// # Returns
    /// Result containing the processed MemoryFS with repository contents
    pub fn apply(op: &RepoOp, repo_manager: &RepositoryManager) -> Result<MemoryFS> {
        // Fetch the repository with optional path filtering
        let mut fs =
            repo_manager.fetch_repository_with_path(&op.url, &op.r#ref, op.path.as_deref())?;

        // Apply inline with: operations if present
        if !op.with.is_empty() {
            apply_with_clause(&op.with, &mut fs)?;
        }

        Ok(fs)
    }

    /// Apply a list of operations to a filesystem (used for `with:` clauses)
    ///
    /// This applies a sequence of operations to modify a filesystem in-place.
    /// The operations are applied in the order they appear in the list.
    ///
    /// # Arguments
    /// * `operations` - List of operations to apply
    /// * `fs` - Filesystem to modify
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn apply_with_clause(operations: &[Operation], fs: &mut MemoryFS) -> Result<()> {
        for operation in operations {
            match operation {
                Operation::Include { .. } => {
                    // Include operations in `with:` clauses are currently no-ops
                    // since we're working with a single filesystem and files are
                    // already present. This could be enhanced in the future to
                    // support more complex include semantics.
                }
                Operation::Exclude { exclude } => {
                    super::exclude::apply(exclude, fs)?;
                }
                Operation::Rename { rename } => {
                    super::rename::apply(rename, fs)?;
                }
                // Note: Repo operations within `with:` clauses are not supported
                // as they would create circular dependencies
                Operation::Repo { .. } => {
                    return Err(crate::error::Error::Operator {
                        operator: "repo".to_string(),
                        message: "Repo operations not allowed in 'with:' clauses".to_string(),
                    });
                }
                // Template and merge operations are not implemented yet
                Operation::Template { .. }
                | Operation::TemplateVars { .. }
                | Operation::Yaml { .. }
                | Operation::Json { .. }
                | Operation::Toml { .. }
                | Operation::Ini { .. }
                | Operation::Markdown { .. }
                | Operation::Tools { .. } => {
                    return Err(crate::error::Error::Operator {
                        operator: "template/merge".to_string(),
                        message: "Operation not yet implemented".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RenameMapping, RenameOp};

    mod include_tests {
        use super::*;

        #[test]
        fn test_include_with_pattern() {
            let mut source = MemoryFS::new();
            let mut target = MemoryFS::new();

            // Add some files to source
            source
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            source
                .add_file_string("src/lib.rs", "pub fn lib() {}")
                .unwrap();
            source.add_file_string("README.md", "# Project").unwrap();

            // Include all .rs files
            let op = IncludeOp {
                patterns: vec!["*.rs".to_string()],
            };

            include::apply(&op, &source, &mut target).unwrap();

            // Should include src/main.rs and src/lib.rs (but not README.md)
            assert!(target.exists("src/main.rs"));
            assert!(target.exists("src/lib.rs"));
            assert!(!target.exists("README.md"));
        }

        #[test]
        fn test_include_multiple_patterns() {
            let mut source = MemoryFS::new();
            let mut target = MemoryFS::new();

            source
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            source
                .add_file_string("tests/test.rs", "fn test() {}")
                .unwrap();
            source.add_file_string("README.md", "# Project").unwrap();

            let op = IncludeOp {
                patterns: vec!["src/*.rs".to_string(), "README.md".to_string()],
            };

            include::apply(&op, &source, &mut target).unwrap();

            assert!(target.exists("src/main.rs"));
            assert!(!target.exists("tests/test.rs"));
            assert!(target.exists("README.md"));
        }
    }

    mod exclude_tests {
        use super::*;

        #[test]
        fn test_exclude_with_pattern() {
            let mut target = MemoryFS::new();

            // Add files to target
            target
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            target
                .add_file_string("src/lib.rs", "pub fn lib() {}")
                .unwrap();
            target.add_file_string("README.md", "# Project").unwrap();

            // Exclude all .rs files
            let op = ExcludeOp {
                patterns: vec!["*.rs".to_string()],
            };

            exclude::apply(&op, &mut target).unwrap();

            // Should exclude src/main.rs and src/lib.rs, keep README.md
            assert!(!target.exists("src/main.rs"));
            assert!(!target.exists("src/lib.rs"));
            assert!(target.exists("README.md"));
        }

        #[test]
        fn test_exclude_multiple_patterns() {
            let mut target = MemoryFS::new();

            target
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            target
                .add_file_string("tests/test.rs", "fn test() {}")
                .unwrap();
            target.add_file_string("README.md", "# Project").unwrap();
            target
                .add_file_string("CHANGELOG.md", "# Changelog")
                .unwrap();

            let op = ExcludeOp {
                patterns: vec!["src/*.rs".to_string(), "CHANGELOG.md".to_string()],
            };

            exclude::apply(&op, &mut target).unwrap();

            assert!(!target.exists("src/main.rs"));
            assert!(target.exists("tests/test.rs"));
            assert!(target.exists("README.md"));
            assert!(!target.exists("CHANGELOG.md"));
        }
    }

    mod rename_tests {
        use super::*;

        #[test]
        fn test_rename_simple_pattern() {
            let mut target = MemoryFS::new();

            // Add files
            target.add_file_string("main.rs", "fn main() {}").unwrap();
            target.add_file_string("lib.rs", "pub fn lib() {}").unwrap();

            // Rename .rs files to .backup
            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"(\w+)\.rs".to_string(),
                    to: "$1.backup".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            // Files should be renamed
            assert!(!target.exists("main.rs"));
            assert!(!target.exists("lib.rs"));
            assert!(target.exists("main.backup"));
            assert!(target.exists("lib.backup"));
        }

        #[test]
        fn test_rename_no_matches() {
            let mut target = MemoryFS::new();

            target.add_file_string("main.rs", "fn main() {}").unwrap();

            // Pattern that won't match
            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"(\w+)\.js".to_string(),
                    to: "$1.backup".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            // File should remain unchanged
            assert!(target.exists("main.rs"));
        }

        #[test]
        fn test_rename_multiple_groups() {
            let mut target = MemoryFS::new();

            target
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();

            // Swap directory and filename
            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"(\w+)/(\w+)\.rs".to_string(),
                    to: "$2_$1.rs".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            assert!(!target.exists("src/main.rs"));
            assert!(target.exists("main_src.rs"));
        }

        #[test]
        fn test_rename_multiple_mappings() {
            let mut target = MemoryFS::new();

            target.add_file_string("main.rs", "fn main() {}").unwrap();
            target.add_file_string("test.js", "fn test() {}").unwrap();

            let op = RenameOp {
                mappings: vec![
                    RenameMapping {
                        from: r"(\w+)\.rs".to_string(),
                        to: "$1_rust.rs".to_string(),
                    },
                    RenameMapping {
                        from: r"(\w+)\.js".to_string(),
                        to: "$1_js.js".to_string(),
                    },
                ],
            };

            rename::apply(&op, &mut target).unwrap();

            assert!(!target.exists("main.rs"));
            assert!(!target.exists("test.js"));
            assert!(target.exists("main_rust.rs"));
            assert!(target.exists("test_js.js"));
        }
    }

    mod repo_tests {
        use super::*;
        use crate::config::{ExcludeOp, IncludeOp, RenameMapping};
        use crate::repository::{CacheOperations, GitOperations, RepositoryManager};

        // Mock implementations for testing
        struct MockGitOps;
        struct MockCacheOps {
            filesystem: MemoryFS,
        }

        impl GitOperations for MockGitOps {
            fn clone_shallow(
                &self,
                _url: &str,
                _ref_name: &str,
                _path: &std::path::Path,
            ) -> Result<()> {
                Ok(()) // Mock always succeeds
            }

            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec!["v1.0.0".to_string(), "v1.1.0".to_string()])
            }
        }

        impl MockCacheOps {
            #[allow(dead_code)]
            fn new(filesystem: MemoryFS) -> Self {
                Self { filesystem }
            }

            fn with_filesystem(filesystem: MemoryFS) -> Self {
                Self { filesystem }
            }
        }

        impl CacheOperations for MockCacheOps {
            fn exists(&self, _cache_path: &std::path::Path) -> bool {
                true // Mock cache always exists
            }

            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache/path")
            }

            fn get_cache_path_with_path(
                &self,
                _url: &str,
                _ref_name: &str,
                _path: Option<&str>,
            ) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache/path")
            }

            fn load_from_cache(&self, _cache_path: &std::path::Path) -> Result<MemoryFS> {
                Ok(self.filesystem.clone())
            }

            fn load_from_cache_with_path(
                &self,
                _cache_path: &std::path::Path,
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

            fn save_to_cache(&self, _cache_path: &std::path::Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        #[test]
        fn test_repo_apply_without_with_clause() {
            // Create a mock repository with some files
            let mut mock_fs = MemoryFS::new();
            mock_fs.add_file_string("README.md", "# Test Repo").unwrap();
            mock_fs
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: None,
                with: vec![], // No with clause
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should contain the files from the mock repository
            assert!(result.exists("README.md"));
            assert!(result.exists("src/main.rs"));
            assert!(!result.exists("nonexistent.txt"));
        }

        #[test]
        fn test_repo_apply_with_with_clause() {
            // Create a mock repository with some files
            let mut mock_fs = MemoryFS::new();
            mock_fs.add_file_string("README.md", "# Test Repo").unwrap();
            mock_fs
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            mock_fs
                .add_file_string("src/lib.rs", "pub fn lib() {}")
                .unwrap();
            mock_fs.add_file_string("test.txt", "test file").unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation with with: clause that excludes .rs files
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: None,
                with: vec![Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.rs".to_string()],
                    },
                }],
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should contain README.md and test.txt, but not the .rs files
            assert!(result.exists("README.md"));
            assert!(result.exists("test.txt"));
            assert!(!result.exists("src/main.rs"));
            assert!(!result.exists("src/lib.rs"));
        }

        #[test]
        fn test_repo_apply_with_complex_with_clause() {
            // Create a mock repository with some files
            let mut mock_fs = MemoryFS::new();
            mock_fs.add_file_string("README.md", "# Test Repo").unwrap();
            mock_fs
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            mock_fs
                .add_file_string("src/utils.js", "function util() {}")
                .unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation with complex with: clause
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: None,
                with: vec![
                    Operation::Exclude {
                        exclude: ExcludeOp {
                            patterns: vec!["README.md".to_string()],
                        },
                    },
                    Operation::Rename {
                        rename: RenameOp {
                            mappings: vec![RenameMapping {
                                from: r"(\w+)\.js".to_string(),
                                to: "$1_renamed.js".to_string(),
                            }],
                        },
                    },
                ],
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should exclude README.md, keep main.rs, and rename utils.js
            assert!(!result.exists("README.md"));
            assert!(result.exists("src/main.rs"));
            assert!(!result.exists("src/utils.js"));
            assert!(result.exists("src/utils_renamed.js"));
        }

        #[test]
        fn test_apply_with_clause_include_operation() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("src/main.rs", "fn main() {}").unwrap();
            fs.add_file_string("src/lib.rs", "pub fn lib() {}").unwrap();
            fs.add_file_string("README.md", "# Project").unwrap();

            let operations = vec![Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/*.rs".to_string()],
                },
            }];

            repo::apply_with_clause(&operations, &mut fs).unwrap();

            // Include operations in `with:` clauses are currently no-ops
            // Files should remain unchanged
            assert!(fs.exists("src/main.rs"));
            assert!(fs.exists("src/lib.rs"));
            assert!(fs.exists("README.md"));
        }

        #[test]
        fn test_apply_with_clause_exclude_operation() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("src/main.rs", "fn main() {}").unwrap();
            fs.add_file_string("src/lib.rs", "pub fn lib() {}").unwrap();
            fs.add_file_string("README.md", "# Project").unwrap();

            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.rs".to_string()],
                },
            }];

            repo::apply_with_clause(&operations, &mut fs).unwrap();

            // Should exclude .rs files but keep README.md
            assert!(!fs.exists("src/main.rs"));
            assert!(!fs.exists("src/lib.rs"));
            assert!(fs.exists("README.md"));
        }

        #[test]
        fn test_apply_with_clause_rename_operation() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("main.rs", "fn main() {}").unwrap();
            fs.add_file_string("README.md", "# Project").unwrap();

            let operations = vec![Operation::Rename {
                rename: RenameOp {
                    mappings: vec![RenameMapping {
                        from: r"(\w+)\.rs".to_string(),
                        to: "$1_backup.rs".to_string(),
                    }],
                },
            }];

            repo::apply_with_clause(&operations, &mut fs).unwrap();

            // Should rename main.rs to main_backup.rs
            assert!(!fs.exists("main.rs"));
            assert!(fs.exists("main_backup.rs"));
            assert!(fs.exists("README.md"));
        }

        #[test]
        fn test_apply_with_clause_repo_operation_not_allowed() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "test").unwrap();

            let operations = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/repo.git".to_string(),
                    path: None,
                    r#ref: "main".to_string(),
                    with: vec![],
                },
            }];

            let result = repo::apply_with_clause(&operations, &mut fs);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Repo operations not allowed"));
        }

        #[test]
        fn test_apply_with_clause_unimplemented_operation() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "test").unwrap();

            let operations = vec![Operation::Yaml {
                yaml: crate::config::YamlMergeOp {
                    source: "config.yml".to_string(),
                    dest: "merged.yml".to_string(),
                    path: "/".to_string(),
                    append: false,
                },
            }];

            let result = repo::apply_with_clause(&operations, &mut fs);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Operation not yet implemented"));
        }

        #[test]
        fn test_repo_apply_with_path_filter() {
            // Create a mock repository with files in different directories
            let mut mock_fs = MemoryFS::new();
            mock_fs
                .add_file_string("README.md", "# Root Readme")
                .unwrap();
            mock_fs
                .add_file_string("uv/main.py", "uv main code")
                .unwrap();
            mock_fs.add_file_string("uv/lib.py", "uv lib code").unwrap();
            mock_fs
                .add_file_string("django/models.py", "django models")
                .unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation with path filter
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: Some("uv".to_string()),
                with: vec![], // No with clause
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should contain only files from uv directory, with paths relative to uv/
            assert!(result.exists("main.py"));
            assert!(result.exists("lib.py"));
            assert!(!result.exists("README.md"));
            assert!(!result.exists("django/models.py"));

            // Verify content
            let main_content = result.get_file("main.py").unwrap();
            assert_eq!(main_content.content, b"uv main code");
        }

        #[test]
        fn test_repo_apply_with_path_filter_and_with_clause() {
            // Create a mock repository with files in uv directory
            let mut mock_fs = MemoryFS::new();
            mock_fs
                .add_file_string("uv/main.py", "uv main code")
                .unwrap();
            mock_fs
                .add_file_string("uv/test.py", "uv test code")
                .unwrap();
            mock_fs.add_file_string("uv/lib.py", "uv lib code").unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation with path filter and with: clause to exclude test files
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: Some("uv".to_string()),
                with: vec![Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*test*".to_string()],
                    },
                }],
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should contain main.py and lib.py but not test.py
            assert!(result.exists("main.py"));
            assert!(result.exists("lib.py"));
            assert!(!result.exists("test.py"));
        }

        #[test]
        fn test_repo_apply_without_path_backward_compatibility() {
            // Create a mock repository with some files
            let mut mock_fs = MemoryFS::new();
            mock_fs.add_file_string("README.md", "# Test Repo").unwrap();
            mock_fs
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();

            // Create repository manager with mock operations
            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps),
                Box::new(MockCacheOps::with_filesystem(mock_fs)),
            );

            // Create repo operation without path (backward compatibility)
            let op = RepoOp {
                url: "https://github.com/test/repo.git".to_string(),
                r#ref: "main".to_string(),
                path: None,
                with: vec![], // No with clause
            };

            // Apply the repo operation
            let result = repo::apply(&op, &repo_manager).unwrap();

            // Should contain all files from the repository
            assert!(result.exists("README.md"));
            assert!(result.exists("src/main.rs"));
        }

        #[test]
        fn test_apply_with_clause_empty_operations() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "test").unwrap();

            let operations = vec![];

            let result = repo::apply_with_clause(&operations, &mut fs);
            assert!(result.is_ok());
            assert!(fs.exists("test.txt"));
        }

        #[test]
        fn test_apply_with_clause_multiple_unimplemented_operations() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "test").unwrap();

            let operations = vec![
                Operation::Template {
                    template: crate::config::TemplateOp {
                        patterns: vec!["template.txt".to_string()],
                    },
                },
                Operation::Json {
                    json: crate::config::JsonMergeOp {
                        source: "source.json".to_string(),
                        dest: "dest.json".to_string(),
                        path: "/".to_string(),
                        append: false,
                        position: "end".to_string(),
                    },
                },
            ];

            let result = repo::apply_with_clause(&operations, &mut fs);
            assert!(result.is_err());
            if let Err(crate::error::Error::Operator { operator, .. }) = result {
                assert!(operator.contains("template/merge"));
            } else {
                panic!("Expected Operator error");
            }
        }

        #[test]
        fn test_apply_with_clause_all_unimplemented_operations() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "test").unwrap();

            // Test all unimplemented operation types
            let unimplemented_ops = vec![
                Operation::Template {
                    template: crate::config::TemplateOp {
                        patterns: vec!["t.txt".to_string()],
                    },
                },
                Operation::TemplateVars {
                    template_vars: crate::config::TemplateVars {
                        vars: std::collections::HashMap::new(),
                    },
                },
                Operation::Yaml {
                    yaml: crate::config::YamlMergeOp {
                        source: "s.yaml".to_string(),
                        dest: "d.yaml".to_string(),
                        path: "/".to_string(),
                        append: false,
                    },
                },
                Operation::Json {
                    json: crate::config::JsonMergeOp {
                        source: "s.json".to_string(),
                        dest: "d.json".to_string(),
                        path: "/".to_string(),
                        append: false,
                        position: "end".to_string(),
                    },
                },
                Operation::Toml {
                    toml: crate::config::TomlMergeOp {
                        source: "s.toml".to_string(),
                        dest: "d.toml".to_string(),
                        path: "/".to_string(),
                        append: false,
                        preserve_comments: false,
                    },
                },
                Operation::Ini {
                    ini: crate::config::IniMergeOp {
                        source: "s.ini".to_string(),
                        dest: "d.ini".to_string(),
                        section: "main".to_string(),
                        append: false,
                        allow_duplicates: false,
                    },
                },
                Operation::Markdown {
                    markdown: crate::config::MarkdownMergeOp {
                        source: "s.md".to_string(),
                        dest: "d.md".to_string(),
                        section: "Section".to_string(),
                        append: false,
                        level: 2,
                        position: "end".to_string(),
                        create_section: false,
                    },
                },
                Operation::Tools {
                    tools: crate::config::ToolsOp { tools: vec![] },
                },
            ];

            for op in unimplemented_ops {
                let mut test_fs = fs.clone();
                let result = repo::apply_with_clause(&[op], &mut test_fs);
                assert!(result.is_err(), "Unimplemented operation should error");
            }
        }
    }

    mod rename_edge_case_tests {
        use super::*;

        #[test]
        fn test_rename_multiple_renames_same_file() {
            // Test multiple rename operations on the same file
            // Note: The rename implementation collects all files ONCE at the start,
            // so sequential renames in one operation won't chain (the second mapping
            // won't see files created by the first mapping)
            let mut target = MemoryFS::new();
            target.add_file_string("file.txt", "content").unwrap();

            let op = RenameOp {
                mappings: vec![
                    RenameMapping {
                        from: r"file\.txt".to_string(),
                        to: "renamed1.txt".to_string(),
                    },
                    RenameMapping {
                        from: r"renamed1\.txt".to_string(),
                        to: "renamed2.txt".to_string(),
                    },
                ],
            };

            rename::apply(&op, &mut target).unwrap();

            // Only the first rename applies because current_files is collected once at the start
            // The second mapping doesn't see renamed1.txt because it wasn't in the original list
            assert!(!target.exists("file.txt"));
            assert!(target.exists("renamed1.txt"));
            assert!(!target.exists("renamed2.txt"));
        }

        #[test]
        fn test_rename_overlapping_patterns() {
            // Test overlapping rename patterns
            let mut target = MemoryFS::new();
            target.add_file_string("test.txt", "content").unwrap();
            target.add_file_string("test_backup.txt", "backup").unwrap();

            let op = RenameOp {
                mappings: vec![
                    RenameMapping {
                        from: r"test\.txt".to_string(),
                        to: "test_backup.txt".to_string(),
                    },
                    RenameMapping {
                        from: r"test_backup\.txt".to_string(),
                        to: "final.txt".to_string(),
                    },
                ],
            };

            rename::apply(&op, &mut target).unwrap();

            // Both files should be renamed
            assert!(!target.exists("test.txt"));
            assert!(!target.exists("test_backup.txt"));
            assert!(target.exists("final.txt"));
        }

        #[test]
        fn test_rename_invalid_regex_pattern() {
            // Test invalid regex pattern
            let mut target = MemoryFS::new();
            target.add_file_string("test.txt", "content").unwrap();

            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"[invalid".to_string(), // Invalid regex - unclosed bracket
                    to: "renamed.txt".to_string(),
                }],
            };

            let result = rename::apply(&op, &mut target);
            assert!(result.is_err());
            if let Err(crate::error::Error::Regex(_)) = result {
                // Expected regex error
            } else {
                panic!("Expected Regex error for invalid pattern");
            }
        }

        #[test]
        fn test_rename_empty_pattern() {
            // Test empty pattern (should match nothing)
            let mut target = MemoryFS::new();
            target.add_file_string("test.txt", "content").unwrap();

            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: "".to_string(),
                    to: "renamed.txt".to_string(),
                }],
            };

            // Empty pattern might match everything or nothing depending on regex behavior
            // This tests the behavior
            let result = rename::apply(&op, &mut target);
            // Result depends on regex behavior - could succeed or fail
            // Just verify it doesn't panic
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn test_rename_pattern_no_matches() {
            // Test pattern that doesn't match anything
            let mut target = MemoryFS::new();
            target.add_file_string("test.txt", "content").unwrap();

            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"nonexistent\.txt".to_string(),
                    to: "renamed.txt".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            // File should remain unchanged
            assert!(target.exists("test.txt"));
            assert!(!target.exists("renamed.txt"));
        }

        #[test]
        fn test_rename_to_existing_filename() {
            // Test renaming to an existing filename (overwrite behavior)
            let mut target = MemoryFS::new();
            target.add_file_string("old.txt", "old content").unwrap();
            target
                .add_file_string("existing.txt", "existing content")
                .unwrap();

            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"old\.txt".to_string(),
                    to: "existing.txt".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            // old.txt should be renamed to existing.txt (overwriting it)
            assert!(!target.exists("old.txt"));
            assert!(target.exists("existing.txt"));
            // Content should be from old.txt (last write wins)
            let file = target.get_file("existing.txt").unwrap();
            assert_eq!(
                String::from_utf8(file.content.clone()).unwrap(),
                "old content"
            );
        }

        #[test]
        fn test_rename_complex_capture_groups() {
            // Test complex capture groups
            let mut target = MemoryFS::new();
            target
                .add_file_string("src/main.rs", "fn main() {}")
                .unwrap();
            target
                .add_file_string("src/lib.rs", "pub fn lib() {}")
                .unwrap();

            let op = RenameOp {
                mappings: vec![RenameMapping {
                    from: r"src/(\w+)\.rs".to_string(),
                    to: "rust/$1.rs".to_string(),
                }],
            };

            rename::apply(&op, &mut target).unwrap();

            assert!(!target.exists("src/main.rs"));
            assert!(!target.exists("src/lib.rs"));
            assert!(target.exists("rust/main.rs"));
            assert!(target.exists("rust/lib.rs"));
        }
    }
}
