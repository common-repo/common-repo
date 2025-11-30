//! Implementation of the 6 phases of the common-repo pull operation.
//!
//! ## Overview
//!
//! The pull operation follows 6 phases:
//! 1. Discovery and Cloning - Fetch all inherited repos in parallel (with automatic caching)
//! 2. Processing Individual Repos - Apply operations to each repo
//! 3. Determining Operation Order - Calculate deterministic merge order
//! 4. Composite Filesystem Construction - Merge all intermediate filesystems
//! 5. Local File Merging - Merge with local files
//! 6. Writing to Disk - Write final result to host filesystem
//!
//! Note: Caching happens automatically during Phase 1 via RepositoryManager, so there is no
//! separate cache update phase.
//!
//! Each phase depends only on the previous phases and the foundation layers (0-2).

use std::collections::{HashMap, HashSet};

use crate::config::{Operation, Schema};
use crate::error::Result;
use crate::filesystem::MemoryFS;

// Phase modules
pub mod composite;
pub mod discovery;
pub mod local_merge;
pub mod ordering;
pub mod processing;
pub mod write;

// Re-export phase modules to preserve public API
pub use composite as phase4;
pub use discovery as phase1;
pub use local_merge as phase5;
pub use ordering as phase3;
pub use processing as phase2;
pub use write as phase6;

/// Repository tree node representing inheritance hierarchy
#[derive(Debug, Clone)]
pub struct RepoNode {
    /// Repository URL
    pub url: String,
    /// Git reference (tag, branch, commit)
    pub ref_: String,
    /// Child repositories that inherit from this one
    pub children: Vec<RepoNode>,
    /// Operations to apply to this repository
    pub operations: Vec<Operation>,
}

impl RepoNode {
    pub fn new(url: String, ref_: String, operations: Vec<Operation>) -> Self {
        Self {
            url,
            ref_,
            children: Vec::new(),
            operations,
        }
    }

    pub fn add_child(&mut self, child: RepoNode) {
        self.children.push(child);
    }
}

/// Repository dependency tree for inheritance tracking
#[derive(Debug)]
pub struct RepoTree {
    /// Root repository (the one being pulled)
    pub root: RepoNode,
    /// All repositories in the tree (for cycle detection)
    pub all_repos: HashSet<(String, String)>,
}

impl RepoTree {
    pub fn new(root: RepoNode) -> Self {
        let mut all_repos = HashSet::new();
        Self::collect_repos(&root, &mut all_repos);
        Self { root, all_repos }
    }

    fn collect_repos(node: &RepoNode, repos: &mut HashSet<(String, String)>) {
        // Only add real repositories, not the synthetic "local" root
        if node.url != "local" {
            repos.insert((node.url.clone(), node.ref_.clone()));
        }
        for child in &node.children {
            Self::collect_repos(child, repos);
        }
    }

    /// Check if a repo exists anywhere in the tree
    ///
    /// Note: This is a simple existence check. For proper cycle detection during
    /// tree construction, use `detect_cycles()` which checks for cycles in dependency paths.
    /// Multiple branches can reference the same repo without creating a cycle.
    #[allow(dead_code)]
    pub fn would_create_cycle(&self, url: &str, ref_: &str) -> bool {
        self.all_repos
            .contains(&(url.to_string(), ref_.to_string()))
    }
}

// Placeholder modules for remaining phases
/// Intermediate filesystem wrapper with metadata
#[derive(Debug, Clone)]
pub struct IntermediateFS {
    /// The processed filesystem
    pub fs: MemoryFS,
    /// Repository URL this FS came from (for debugging/tracking)
    pub source_url: String,
    /// Git reference used
    pub source_ref: String,
    /// Template variables collected from this repository's operations
    pub template_vars: HashMap<String, String>,
    /// Merge operations to be applied during Phase 4 composition
    pub merge_operations: Vec<Operation>,
}

impl IntermediateFS {
    pub fn new(fs: MemoryFS, source_url: String, source_ref: String) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
            template_vars: HashMap::new(),
            merge_operations: Vec::new(),
        }
    }

    pub fn new_with_vars(
        fs: MemoryFS,
        source_url: String,
        source_ref: String,
        template_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
            template_vars,
            merge_operations: Vec::new(),
        }
    }

    pub fn new_with_vars_and_merges(
        fs: MemoryFS,
        source_url: String,
        source_ref: String,
        template_vars: HashMap<String, String>,
        merge_operations: Vec<Operation>,
    ) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
            template_vars,
            merge_operations,
        }
    }
}

