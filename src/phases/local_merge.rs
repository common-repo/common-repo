//! # Phase 5: Local File Merging
//!
//! This is the fifth phase of the `common-repo` execution pipeline. Its purpose
//! is to combine the composite filesystem (created in Phase 4) with the files
//! from the local project directory, execute deferred merge operations, and
//! apply consumer-level operations in declaration order.
//!
//! ## Process
//!
//! 1.  **Load Local Files**: Load all files from the working directory into a
//!     new `MemoryFS`. Specific build/tool directories and config files are
//!     skipped. Dotfiles are loaded, since common-repo manages them.
//!
//! 2.  **Apply Local Template Operations**: Template marking and variable
//!     substitution are applied to local files.
//!
//! 3.  **Combine with Composite**: The composite filesystem is overlaid on top
//!     of local files. Composite files win for shared paths (ensuring upstream
//!     updates propagate). Local-only files are preserved.
//!
//! 4.  **Execute Deferred Merges**: Merge operations collected during Phase 4
//!     are executed against the combined filesystem. This allows fragments to
//!     merge into the actual destination file, whether it came from another
//!     upstream or from the consumer's working directory.
//!
//! 5.  **Apply Consumer Operations**: All consumer-level operations (merges and
//!     filters) execute in YAML declaration order. Each operation transforms the
//!     filesystem as it exists at that point. This means:
//!     - A merge after an exclude won't find files the exclude removed
//!     - An include before a merge filters the FS before the merge runs
//!     - Operations interleave naturally: include, merge, exclude, rename, etc.
//!
//! This phase produces the final, fully merged `MemoryFS`, which is an exact
//! representation of what the output directory should look like.

use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::config::{Operation, Schema};
use crate::defaults::{ALT_CONFIG_FILENAME, DEFAULT_CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Executes Phase 5 of the pipeline.
///
/// Combines local files with the composite filesystem (composite wins for
/// shared paths), executes deferred merge operations, then runs all consumer
/// operations in YAML declaration order.
pub fn execute(
    composite_fs: &MemoryFS,
    local_config: &Schema,
    working_dir: &Path,
    deferred_ops: &[Operation],
) -> Result<MemoryFS> {
    // Load local files and apply local template operations
    let mut local_fs = load_local_fs(working_dir)?;
    apply_local_operations_to_local_fs(&mut local_fs, local_config)?;

    // Combine: start with local, overlay composite on top (composite wins)
    let mut final_fs = local_fs;
    merge_composite_over_local(&mut final_fs, composite_fs)?;

    // Execute deferred merge operations against the combined filesystem
    for op in deferred_ops {
        super::composite::execute_merge_operation(&mut final_fs, op)?;
    }

    // Apply consumer operations in declaration order (YAML order = execution order)
    apply_consumer_operations(&mut final_fs, local_config)?;

    Ok(final_fs)
}

/// Load local files from the working directory into a MemoryFS
///
/// Recursively walks the directory and loads all files, preserving relative paths.
/// Skips common build/artifact directories to avoid loading unnecessary data
/// into memory. Dotfiles are loaded since common-repo manages them
/// (e.g. `.editorconfig`, `.pre-commit-config.yaml`).
fn load_local_fs(working_dir: &Path) -> Result<MemoryFS> {
    let mut local_fs = MemoryFS::new();

    // VCS metadata, build artifacts, dependency caches, and IDE configs.
    //
    // This is a hardcoded list rather than a blanket "skip all dotfiles"
    // approach because common-repo needs to load dotfiles it manages
    // (.editorconfig, .pre-commit-config.yaml, etc.).
    //
    // The list goes beyond the spec's explicit examples (.git, .svn, .hg,
    // node_modules, target, build, dist, __pycache__) to also cover IDE
    // directories and language-specific caches.
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
        ".env",          // Environment dir (virtualenv) or file (dotenv) — both skipped
        ".idea",         // IntelliJ IDEA
        ".vscode",       // VS Code
        ".vs",           // Visual Studio
        ".cache",        // Generic cache directory
        ".npm",          // npm cache
        ".yarn",         // Yarn cache/releases
        ".next",         // Next.js build cache
        ".nuxt",         // Nuxt.js build output
        ".turbo",        // Turborepo cache
        ".nx",           // Nx build cache
        ".angular",      // Angular cache
        ".gradle",       // Gradle cache
        ".parcel-cache", // Parcel bundler cache
        ".svelte-kit",   // SvelteKit build
        "bin",           // Binary output (broader than spec's explicit list)
        "obj",           // Object files (broader than spec's explicit list)
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

            // Skip if it's one of the common build/tool directories
            if SKIP_DIRS.contains(&file_name) {
                return false;
            }

            // Skip platform-specific metadata files
            if file_name == ".DS_Store"
                || file_name == "Thumbs.db"
                || file_name == "Desktop.ini"
                || file_name == ".directory"
                || file_name.starts_with("._")
            {
                return false;
            }

            // Allow everything else (including dotfiles — common-repo
            // manages dotfiles like .editorconfig, .pre-commit-config.yaml)
            true
        })
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                warn!("Skipping inaccessible path during local FS load: {}", err);
                None
            }
        })
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

