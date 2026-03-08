//! # Phase 5: Local File Merging
//!
//! This is the fifth phase of the `common-repo` execution pipeline. Its purpose
//! is to merge the composite filesystem (created in Phase 4) with the files
//! from the local project directory. It also handles any operations that are
//! defined in the root `.common-repo.yaml` that are intended to be applied
//! locally.
//!
//! ## Process
//!
//! 1.  **Load Local Files**: The process begins by loading all the files from the
//!     current working directory into a new `MemoryFS`. Certain files, like
//!     those in the `.git` directory or the `.common-repo.yaml` file itself,
//!     are automatically ignored.
//!
//! 2.  **Apply Local Operations**: Any operations defined in the root configuration
//!     that are intended for local application (such as `template` and `merge`
//!     operations) are applied to the local `MemoryFS`. This allows for local
//!     files to be processed as templates or merged with other files.
//!
//! 3.  **Merge with Composite**: The processed local filesystem is then merged
//!     on top of the composite filesystem. This ensures that local files always
//!     take precedence, overwriting any inherited files with the same name.
//!
//! This phase produces the final, fully merged `MemoryFS`, which is an exact
//! representation of what the output directory should look like.

use std::collections::HashMap;
use std::path::Path;

use crate::config::{Operation, Schema};
use crate::defaults::{ALT_CONFIG_FILENAME, DEFAULT_CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Executes Phase 5 of the pipeline.
///
/// This function orchestrates the merging of the composite filesystem with
/// the local files. It loads the local files, applies any local operations
/// to them, and then merges the result into the composite filesystem.
pub fn execute(
    composite_fs: &MemoryFS,
    local_config: &Schema,
    working_dir: &Path,
) -> Result<MemoryFS> {
    // Start with a copy of the composite filesystem
    let mut final_fs = composite_fs.clone();

    // Load local files and apply local operations to them
    let mut local_fs = load_local_fs(working_dir)?;
    apply_local_operations_to_local_fs(&mut local_fs, local_config)?;

    // Merge local files into final filesystem
    merge_local_files(&mut final_fs, &local_fs)?;

    // Apply any merge operations defined in the local configuration to the final filesystem
    apply_local_operations(&mut final_fs, local_config)?;

    Ok(final_fs)
}

/// Load local files from the working directory into a MemoryFS
///
/// Recursively walks the directory and loads all files, preserving relative paths.
/// Skips common build/artifact directories and hidden files to avoid loading
/// unnecessary data into memory.
fn load_local_fs(working_dir: &Path) -> Result<MemoryFS> {
    let mut local_fs = MemoryFS::new();

    // Common directories to skip (build artifacts, dependencies, caches, etc.)
    const SKIP_DIRS: &[&str] = &[
        "target",        // Rust build artifacts
        "node_modules",  // Node.js dependencies
        ".git",          // Git repository data
        ".svn",          // SVN repository data
        ".hg",           // Mercurial repository data
        "build",         // Generic build output
        "dist",          // Distribution files
        "__pycache__",   // Python bytecode cache
        ".pytest_cache", // Pytest cache
        ".mypy_cache",   // MyPy cache
        ".tox",          // Tox environments
        "venv",          // Python virtual environment
        ".venv",         // Python virtual environment
        "env",           // Generic environment
        ".env",          // Environment files
        ".idea",         // IntelliJ IDEA
        ".vscode",       // VS Code
        ".vs",           // Visual Studio
        "bin",           // Binary output
        "obj",           // Object files
    ];

    // Use walkdir to recursively find all files, filtering directories early
    for entry in walkdir::WalkDir::new(working_dir)
        .into_iter()
        .filter_entry(|e| {
            // Always allow the root directory (depth 0) to be processed
            if e.depth() == 0 {
                return true;
            }

            // Get the file/directory name
            let file_name = e.file_name().to_str().unwrap_or("");

            // Skip if it's one of the common build directories
            if SKIP_DIRS.contains(&file_name) {
                return false;
            }

            // Skip hidden files/directories (starting with .)
            if file_name.starts_with('.') {
                return false;
            }

            // Allow everything else
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();

        // Get relative path from working directory
        let relative_path = file_path
            .strip_prefix(working_dir)
            .map_err(|_| Error::Path {
                message: format!("Failed to make path relative: {}", file_path.display()),
            })?;

        // Skip .common-repo.yaml config file
        if relative_path
            .to_str()
            .map(|s| s == DEFAULT_CONFIG_FILENAME || s == ALT_CONFIG_FILENAME)
            .unwrap_or(false)
        {
            continue;
        }

        // Read file content
        let content = std::fs::read(file_path)?;

        // Add to filesystem with relative path
        local_fs.add_file(relative_path, File::new(content))?;
    }

    Ok(local_fs)
}

/// Merge local files into the final filesystem
fn merge_local_files(final_fs: &mut MemoryFS, local_fs: &MemoryFS) -> Result<()> {
    for (path, file) in local_fs.files() {
        // Local files override inherited files
        final_fs.add_file(path, file.clone())?;
    }
    Ok(())
}

/// Apply operations to the local filesystem before merging
///
/// Applies template operations (marking and variable collection) to local files,
/// then processes templates with collected variables, and finally applies
fn apply_local_operations_to_local_fs(
    local_fs: &mut MemoryFS,
    local_config: &Schema,
) -> Result<()> {
    // Collect template variables from local config
    let mut local_template_vars = HashMap::new();
    for operation in local_config {
        if let Operation::TemplateVars { template_vars } = operation {
            crate::operators::template_vars::collect(template_vars, &mut local_template_vars)?;
        }
    }

    // Apply template marking operations to local files
    for operation in local_config {
        if let Operation::Template { template } = operation {
            crate::operators::template::mark(template, local_fs)?;
        }
    }

    // Process templates in local files
    crate::operators::template::process(local_fs, &local_template_vars)?;

    // Note: Merge operations (yaml, json, toml, ini, markdown) are NOT applied here.
    // They are applied later in apply_local_operations() after local files are
    // merged into the final filesystem. Applying them twice would cause duplicate
    // merges (e.g., arrays would be appended twice).

    Ok(())
}

/// Apply local operations from the configuration
///
/// These are operations that apply to the final merged filesystem.
/// Filtering operations (exclude, include, rename) are applied first to
/// ensure the file set is correct before merge operations run.
fn apply_local_operations(final_fs: &mut MemoryFS, local_config: &Schema) -> Result<()> {
    use crate::operators;

    for operation in local_config {
        match operation {
            // Filtering operations: applied to final_fs to control which files
            // appear in the output. This fixes #226 where consumer-level
            // exclude/include/rename operations were silently ignored.
            Operation::Exclude { exclude } => {
                operators::exclude::apply(exclude, final_fs)?;
            }
            Operation::Include { include } => {
                let mut filtered_fs = MemoryFS::new();
                operators::include::apply(include, final_fs, &mut filtered_fs)?;
                *final_fs = filtered_fs;
            }
            Operation::Rename { rename } => {
                operators::rename::apply(rename, final_fs)?;
            }
            // Merge operations: combine local and inherited content
            Operation::Yaml { yaml } => {
                crate::merge::yaml::apply_yaml_merge_operation(final_fs, yaml)?;
            }
            Operation::Json { json } => {
                crate::merge::json::apply_json_merge_operation(final_fs, json)?;
            }
            Operation::Toml { toml } => {
                crate::merge::toml::apply_toml_merge_operation(final_fs, toml)?;
            }
            Operation::Ini { ini } => {
                crate::merge::ini::apply_ini_merge_operation(final_fs, ini)?;
            }
            Operation::Markdown { markdown } => {
                crate::merge::markdown::apply_markdown_merge_operation(final_fs, markdown)?;
            }
            // Template and template_vars are handled in apply_local_operations_to_local_fs.
            // Repo operations are resolved in Phase 1. Tools are validated separately.
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExcludeOp, IncludeOp, RenameMapping, RenameOp};
    use tempfile::TempDir;

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

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

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

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

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

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

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

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

        assert_eq!(final_fs.len(), 1);
        assert!(final_fs.exists("local.txt"));
    }

    #[test]
    fn test_phase5_consumer_exclude_removes_files() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Create local files
        std::fs::write(working_dir.join("keep.txt"), b"keep").unwrap();
        std::fs::write(working_dir.join("remove.txt"), b"remove").unwrap();

        // Composite has inherited files
        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("inherited.txt", "inherited")
            .unwrap();

        // Consumer config excludes "remove.txt"
        let local_config = vec![Operation::Exclude {
            exclude: ExcludeOp {
                patterns: vec!["remove.txt".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

        assert!(final_fs.exists("keep.txt"));
        assert!(final_fs.exists("inherited.txt"));
        assert!(!final_fs.exists("remove.txt"));
    }

    #[test]
    fn test_phase5_consumer_exclude_with_glob_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::create_dir_all(working_dir.join("cmd/app")).unwrap();
        std::fs::write(working_dir.join("main.go"), b"package main").unwrap();
        std::fs::write(working_dir.join("cmd/app/run.go"), b"package app").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("inherited.txt", "data")
            .unwrap();

        // Exclude cmd/** glob
        let local_config = vec![Operation::Exclude {
            exclude: ExcludeOp {
                patterns: vec!["cmd/**".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

        assert!(final_fs.exists("main.go"));
        assert!(final_fs.exists("inherited.txt"));
        assert!(!final_fs.exists("cmd/app/run.go"));
    }

    #[test]
    fn test_phase5_consumer_include_filters_to_matching() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("wanted.txt"), b"yes").unwrap();
        std::fs::write(working_dir.join("unwanted.rs"), b"no").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("inherited.txt", "data")
            .unwrap();

        // Include only *.txt files
        let local_config = vec![Operation::Include {
            include: IncludeOp {
                patterns: vec!["*.txt".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

        assert!(final_fs.exists("wanted.txt"));
        assert!(final_fs.exists("inherited.txt"));
        assert!(!final_fs.exists("unwanted.rs"));
    }

    #[test]
    fn test_phase5_consumer_rename_renames_files() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("old_name.txt", "content")
            .unwrap();

        let local_config = vec![Operation::Rename {
            rename: RenameOp {
                mappings: vec![RenameMapping {
                    from: r"old_name\.txt".to_string(),
                    to: "new_name.txt".to_string(),
                }],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir).unwrap();

        assert!(!final_fs.exists("old_name.txt"));
        assert!(final_fs.exists("new_name.txt"));
    }
}