/// Operation order for deterministic merging
#[derive(Debug, Clone)]
pub struct OperationOrder {
    /// Ordered list of repository keys in the correct merge order
    /// Format: "url@ref" (e.g., "https://github.com/user/repo@main")
    pub order: Vec<String>,
}

impl OperationOrder {
    pub fn new(order: Vec<String>) -> Self {
        Self { order }
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }
}

/// Orchestrator for the complete pull operation
///
/// This module coordinates all phases to provide a clean API for the complete
/// pull operation. Currently implements Phases 1-5 for end-to-end inheritance.
pub mod orchestrator {
    use super::*;
    use crate::cache::RepoCache;
    use crate::repository::RepositoryManager;
    use std::path::Path;

    /// Execute the complete pull operation (Phases 1-6)
    ///
    /// This orchestrates the complete inheritance pipeline:
    /// 1. Discover and clone repositories (with automatic caching)
    /// 2. Process each repository with its operations
    /// 3. Determine correct merge order
    /// 4. Merge into composite filesystem
    /// 5. Merge with local files and apply local operations
    /// 6. Write final filesystem to disk (if output_path is provided)
    ///
    /// If `output_path` is `None`, returns the final MemoryFS without writing to disk.
    /// If `output_path` is `Some(path)`, writes to disk and returns the MemoryFS.
    pub fn execute_pull(
        config: &Schema,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
        working_dir: &Path,
        output_path: Option<&Path>,
    ) -> Result<MemoryFS> {
        // Phase 1: Discovery and Cloning
        let repo_tree = phase1::execute(config, repo_manager, cache)?;

        // Phase 2: Processing Individual Repos
        let intermediate_fss = phase2::execute(&repo_tree, repo_manager, cache)?;

        // Phase 3: Determining Operation Order
        let operation_order = phase3::execute(&repo_tree)?;

        // Phase 4: Composite Filesystem Construction
        let composite_fs = phase4::execute(&operation_order, &intermediate_fss)?;

        // Phase 5: Local File Merging
        let final_fs = phase5::execute(&composite_fs, config, working_dir)?;

        // Phase 6: Write to Disk (if output path provided)
        if let Some(output) = output_path {
            phase6::execute(&final_fs, output)?;
        }

        Ok(final_fs)
    }
}

#[cfg(test)]
mod phase_tests {
    use super::*;
    use crate::repository::{CacheOperations, GitOperations};
    use std::path::Path;
    use tempfile::TempDir;

    mod phase5_tests {
        use super::*;
        use crate::merge::markdown::apply_markdown_merge_operation;
        use crate::merge::toml::{apply_toml_merge_operation, parse_toml_path};
        use crate::merge::PathSegment;

        // Note: MockGitOps and MockCacheOps are defined but not used in phase5 tests
        // They're kept for potential future use
        #[allow(dead_code)]
        struct MockGitOps;
        #[allow(dead_code)]
        struct MockCacheOps;