/// Merge composite files over local files (composite wins for shared paths)
fn merge_composite_over_local(final_fs: &mut MemoryFS, composite_fs: &MemoryFS) -> Result<()> {
    for (path, file) in composite_fs.files() {
        final_fs.add_file(path, file.clone())?;
    }
    Ok(())
}

/// Apply operations to the local filesystem before merging
///
/// Applies template operations (marking and variable collection) to local files,
/// then processes templates with collected variables. Merge and filter operations
/// are skipped here; they run later in `apply_consumer_operations` against the
/// combined filesystem.
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

    // Note: Merge operations (yaml, json, toml, ini, markdown, xml) are NOT applied here.
    // They run later in apply_consumer_operations() against the combined filesystem
    // (local + composite). Applying them here AND there would cause duplicate merges
    // (e.g., arrays would be appended twice).

    Ok(())
}

/// Apply consumer operations from the local configuration in declaration order
///
/// All operations -- merges (yaml, json, toml, ini, markdown, xml) and filters
/// (include, exclude, rename) -- execute sequentially in the order they appear
/// in the config. YAML order = execution order.
fn apply_consumer_operations(final_fs: &mut MemoryFS, local_config: &Schema) -> Result<()> {
    use crate::operators;

    for operation in local_config {
        match operation {
            // Filter operations
            Operation::Exclude { exclude } => {
                operators::exclude::apply(exclude, final_fs)?;
            }
            Operation::Include { include } => {
                // include::apply copies matching files into a new FS rather than
                // removing non-matching files, so we replace the entire FS with
                // the filtered result.
                let mut filtered_fs = MemoryFS::new();
                operators::include::apply(include, final_fs, &mut filtered_fs)?;
                *final_fs = filtered_fs;
            }
            Operation::Rename { rename } => {
                operators::rename::apply(rename, final_fs)?;
            }
            // Merge operations — delegate to the shared dispatcher
            Operation::Yaml { .. }
            | Operation::Json { .. }
            | Operation::Toml { .. }
            | Operation::Ini { .. }
            | Operation::Markdown { .. }
            | Operation::Xml { .. } => {
                super::composite::execute_merge_operation(final_fs, operation)?;
            }
            // Repo operations are resolved in Phase 1; template and
            // template_vars are applied earlier in this phase via
            // apply_local_operations_to_local_fs; tools and self run
            // outside the consumer pipeline.
            Operation::Repo { .. } => {}
            Operation::Template { .. } => {}
            Operation::TemplateVars { .. } => {}
            Operation::Tools { .. } => {}
            Operation::Self_ { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExcludeOp, IncludeOp, JsonMergeOp, RenameMapping, RenameOp};
    use tempfile::TempDir;

    #[test]
    fn test_phase5_execute_merge_local_files() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::create_dir_all(working_dir.join("subdir")).unwrap();
        std::fs::write(working_dir.join("local.txt"), b"local content").unwrap();
        std::fs::write(working_dir.join("subdir/nested.txt"), b"nested content").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("composite.txt", "composite content")
            .unwrap();

        let local_config = vec![];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        assert!(final_fs.exists("composite.txt"));
        assert!(final_fs.exists("local.txt"));
        assert!(final_fs.exists("subdir/nested.txt"));
    }

    #[test]
    fn test_phase5_composite_wins_for_shared_paths() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("common.txt"), b"local version").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("common.txt", "composite version")
            .unwrap();

        let local_config = vec![];
        let deferred_ops = vec![];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        let file = final_fs.get_file("common.txt").unwrap();
        assert_eq!(
            String::from_utf8(file.content.clone()).unwrap(),
            "composite version"
        );
    }

    #[test]
    fn test_phase5_local_only_files_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("local_only.txt"), b"my local file").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("upstream.txt", "upstream content")
            .unwrap();

        let local_config = vec![];
        let deferred_ops = vec![];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        assert!(final_fs.exists("upstream.txt"));
        assert!(final_fs.exists("local_only.txt"));
        let local_file = final_fs.get_file("local_only.txt").unwrap();
        assert_eq!(
            String::from_utf8(local_file.content.clone()).unwrap(),
            "my local file"
        );
    }

    #[test]
    fn test_phase5_deferred_merges_execute_after_combination() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(
            working_dir.join("package.json"),
            r#"{"name": "my-app", "version": "1.0.0"}"#,
        )
        .unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("fragment.json", r#"{"scripts": {"test": "jest"}}"#)
            .unwrap();

        let local_config = vec![];

        let deferred_ops = vec![Operation::Json {
            json: JsonMergeOp {
                source: Some("fragment.json".to_string()),
                dest: Some("package.json".to_string()),
                ..Default::default()
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        let package_file = final_fs.get_file("package.json").unwrap();
        let content = String::from_utf8(package_file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(json["name"], "my-app");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["scripts"]["test"], "jest");
    }

    #[test]
    fn test_phase5_deferred_merge_into_shared_path() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(
            working_dir.join("ci-config.json"),
            r#"{"runner": "old-runner", "timeout": 30}"#,
        )
        .unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string(
                "ci-config.json",
                r#"{"runner": "new-runner", "timeout": 60}"#,
            )
            .unwrap();
        composite_fs
            .add_file_string("ci-fragment.json", r#"{"lint": true}"#)
            .unwrap();

        let local_config = vec![];

        let deferred_ops = vec![Operation::Json {
            json: JsonMergeOp {
                source: Some("ci-fragment.json".to_string()),
                dest: Some("ci-config.json".to_string()),
                ..Default::default()
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        let file = final_fs.get_file("ci-config.json").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Composite version wins (new-runner, 60)
        assert_eq!(json["runner"], "new-runner");
        assert_eq!(json["timeout"], 60);
        // Fragment merged in
        assert_eq!(json["lint"], true);
    }

    #[test]
    fn test_phase5_consumer_operations_run_in_declaration_order() {
        // Declaration order: merge first, THEN exclude.
        // Sequential model: merge succeeds (source exists), then exclude removes source.
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("output.json"), r#"{"base": true}"#).unwrap();
        std::fs::write(working_dir.join("fragment.json"), r#"{"added": true}"#).unwrap();

        let composite_fs = MemoryFS::new();

        // Merge runs first (in declaration order), exclude runs second.
        let local_config = vec![
            Operation::Json {
                json: JsonMergeOp {
                    source: Some("fragment.json".to_string()),
                    dest: Some("output.json".to_string()),
                    ..Default::default()
                },
            },
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["fragment.json".to_string()],
                },
            },
        ];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        let file = final_fs.get_file("output.json").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["base"], true);
        assert_eq!(json["added"], true);

        // Fragment excluded after merge
        assert!(!final_fs.exists("fragment.json"));
    }

    #[test]
    fn test_phase5_execute_skips_specific_dirs_and_config() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Dotfiles are loaded (common-repo manages these)
        std::fs::write(working_dir.join(".hidden"), b"hidden").unwrap();

        // Config files are skipped
        std::fs::write(working_dir.join(".common-repo.yaml"), b"config").unwrap();

        // SKIP_DIRS entries are skipped
        std::fs::create_dir_all(working_dir.join(".git")).unwrap();
        std::fs::write(working_dir.join(".git/config"), b"git config").unwrap();
        std::fs::create_dir_all(working_dir.join("node_modules/pkg")).unwrap();
        std::fs::write(working_dir.join("node_modules/pkg/index.js"), b"module").unwrap();

        std::fs::write(working_dir.join("visible.txt"), b"visible").unwrap();

        let composite_fs = MemoryFS::new();
        let local_config = vec![];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        assert!(final_fs.exists("visible.txt"));
        // Dotfiles are now loaded
        assert!(final_fs.exists(".hidden"));
        // Config files still skipped
        assert!(!final_fs.exists(".common-repo.yaml"));
        // SKIP_DIRS still skipped
        assert!(!final_fs.exists(".git/config"));
        assert!(!final_fs.exists("node_modules/pkg/index.js"));
    }

    #[test]
    fn test_phase5_load_local_fs_includes_dotfiles() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Dotfiles that common-repo manages — these must be loaded
        std::fs::write(working_dir.join(".pre-commit-config.yaml"), b"repos: []").unwrap();
        std::fs::write(working_dir.join(".editorconfig"), b"root = true").unwrap();
        std::fs::write(working_dir.join(".eslintrc.yaml"), b"rules: {}").unwrap();

        // Specific dirs that should still be skipped
        std::fs::create_dir_all(working_dir.join(".git")).unwrap();
        std::fs::write(working_dir.join(".git/config"), b"git config").unwrap();

        // Config files that should still be skipped
        std::fs::write(working_dir.join(".common-repo.yaml"), b"config").unwrap();

        // Regular file
        std::fs::write(working_dir.join("README.md"), b"readme").unwrap();

        let local_fs = load_local_fs(working_dir).unwrap();

        // Dotfiles are loaded
        assert!(local_fs.exists(".pre-commit-config.yaml"));
        assert!(local_fs.exists(".editorconfig"));
        assert!(local_fs.exists(".eslintrc.yaml"));

        // Regular files are loaded
        assert!(local_fs.exists("README.md"));

        // SKIP_DIRS entries are still skipped
        assert!(!local_fs.exists(".git/config"));

        // Config files are still skipped
        assert!(!local_fs.exists(".common-repo.yaml"));
    }

    #[test]
    fn test_phase5_load_local_fs_includes_dotfiles_in_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::create_dir_all(working_dir.join("subdir")).unwrap();
        std::fs::write(working_dir.join("subdir/.eslintrc.yaml"), b"rules: {}").unwrap();
        std::fs::write(working_dir.join("subdir/visible.txt"), b"visible").unwrap();

        let local_fs = load_local_fs(working_dir).unwrap();

        assert!(local_fs.exists("subdir/.eslintrc.yaml"));
        assert!(local_fs.exists("subdir/visible.txt"));
    }

    #[test]
    fn test_phase5_load_local_fs_includes_dot_prefixed_directory() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // A dot-prefixed directory NOT in SKIP_DIRS should have its contents loaded
        std::fs::create_dir_all(working_dir.join(".config")).unwrap();
        std::fs::write(working_dir.join(".config/settings.yaml"), b"key: value").unwrap();

        let local_fs = load_local_fs(working_dir).unwrap();

        assert!(local_fs.exists(".config/settings.yaml"));
    }

    #[test]
    fn test_phase5_load_local_fs_skips_platform_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join(".DS_Store"), b"metadata").unwrap();
        std::fs::write(working_dir.join("Thumbs.db"), b"metadata").unwrap();
        std::fs::write(working_dir.join("._resource"), b"metadata").unwrap();
        std::fs::write(working_dir.join("real.txt"), b"content").unwrap();

        let local_fs = load_local_fs(working_dir).unwrap();

        assert!(!local_fs.exists(".DS_Store"));
        assert!(!local_fs.exists("Thumbs.db"));
        assert!(!local_fs.exists("._resource"));
        assert!(local_fs.exists("real.txt"));
    }

    #[test]
    fn test_phase5_execute_empty_composite() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("local.txt"), b"local").unwrap();

        let composite_fs = MemoryFS::new();
        let local_config = vec![];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        assert_eq!(final_fs.len(), 1);
        assert!(final_fs.exists("local.txt"));
    }

    #[test]
    fn test_phase5_consumer_exclude_removes_files() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("keep.txt"), b"keep").unwrap();
        std::fs::write(working_dir.join("remove.txt"), b"remove").unwrap();

        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("inherited.txt", "inherited")
            .unwrap();

        let local_config = vec![Operation::Exclude {
            exclude: ExcludeOp {
                patterns: vec!["remove.txt".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

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

        let local_config = vec![Operation::Exclude {
            exclude: ExcludeOp {
                patterns: vec!["cmd/**".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

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

        let local_config = vec![Operation::Include {
            include: IncludeOp {
                patterns: vec!["*.txt".to_string()],
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

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

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        assert!(!final_fs.exists("old_name.txt"));
        assert!(final_fs.exists("new_name.txt"));
    }

    #[test]
    fn test_phase5_deferred_merge_ordering_multiple_upstreams() {
        use crate::config::{JsonMergeOp, Operation};

        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Consumer has the destination file locally
        std::fs::write(working_dir.join("config.json"), r#"{"base": true}"#).unwrap();

        // Composite has two fragments from two different upstreams
        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string(
                "fragment-a.json",
                r#"{"from_a": true, "shared": "a-value"}"#,
            )
            .unwrap();
        composite_fs
            .add_file_string(
                "fragment-b.json",
                r#"{"from_b": true, "shared": "b-value"}"#,
            )
            .unwrap();

        let local_config = vec![];

        // Deferred ops in order: repo A first, then repo B
        let deferred_ops = vec![
            Operation::Json {
                json: JsonMergeOp {
                    source: Some("fragment-a.json".to_string()),
                    dest: Some("config.json".to_string()),
                    ..Default::default()
                },
            },
            Operation::Json {
                json: JsonMergeOp {
                    source: Some("fragment-b.json".to_string()),
                    dest: Some("config.json".to_string()),
                    ..Default::default()
                },
            },
        ];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        let file = final_fs.get_file("config.json").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Base preserved
        assert_eq!(json["base"], true);
        // Both fragments merged
        assert_eq!(json["from_a"], true);
        assert_eq!(json["from_b"], true);
        // Repo B wins for shared key (applied second)
        assert_eq!(json["shared"], "b-value");
    }

    #[test]
    fn test_phase5_sequential_exclude_before_merge_removes_source() {
        // Sequential model: exclude runs FIRST, removing the merge source.
        // The subsequent merge should fail because its source no longer exists.
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::write(working_dir.join("output.json"), r#"{"base": true}"#).unwrap();
        std::fs::write(working_dir.join("fragment.json"), r#"{"added": true}"#).unwrap();

        let composite_fs = MemoryFS::new();

        let local_config = vec![
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["fragment.json".to_string()],
                },
            },
            Operation::Json {
                json: JsonMergeOp {
                    source: Some("fragment.json".to_string()),
                    dest: Some("output.json".to_string()),
                    ..Default::default()
                },
            },
        ];

        let result = execute(&composite_fs, &local_config, working_dir, &[]);

        // Sequential execution: exclude removed fragment.json, merge can't find source
        assert!(result.is_err());
    }

    #[test]
    fn test_phase5_deferred_merge_source_overwrites_conflicting_keys() {
        use crate::config::{JsonMergeOp, Operation};

        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Consumer has a config with existing keys
        std::fs::write(
            working_dir.join("settings.json"),
            r#"{"timeout": 30, "retries": 3, "local_only": true}"#,
        )
        .unwrap();

        // Upstream fragment overrides some keys
        let mut composite_fs = MemoryFS::new();
        composite_fs
            .add_file_string("settings-override.json", r#"{"timeout": 60, "retries": 5}"#)
            .unwrap();

        let local_config = vec![];

        let deferred_ops = vec![Operation::Json {
            json: JsonMergeOp {
                source: Some("settings-override.json".to_string()),
                dest: Some("settings.json".to_string()),
                ..Default::default()
            },
        }];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &deferred_ops).unwrap();

        let file = final_fs.get_file("settings.json").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Source wins for conflicting keys (SourceAlwaysWins invariant)
        assert_eq!(json["timeout"], 60);
        assert_eq!(json["retries"], 5);
        // Destination-only keys preserved
        assert_eq!(json["local_only"], true);
    }

    #[test]
    fn test_phase5_sequential_include_merge_exclude() {
        // Sequential model: include filters FS, merge runs against filtered FS,
        // exclude cleans up merge source.
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        std::fs::create_dir_all(working_dir.join("src")).unwrap();
        std::fs::write(working_dir.join("src/config.json"), r#"{"base": true}"#).unwrap();
        std::fs::write(working_dir.join("src/fragment.json"), r#"{"extra": true}"#).unwrap();
        std::fs::write(working_dir.join("root.txt"), b"root file").unwrap();

        let composite_fs = MemoryFS::new();

        let local_config = vec![
            // Step 1: Keep only src/**
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/**".to_string()],
                },
            },
            // Step 2: Merge fragment into config (both survived the include)
            Operation::Json {
                json: JsonMergeOp {
                    source: Some("src/fragment.json".to_string()),
                    dest: Some("src/config.json".to_string()),
                    ..Default::default()
                },
            },
            // Step 3: Clean up the fragment file
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["src/fragment.json".to_string()],
                },
            },
        ];

        let final_fs = execute(&composite_fs, &local_config, working_dir, &[]).unwrap();

        // root.txt gone (filtered by include)
        assert!(!final_fs.exists("root.txt"));
        // Merge happened: config.json has merged content
        let file = final_fs.get_file("src/config.json").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["base"], true);
        assert_eq!(json["extra"], true);
        // Fragment cleaned up
        assert!(!final_fs.exists("src/fragment.json"));
    }
}