        #[allow(dead_code)]
        impl GitOperations for MockGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                Ok(())
            }

            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        #[allow(dead_code)]
        impl CacheOperations for MockCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                false
            }

            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache")
            }

            fn get_cache_path_with_path(
                &self,
                _url: &str,
                _ref_name: &str,
                _path: Option<&str>,
            ) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache")
            }

            fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
                Ok(MemoryFS::new())
            }

            fn load_from_cache_with_path(
                &self,
                _cache_path: &Path,
                _path: Option<&str>,
            ) -> Result<MemoryFS> {
                Ok(MemoryFS::new())
            }

            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        #[test]
        fn test_phase5_execute_merge_local_files() {
            // Test merging composite filesystem with local files
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            // Create local files
            std::fs::create_dir_all(working_dir.join("subdir")).unwrap();
            std::fs::write(working_dir.join("local.txt"), b"local content").unwrap();
            std::fs::write(working_dir.join("subdir/nested.txt"), b"nested content").unwrap();

            // Create composite filesystem
            let mut composite_fs = MemoryFS::new();
            composite_fs
                .add_file_string("composite.txt", "composite content")
                .unwrap();

            // Create local config (empty for this test)
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Should contain both composite and local files
            assert!(final_fs.exists("composite.txt"));
            assert!(final_fs.exists("local.txt"));
            assert!(final_fs.exists("subdir/nested.txt"));
        }

        #[test]
        fn test_phase5_execute_local_files_override_composite() {
            // Test that local files override composite files (last-write-wins)
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            // Create local file with same name as composite
            std::fs::write(working_dir.join("common.txt"), b"local version").unwrap();

            // Create composite filesystem with same file
            let mut composite_fs = MemoryFS::new();
            composite_fs
                .add_file_string("common.txt", "composite version")
                .unwrap();

            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Local file should override composite
            let file = final_fs.get_file("common.txt").unwrap();
            assert_eq!(
                String::from_utf8(file.content.clone()).unwrap(),
                "local version"
            );
        }

        #[test]
        fn test_phase5_execute_skips_hidden_files() {
            // Test that hidden files and .git directory are skipped
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            std::fs::write(working_dir.join(".hidden"), b"hidden").unwrap();
            std::fs::write(working_dir.join(".common-repo.yaml"), b"config").unwrap();
            std::fs::create_dir_all(working_dir.join(".git")).unwrap();
            std::fs::write(working_dir.join(".git/config"), b"git config").unwrap();
            std::fs::write(working_dir.join("visible.txt"), b"visible").unwrap();

            let composite_fs = MemoryFS::new();
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Should only contain visible.txt
            assert!(final_fs.exists("visible.txt"));
            assert!(!final_fs.exists(".hidden"));
            assert!(!final_fs.exists(".common-repo.yaml"));
            assert!(!final_fs.exists(".git/config"));
        }

        #[test]
        fn test_phase5_execute_empty_composite() {
            // Test with empty composite filesystem
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            std::fs::write(working_dir.join("local.txt"), b"local").unwrap();

            let composite_fs = MemoryFS::new();
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            assert_eq!(final_fs.len(), 1);
            assert!(final_fs.exists("local.txt"));
        }

        #[test]
        fn test_parse_toml_path_empty() {
            assert_eq!(parse_toml_path("").len(), 0);
            assert_eq!(parse_toml_path("  ").len(), 0);
        }

        #[test]
        fn test_parse_toml_path_simple_keys() {
            let segments = parse_toml_path("package.dependencies");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "package"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "dependencies"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_toml_path_array_index() {
            let segments = parse_toml_path("workspace.members[0]");
            assert_eq!(segments.len(), 3);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "workspace"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "members"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[2] {
                PathSegment::Index(idx) => assert_eq!(*idx, 0),
                _ => panic!("Expected Index segment"),
            }
        }

        #[test]
        fn test_parse_toml_path_quoted_keys() {
            let segments = parse_toml_path(r#"package["version"]"#);
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "package"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "version"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_toml_path_escaped_quotes() {
            // Test escaped quotes within quoted keys
            let segments = parse_toml_path(r#"config["key\"with\"quotes"]"#);
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "config"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, r#"key"with"quotes"#),
                _ => panic!("Expected Key segment"),
            }

            // Test escaped backslash
            let segments = parse_toml_path(r#"data["path\\with\\backslashes"]"#);
            assert_eq!(segments.len(), 2);
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, r"path\with\backslashes"),
                _ => panic!("Expected Key segment"),
            }

            // Test single quotes with escaped single quotes
            let segments = parse_toml_path(r"config['key\'with\'quotes']");
            assert_eq!(segments.len(), 2);
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "key'with'quotes"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_toml_path_complex() {
            let segments = parse_toml_path(r#"config.database[0].settings"#);
            assert_eq!(segments.len(), 4);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "config"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "database"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[2] {
                PathSegment::Index(idx) => assert_eq!(*idx, 0),
                _ => panic!("Expected Index segment"),
            }
            match &segments[3] {
                PathSegment::Key(k) => assert_eq!(k, "settings"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_toml_merge_operation_root_level() {
            // Test TOML merge at root level
            let mut fs = MemoryFS::new();

            // Create source TOML fragment
            let source_toml = r#"
[package]
name = "test-package"
version = "1.0.0"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            // Create destination TOML file
            let dest_toml = r#"
[dependencies]
serde = "1.0"
"#;
            fs.add_file_string("Cargo.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "Cargo.toml".to_string(),
                path: "".to_string(), // root level
                append: false,
                preserve_comments: false,
                array_mode: None,
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("Cargo.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain both original and merged content
            assert!(result_str.contains("serde = \"1.0\""));
            assert!(result_str.contains("name = \"test-package\""));
            assert!(result_str.contains("version = \"1.0.0\""));
        }

        #[test]
        fn test_toml_merge_operation_nested_path() {
            // Test TOML merge at nested path
            let mut fs = MemoryFS::new();

            // Create source TOML fragment
            let source_toml = r#"
enabled = true
timeout = 30
"#;
            fs.add_file_string("config.toml", source_toml).unwrap();

            // Create destination TOML file
            let dest_toml = r#"
[server]
host = "localhost"

[database]
name = "mydb"
"#;
            fs.add_file_string("merged.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "config.toml".to_string(),
                dest: "merged.toml".to_string(),
                path: "server".to_string(),
                append: false,
                preserve_comments: false,
                array_mode: None,
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("merged.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should have server section with new fields
            assert!(result_str.contains("[server]"));
            assert!(result_str.contains("host = \"localhost\""));
            assert!(result_str.contains("enabled = true"));
            assert!(result_str.contains("timeout = 30"));
            // Should still have database section
            assert!(result_str.contains("[database]"));
            assert!(result_str.contains("name = \"mydb\""));
        }

        #[test]
        fn test_toml_merge_array_mode_replace() {
            let mut fs = MemoryFS::new();
            let source_toml = r#"
[package]
items = ["new1", "new2"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = ["old1", "old2"]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "dest.toml".to_string(),
                path: "".to_string(),
                append: false,
                preserve_comments: false,
                array_mode: Some(crate::config::ArrayMergeMode::Replace),
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("dest.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();
            let value: toml::Value = result_str.parse().unwrap();
            let items = value["package"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("new1"));
            assert_eq!(items[1].as_str(), Some("new2"));
        }

        #[test]
        fn test_toml_merge_array_mode_append() {
            let mut fs = MemoryFS::new();
            let source_toml = r#"
[package]
items = ["new1", "new2"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = ["old1", "old2"]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "dest.toml".to_string(),
                path: "".to_string(),
                append: false,
                preserve_comments: false,
                array_mode: Some(crate::config::ArrayMergeMode::Append),
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("dest.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();
            let value: toml::Value = result_str.parse().unwrap();
            let items = value["package"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 4);
            assert_eq!(items[0].as_str(), Some("old1"));
            assert_eq!(items[1].as_str(), Some("old2"));
            assert_eq!(items[2].as_str(), Some("new1"));
            assert_eq!(items[3].as_str(), Some("new2"));
        }

        #[test]
        fn test_toml_merge_array_mode_append_unique() {
            let mut fs = MemoryFS::new();
            let source_toml = r#"
[package]
items = ["item1", "item2", "item3"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = ["item1", "item4"]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "dest.toml".to_string(),
                path: "".to_string(),
                append: false,
                preserve_comments: false,
                array_mode: Some(crate::config::ArrayMergeMode::AppendUnique),
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("dest.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();
            let value: toml::Value = result_str.parse().unwrap();
            let items = value["package"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 4);
            assert_eq!(items[0].as_str(), Some("item1"));
            assert_eq!(items[1].as_str(), Some("item4"));
            assert_eq!(items[2].as_str(), Some("item2"));
            assert_eq!(items[3].as_str(), Some("item3"));
        }

        #[test]
        fn test_toml_merge_backward_compatibility_append_bool() {
            let mut fs = MemoryFS::new();
            let source_toml = r#"
[package]
items = ["new1"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = ["old1"]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "dest.toml".to_string(),
                path: "".to_string(),
                append: true,
                preserve_comments: false,
                array_mode: None,
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("dest.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();
            let value: toml::Value = result_str.parse().unwrap();
            let items = value["package"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("old1"));
            assert_eq!(items[1].as_str(), Some("new1"));
        }

        // Note: INI merge tests have been moved to src/merge/ini.rs

        #[test]
        fn test_markdown_merge_operation_basic() {
            // Test Markdown merge with section
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"## Installation

Run the following command:

```
npm install my-package
```

## Usage

Basic usage example here.
"#;
            fs.add_file_string("install.md", source_md).unwrap();

            // Create destination markdown file
            let dest_md = r#"# My Package

This is a great package.

## Features

- Feature 1
- Feature 2
"#;
            fs.add_file_string("README.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "install.md".to_string(),
                dest: "README.md".to_string(),
                section: "Installation".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true,
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("README.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain both original content and merged section
            assert!(result_str.contains("# My Package"));
            assert!(result_str.contains("## Features"));
            assert!(result_str.contains("## Installation"));
            assert!(result_str.contains("npm install my-package"));
            assert!(result_str.contains("## Usage"));
        }

        #[test]
        fn test_markdown_merge_operation_create_section() {
            // Test Markdown merge creating a new section
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"This package provides excellent functionality."#;
            fs.add_file_string("description.md", source_md).unwrap();

            // Create destination markdown file without the target section
            let dest_md = r#"# My Package

## Installation

Install instructions here.
"#;
            fs.add_file_string("README.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "description.md".to_string(),
                dest: "README.md".to_string(),
                section: "Description".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true, // create section if it doesn't exist
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("README.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain the new section
            assert!(result_str.contains("## Description"));
            assert!(result_str.contains("This package provides excellent functionality"));
        }

        #[test]
        fn test_markdown_merge_operation_append_mode() {
            // Test Markdown merge in append mode
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"- New feature added
- Bug fixes included"#;
            fs.add_file_string("updates.md", source_md).unwrap();

            // Create destination markdown file with existing section
            let dest_md = r#"# Changelog

## Version 1.0.0

- Initial release
- Basic functionality
"#;
            fs.add_file_string("CHANGELOG.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "updates.md".to_string(),
                dest: "CHANGELOG.md".to_string(),
                section: "Version 1.0.0".to_string(),
                append: true, // append mode
                level: 2,
                position: "end".to_string(),
                create_section: false,
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("CHANGELOG.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain original content plus appended content
            assert!(result_str.contains("## Version 1.0.0"));
            assert!(result_str.contains("- Initial release"));
            assert!(result_str.contains("- Basic functionality"));
            assert!(result_str.contains("- New feature added"));
            assert!(result_str.contains("- Bug fixes included"));
        }
    }

    mod repo_tree_tests {
        use super::*;

        #[test]
        fn test_repo_tree_creation() {
            let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 0); // local is not counted
        }

        #[test]
        fn test_repo_tree_with_children() {
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let child = RepoNode::new(
                "https://github.com/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 1);
            assert!(tree.all_repos.contains(&(
                "https://github.com/repo.git".to_string(),
                "main".to_string()
            )));
        }

        #[test]
        fn test_repo_node_add_child() {
            let mut parent = RepoNode::new(
                "https://github.com/parent.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let child = RepoNode::new(
                "https://github.com/child.git".to_string(),
                "main".to_string(),
                vec![],
            );

            assert_eq!(parent.children.len(), 0);
            parent.add_child(child);
            assert_eq!(parent.children.len(), 1);
            assert_eq!(parent.children[0].url, "https://github.com/child.git");
        }

        #[test]
        fn test_repo_tree_collects_all_repos() {
            // Test that RepoTree collects all repos from nested structure
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let mut parent = RepoNode::new(
                "https://github.com/parent.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let child = RepoNode::new(
                "https://github.com/child.git".to_string(),
                "main".to_string(),
                vec![],
            );
            parent.add_child(child);
            root.add_child(parent);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 2);
            assert!(tree.all_repos.contains(&(
                "https://github.com/parent.git".to_string(),
                "main".to_string()
            )));
            assert!(tree.all_repos.contains(&(
                "https://github.com/child.git".to_string(),
                "main".to_string()
            )));
        }

        #[test]
        fn test_repo_tree_would_create_cycle() {
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let repo = RepoNode::new(
                "https://github.com/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(repo);
            let tree = RepoTree::new(root);

            // Check if repo exists
            assert!(tree.would_create_cycle("https://github.com/repo.git", "main"));
            assert!(!tree.would_create_cycle("https://github.com/other.git", "main"));
        }
    }

    mod parse_path_tests {
        use crate::phases::phase5::{parse_path, PathSegment};

        #[test]
        fn test_parse_path_empty() {
            assert_eq!(parse_path("").len(), 0);
            assert_eq!(parse_path("  ").len(), 0);
            assert_eq!(parse_path("/").len(), 0);
        }

        #[test]
        fn test_parse_path_simple_dots() {
            let segments = parse_path("metadata.labels");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_numeric_index() {
            let segments = parse_path("items[0]");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
        }

        #[test]
        fn test_parse_path_quoted_bracket_double() {
            let segments = parse_path(r#"metadata["labels.with.dot"]"#);
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels.with.dot"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_quoted_bracket_single() {
            let segments = parse_path("metadata['labels.with.dot']");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels.with.dot"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_backslash_escape() {
            let segments = parse_path(r"metadata\.labels");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata.labels"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_complex_mixed() {
            let segments = parse_path(r#"items[0].metadata["labels.app"].value"#);
            assert_eq!(segments.len(), 5);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
            match &segments[2] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[3] {
                PathSegment::Key(k) => assert_eq!(k, "labels.app"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[4] {
                PathSegment::Key(k) => assert_eq!(k, "value"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_unquoted_bracket_key() {
            let segments = parse_path("[key]");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "key"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_escaped_quote_in_bracket() {
            let segments = parse_path(r#"["key\"with\"quotes"]"#);
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, r#"key"with"quotes"#),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_multiple_indices() {
            let segments = parse_path("items[0][1]");
            assert_eq!(segments.len(), 3);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
            match &segments[2] {
                PathSegment::Index(i) => assert_eq!(*i, 1),
                _ => panic!("Expected Index segment"),
            }
        }
    }
}
