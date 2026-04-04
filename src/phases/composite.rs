//! Phase 4: Composite Filesystem Construction
//!
//! This is the fourth phase of the `common-repo` execution pipeline. Its
//! responsibility is to construct a single, composite filesystem from all the
//! intermediate filesystems produced by Phase 2, using the operation order
//! calculated in Phase 3.
//!
//! ## Process
//!
//! 1.  **Variable Consolidation**: The template variables from all
//!     `IntermediateFS` instances are collected into a single, unified set.
//!     If the same variable is defined in multiple repositories, the value
//!     from the repository that appears later in the `OperationOrder` takes
//!     precedence (i.e., a "last-write-wins" strategy). This is consistent
//!     with how file merging works.
//!
//! 2.  **Template Processing**: Once the variables are consolidated, each
//!     `IntermediateFS`'s underlying `MemoryFS` is processed for templates
//!     using the consolidated set of variables. This step substitutes all the
//!     `__COMMON_REPO__VAR__` placeholders with their final values.
//!
//! 3.  **Filesystem Merging**: After template processing, the `MemoryFS` from
//!     each `IntermediateFS` is merged into the composite filesystem. The merge
//!     is performed in the `OperationOrder`, which again ensures a "last-write-wins"
//!     behavior, where files from more specific repositories overwrite those
//!     from their ancestors.
//!
//! 4.  **Collect Deferred Merge Operations**: Merge operations from each
//!     repository are collected in operation order but not executed. They are
//!     returned alongside the composite filesystem for execution during Phase 5,
//!     after local files are available as merge destinations.
//!
//! This phase produces a `MemoryFS` and a `Vec<Operation>` of deferred merge
//! operations, ready for the final local merge in the next phase.

use std::collections::HashMap;

use super::{IntermediateFS, OperationOrder};
use crate::config::Operation;
use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

/// Executes Phase 4 of the pipeline.
///
/// This function orchestrates the construction of the composite filesystem
/// by first processing all templates with a unified set of variables and
/// then merging the resulting filesystems in the correct order.
pub fn execute(
    order: &OperationOrder,
    intermediate_fss: &HashMap<String, IntermediateFS>,
) -> Result<(MemoryFS, Vec<Operation>)> {
    // First, collect all template variables from all intermediate filesystems in operation order
    let mut all_template_vars = HashMap::new();
    for repo_key in &order.order {
        if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
            for (key, value) in &intermediate_fs.template_vars {
                // Later repositories override earlier ones (consistent with other operations)
                all_template_vars.insert(key.clone(), value.clone());
            }
        }
    }

    // Process templates in each intermediate filesystem
    let mut processed_fss = HashMap::new();
    for (repo_key, intermediate_fs) in intermediate_fss {
        let mut processed_fs = intermediate_fs.fs.clone();
        crate::operators::template::process(&mut processed_fs, &all_template_vars)?;
        processed_fss.insert(repo_key.clone(), processed_fs);
    }

    // Merge processed filesystems in the operation order
    // Later filesystems in the order take precedence (last-write-wins),
    // EXCEPT when the incoming repo declares an auto-merge for a file that
    // already exists in the composite — in that case, perform format-aware
    // merging to accumulate content from multiple upstreams.
    let mut composite_fs = MemoryFS::new();
    let mut deferred_ops = Vec::new();

    for repo_key in &order.order {
        if let Some(processed_fs) = processed_fss.get(repo_key) {
            // Get this repo's merge operations to check for auto-merge declarations
            let merge_ops = intermediate_fss
                .get(repo_key)
                .map(|ifs| &ifs.merge_operations)
                .cloned()
                .unwrap_or_default();

            // Build a map of auto-merge target paths -> operations for this repo
            let auto_merge_targets = collect_auto_merge_targets(&merge_ops);

            // Merge files individually, checking for auto-merge conflicts
            merge_filesystem_with_auto_merge(&mut composite_fs, processed_fs, &auto_merge_targets)?;

            // Collect merge operations for Phase 5 deferred execution
            if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
                deferred_ops.extend(intermediate_fs.merge_operations.clone());
            }
        } else {
            // This shouldn't happen if Phase 2 and Phase 3 are implemented correctly
            return Err(Error::Filesystem {
                message: format!(
                    "Missing intermediate filesystem for repository: {}",
                    repo_key
                ),
            });
        }
    }

    Ok((composite_fs, deferred_ops))
}

/// Merge a source filesystem into a target filesystem
///
/// All files from source_fs are copied to target_fs. If a file already exists
/// in target_fs, it is overwritten (last-write-wins strategy).
/// This preserves file metadata from the source filesystem.
#[cfg(test)]
fn merge_filesystem(target_fs: &mut MemoryFS, source_fs: &MemoryFS) -> Result<()> {
    for (path, file) in source_fs.files() {
        target_fs.add_file(path, file.clone())?;
    }
    Ok(())
}

/// Collect auto-merge target paths from a set of merge operations.
///
/// Returns a map from target file path to the corresponding Operation,
/// for operations that have `auto_merge` set.
fn collect_auto_merge_targets(ops: &[Operation]) -> HashMap<String, Operation> {
    let mut targets = HashMap::new();
    for op in ops {
        // Only collect non-explicitly-deferred auto-merge ops for Phase 4 inter-repo merging.
        // Ops with explicit `defer: true` are reserved for Phase 5 (consumer local file merge).
        if !is_explicitly_deferred(op) {
            if let Some(path) = get_auto_merge_path(op) {
                targets.insert(path.to_string(), op.clone());
            }
        }
    }
    targets
}

/// Returns true if the operation has `defer: true` set explicitly.
/// A plain `auto_merge` op without `defer: true` is an inter-repo merge (Phase 4).
fn is_explicitly_deferred(op: &Operation) -> bool {
    match op {
        Operation::Yaml { yaml } => yaml.defer.unwrap_or(false),
        Operation::Json { json } => json.defer.unwrap_or(false),
        Operation::Toml { toml } => toml.defer.unwrap_or(false),
        Operation::Ini { ini } => ini.defer.unwrap_or(false),
        Operation::Markdown { markdown } => markdown.defer.unwrap_or(false),
        Operation::Xml { xml } => xml.defer.unwrap_or(false),
        _ => false,
    }
}

/// Extract the auto-merge target path from an operation, if it has one.
fn get_auto_merge_path(op: &Operation) -> Option<&str> {
    match op {
        Operation::Yaml { yaml } => yaml.auto_merge.as_deref(),
        Operation::Json { json } => json.auto_merge.as_deref(),
        Operation::Toml { toml } => toml.auto_merge.as_deref(),
        Operation::Ini { ini } => ini.auto_merge.as_deref(),
        Operation::Markdown { markdown } => markdown.auto_merge.as_deref(),
        Operation::Xml { xml } => xml.auto_merge.as_deref(),
        _ => None,
    }
}

/// Create a synthetic merge operation with explicit source and dest paths,
/// preserving all other parameters from the original auto-merge operation.
fn make_explicit_merge_op(op: &Operation, source: &str, dest: &str) -> Operation {
    match op {
        Operation::Yaml { yaml } => Operation::Yaml {
            yaml: crate::config::YamlMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                path: yaml.path.clone(),
                // Inter-repo accumulation: upgrade default Replace to AppendUnique
                // so each upstream's array entries are preserved, not overwritten.
                array_mode: if yaml.array_mode == crate::config::ArrayMergeMode::Replace {
                    crate::config::ArrayMergeMode::AppendUnique
                } else {
                    yaml.array_mode
                },
                position: yaml.position,
                defer: None,
                auto_merge: None,
            },
        },
        Operation::Json { json } => Operation::Json {
            json: crate::config::JsonMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                path: json.path.clone(),
                array_mode: if json.array_mode == crate::config::ArrayMergeMode::Replace {
                    crate::config::ArrayMergeMode::AppendUnique
                } else {
                    json.array_mode
                },
                position: json.position,
                defer: None,
                auto_merge: None,
            },
        },
        Operation::Toml { toml } => Operation::Toml {
            toml: crate::config::TomlMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                path: toml.path.clone(),
                array_mode: if toml.array_mode == crate::config::ArrayMergeMode::Replace {
                    crate::config::ArrayMergeMode::AppendUnique
                } else {
                    toml.array_mode
                },
                position: toml.position,
                preserve_comments: toml.preserve_comments,
                defer: None,
                auto_merge: None,
            },
        },
        Operation::Ini { ini } => Operation::Ini {
            ini: crate::config::IniMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                section: ini.section.clone(),
                append: ini.append,
                allow_duplicates: ini.allow_duplicates,
                defer: None,
                auto_merge: None,
            },
        },
        Operation::Markdown { markdown } => Operation::Markdown {
            markdown: crate::config::MarkdownMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                section: markdown.section.clone(),
                append: markdown.append,
                level: markdown.level,
                position: markdown.position,
                create_section: markdown.create_section,
                defer: None,
                auto_merge: None,
            },
        },
        Operation::Xml { xml } => Operation::Xml {
            xml: crate::config::XmlMergeOp {
                source: Some(source.to_string()),
                dest: Some(dest.to_string()),
                path: xml.path.clone(),
                array_mode: xml.array_mode,
                position: xml.position,
                defer: None,
                auto_merge: None,
            },
        },
        // Should not be called with non-merge operations
        _ => op.clone(),
    }
}

/// Merge a source filesystem into a target filesystem with auto-merge awareness.
///
/// For each file in source_fs:
/// - If the file does NOT exist in target_fs: add it (normal behaviour)
/// - If the file exists AND has an auto-merge declaration: perform format-aware
///   merge to accumulate content from both repos
/// - If the file exists but NO auto-merge: overwrite (last-write-wins, normal behaviour)
fn merge_filesystem_with_auto_merge(
    target_fs: &mut MemoryFS,
    source_fs: &MemoryFS,
    auto_merge_targets: &HashMap<String, Operation>,
) -> Result<()> {
    for (path, file) in source_fs.files() {
        let path_str = path.to_string_lossy();
        if target_fs.exists(path) {
            if let Some(merge_op) = auto_merge_targets.get(path_str.as_ref()) {
                // Auto-merge conflict: merge instead of overwrite.
                // Stage the incoming file under a temp name, execute the merge,
                // then clean up.
                let temp_path = format!(".__common_repo_auto_merge_temp__{}", path_str);
                target_fs.add_file(&temp_path, file.clone())?;

                let explicit_op = make_explicit_merge_op(merge_op, &temp_path, &path_str);
                execute_merge_operation(target_fs, &explicit_op)?;

                // Clean up the temporary file
                target_fs.remove_file(&temp_path)?;
            } else {
                // No auto-merge declaration: last-write-wins
                target_fs.add_file(path, file.clone())?;
            }
        } else {
            // No conflict: just add the file
            target_fs.add_file(path, file.clone())?;
        }
    }
    Ok(())
}

/// Execute a single merge operation on the composite filesystem
///
/// This function dispatches to the appropriate merge operation handler
/// based on the operation type (YAML, JSON, TOML, INI, Markdown, or XML).
///
/// Made `pub(crate)` so Phase 5 can execute deferred merge operations
/// after local files are available.
pub(crate) fn execute_merge_operation(fs: &mut MemoryFS, operation: &Operation) -> Result<()> {
    match operation {
        Operation::Yaml { yaml } => crate::merge::yaml::apply_yaml_merge_operation(fs, yaml),
        Operation::Json { json } => crate::merge::json::apply_json_merge_operation(fs, json),
        Operation::Toml { toml } => crate::merge::toml::apply_toml_merge_operation(fs, toml),
        Operation::Ini { ini } => crate::merge::ini::apply_ini_merge_operation(fs, ini),
        Operation::Markdown { markdown } => {
            crate::merge::markdown::apply_markdown_merge_operation(fs, markdown)
        }
        Operation::Xml { xml } => crate::merge::xml::apply_xml_merge_operation(fs, xml),
        _ => {
            // Non-merge operations should not be passed to this function
            Err(Error::Filesystem {
                message: format!("Unexpected non-merge operation: {:?}", operation),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{execute, IntermediateFS, OperationOrder};
    use crate::error::Error;
    use crate::filesystem::MemoryFS;

    #[test]
    fn test_phase4_execute_merge_no_conflicts() {
        // Test merging two filesystems with no conflicts
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("file1.txt", "content1").unwrap();
        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("file2.txt", "content2").unwrap();

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-b.git@main".to_string(),
            IntermediateFS::new(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
            ),
        );

        let order = OperationOrder::new(vec![
            "https://github.com/repo-a.git@main".to_string(),
            "https://github.com/repo-b.git@main".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        assert_eq!(composite.len(), 2);
        assert!(composite.exists("file1.txt"));
        assert!(composite.exists("file2.txt"));
    }

    #[test]
    fn test_phase4_execute_merge_with_conflicts() {
        // Test merging filesystems with file conflicts (last-write-wins)
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("common.txt", "version1").unwrap();
        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("common.txt", "version2").unwrap();

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-b.git@main".to_string(),
            IntermediateFS::new(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
            ),
        );

        let order = OperationOrder::new(vec![
            "https://github.com/repo-a.git@main".to_string(),
            "https://github.com/repo-b.git@main".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Last filesystem should win
        assert_eq!(composite.len(), 1);
        let file = composite.get_file("common.txt").unwrap();
        assert_eq!(String::from_utf8(file.content.clone()).unwrap(), "version2");
    }

    #[test]
    fn test_phase4_execute_merge_multiple_filesystems() {
        // Test merging multiple filesystems in correct order
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("file1.txt", "content1").unwrap();
        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("file2.txt", "content2").unwrap();
        let mut fs3 = MemoryFS::new();
        fs3.add_file_string("file3.txt", "content3").unwrap();

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-b.git@main".to_string(),
            IntermediateFS::new(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-c.git@main".to_string(),
            IntermediateFS::new(
                fs3,
                "https://github.com/repo-c.git".to_string(),
                "main".to_string(),
            ),
        );

        let order = OperationOrder::new(vec![
            "https://github.com/repo-a.git@main".to_string(),
            "https://github.com/repo-b.git@main".to_string(),
            "https://github.com/repo-c.git@main".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        assert_eq!(composite.len(), 3);
        assert!(composite.exists("file1.txt"));
        assert!(composite.exists("file2.txt"));
        assert!(composite.exists("file3.txt"));
    }

    #[test]
    fn test_phase4_execute_missing_intermediate_fs() {
        // Test error when intermediate filesystem is missing
        let intermediate_fss = HashMap::new();
        let order = OperationOrder::new(vec!["https://github.com/repo-a.git@main".to_string()]);

        let result = execute(&order, &intermediate_fss);
        assert!(result.is_err());
        if let Err(Error::Filesystem { message: msg }) = result {
            assert!(msg.contains("Missing intermediate filesystem"));
        } else {
            panic!("Expected Filesystem error");
        }
    }

    #[test]
    fn test_phase4_execute_template_processing() {
        // Test that templates are processed with collected variables during Phase 4
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string(
            "template.txt",
            "Hello __COMMON_REPO__NAME__ from __COMMON_REPO__REPO__!",
        )
        .unwrap();

        // Mark template.txt as a template
        let template_op = crate::config::TemplateOp {
            patterns: vec!["*.txt".to_string()],
        };
        crate::operators::template::mark(&template_op, &mut fs1).unwrap();

        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("config.txt", "Config file").unwrap();

        // Create template variables for each repo
        let mut vars1 = HashMap::new();
        vars1.insert("NAME".to_string(), "Alice".to_string());
        vars1.insert("REPO".to_string(), "repo1".to_string());

        let mut vars2 = HashMap::new();
        vars2.insert("NAME".to_string(), "Bob".to_string()); // Should override
        vars2.insert("VERSION".to_string(), "1.0".to_string()); // Additional var

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new_with_vars(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                vars1,
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-b.git@main".to_string(),
            IntermediateFS::new_with_vars(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
                vars2,
            ),
        );

        let order = OperationOrder::new(vec![
            "https://github.com/repo-a.git@main".to_string(),
            "https://github.com/repo-b.git@main".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Template should be processed with merged variables (later repos override)
        assert!(composite.exists("template.txt"));
        let template_file = composite.get_file("template.txt").unwrap();
        let content = String::from_utf8(template_file.content.clone()).unwrap();
        assert_eq!(content, "Hello Bob from repo1!"); // NAME overridden, REPO from first repo
        assert!(!template_file.is_template); // Should be unmarked after processing

        // Non-template file should be unchanged
        assert!(composite.exists("config.txt"));
        let config_file = composite.get_file("config.txt").unwrap();
        let config_content = String::from_utf8(config_file.content.clone()).unwrap();
        assert_eq!(config_content, "Config file");
    }

    #[test]
    fn test_phase4_execute_template_processing_multiple_templates() {
        // Test processing multiple template files from different repositories
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("greeting.txt", "Hello __COMMON_REPO__USER__!")
            .unwrap();

        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("version.txt", "Version: __COMMON_REPO__VERSION__")
            .unwrap();

        // Mark templates
        let template_op = crate::config::TemplateOp {
            patterns: vec!["*.txt".to_string()],
        };
        crate::operators::template::mark(&template_op, &mut fs1).unwrap();
        crate::operators::template::mark(&template_op, &mut fs2).unwrap();

        // Template variables
        let mut vars1 = HashMap::new();
        vars1.insert("USER".to_string(), "Alice".to_string());

        let mut vars2 = HashMap::new();
        vars2.insert("VERSION".to_string(), "2.1.0".to_string());

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new_with_vars(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                vars1,
            ),
        );
        intermediate_fss.insert(
            "https://github.com/repo-b.git@main".to_string(),
            IntermediateFS::new_with_vars(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
                vars2,
            ),
        );

        let order = OperationOrder::new(vec![
            "https://github.com/repo-a.git@main".to_string(),
            "https://github.com/repo-b.git@main".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Both templates should be processed
        let greeting_file = composite.get_file("greeting.txt").unwrap();
        let greeting_content = String::from_utf8(greeting_file.content.clone()).unwrap();
        assert_eq!(greeting_content, "Hello Alice!");

        let version_file = composite.get_file("version.txt").unwrap();
        let version_content = String::from_utf8(version_file.content.clone()).unwrap();
        assert_eq!(version_content, "Version: 2.1.0");

        // Both should be unmarked as templates
        assert!(!greeting_file.is_template);
        assert!(!version_file.is_template);
    }

    #[test]
    fn test_phase4_execute_with_json_merge_operations() {
        // Test that JSON merge operations are executed during Phase 4
        use crate::config::{JsonMergeOp, Operation};

        // Create a filesystem with source and destination JSON files
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("fragment.json", r#"{"newKey": "newValue"}"#)
            .unwrap();
        fs1.add_file_string("package.json", r#"{"name": "test-package"}"#)
            .unwrap();

        // Create a JSON merge operation
        let json_merge_op = JsonMergeOp {
            source: Some("fragment.json".to_string()),
            dest: Some("package.json".to_string()),
            ..Default::default()
        };

        let merge_operations = vec![Operation::Json {
            json: json_merge_op,
        }];

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new_with_vars_and_merges(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                HashMap::new(),
                merge_operations,
            ),
        );

        let order = OperationOrder::new(vec!["https://github.com/repo-a.git@main".to_string()]);

        let (composite, deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Merge ops should be collected, not executed
        assert_eq!(deferred_ops.len(), 1);
        assert!(matches!(deferred_ops[0], Operation::Json { .. }));

        // package.json should still have original content (not merged)
        assert!(composite.exists("package.json"));
        let package_file = composite.get_file("package.json").unwrap();
        let content = String::from_utf8(package_file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["name"], "test-package");
        assert!(json.get("newKey").is_none());
    }

    #[test]
    fn test_phase4_execute_with_ini_merge_operations() {
        // Test that INI merge operations are executed during Phase 4
        use crate::config::{IniMergeOp, Operation};

        // Create a filesystem with source and destination INI files
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string(
            "fragment.ini",
            r#"
[database]
pool_size = 10
timeout = 30
"#,
        )
        .unwrap();
        fs1.add_file_string(
            "config.ini",
            r#"
[database]
host = localhost
port = 5432

[server]
port = 8080
"#,
        )
        .unwrap();

        // Create an INI merge operation
        let ini_merge_op = IniMergeOp {
            source: Some("fragment.ini".to_string()),
            dest: Some("config.ini".to_string()),
            section: Some("database".to_string()),
            ..Default::default()
        };

        let merge_operations = vec![Operation::Ini { ini: ini_merge_op }];

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new_with_vars_and_merges(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                HashMap::new(),
                merge_operations,
            ),
        );

        let order = OperationOrder::new(vec!["https://github.com/repo-a.git@main".to_string()]);

        let (composite, deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Merge ops should be collected, not executed
        assert_eq!(deferred_ops.len(), 1);
        assert!(matches!(deferred_ops[0], Operation::Ini { .. }));

        // config.ini should still have original content (not merged with fragment)
        assert!(composite.exists("config.ini"));
        let config_file = composite.get_file("config.ini").unwrap();
        let content = String::from_utf8(config_file.content.clone()).unwrap();
        assert!(content.contains("[database]"));
        assert!(content.contains("host = localhost"));
        // Fragment values should NOT be merged yet
        assert!(!content.contains("pool_size"));
        assert!(!content.contains("timeout"));
    }

    // ========================================================================
    // execute_merge_operation Tests
    // ========================================================================

    mod execute_merge_operation_tests {
        use super::*;
        use crate::config::{
            ExcludeOp, IncludeOp, InsertPosition, MarkdownMergeOp, Operation, TomlMergeOp,
            YamlMergeOp,
        };
        use crate::phases::composite::execute_merge_operation;

        #[test]
        fn test_execute_merge_operation_yaml() {
            // Test executing a YAML merge operation
            let mut fs = MemoryFS::new();
            fs.add_file_string("fragment.yaml", "new_key: new_value")
                .unwrap();
            fs.add_file_string("config.yaml", "existing_key: existing_value")
                .unwrap();

            let operation = Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("fragment.yaml".to_string()),
                    dest: Some("config.yaml".to_string()),
                    ..Default::default()
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_ok());

            // Verify the merge happened
            let content = fs.get_file("config.yaml").unwrap();
            let content_str = String::from_utf8(content.content.clone()).unwrap();
            assert!(content_str.contains("new_key"));
            assert!(content_str.contains("existing_key"));
        }

        #[test]
        fn test_execute_merge_operation_toml() {
            // Test executing a TOML merge operation
            let mut fs = MemoryFS::new();
            fs.add_file_string("fragment.toml", "value = 42").unwrap();
            fs.add_file_string("config.toml", "[section]\noriginal = true")
                .unwrap();

            let operation = Operation::Toml {
                toml: TomlMergeOp {
                    source: Some("fragment.toml".to_string()),
                    dest: Some("config.toml".to_string()),
                    path: Some("section".to_string()),
                    ..Default::default()
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_ok());

            // Verify the merge happened
            let content = fs.get_file("config.toml").unwrap();
            let content_str = String::from_utf8(content.content.clone()).unwrap();
            assert!(content_str.contains("section"));
            assert!(content_str.contains("value"));
        }

        #[test]
        fn test_execute_merge_operation_markdown() {
            // Test executing a Markdown merge operation
            let mut fs = MemoryFS::new();
            fs.add_file_string("fragment.md", "New content to insert")
                .unwrap();
            fs.add_file_string(
                "README.md",
                "# Title\n\n## Features\n\nExisting content\n\n## Other",
            )
            .unwrap();

            let operation = Operation::Markdown {
                markdown: MarkdownMergeOp {
                    source: Some("fragment.md".to_string()),
                    dest: Some("README.md".to_string()),
                    section: "Features".to_string(),
                    append: true,
                    level: 2,
                    position: InsertPosition::End,
                    ..Default::default()
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_ok());

            // Verify the merge happened
            let content = fs.get_file("README.md").unwrap();
            let content_str = String::from_utf8(content.content.clone()).unwrap();
            assert!(content_str.contains("Features"));
            assert!(content_str.contains("New content"));
        }

        #[test]
        fn test_execute_merge_operation_non_merge_operation_include() {
            // Test that non-merge operations return an error
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "content").unwrap();

            let operation = Operation::Include {
                include: IncludeOp {
                    patterns: vec!["**/*".to_string()],
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_err());

            // Verify the error message
            if let Err(Error::Filesystem { message }) = result {
                assert!(message.contains("Unexpected non-merge operation"));
            } else {
                panic!("Expected Filesystem error");
            }
        }

        #[test]
        fn test_execute_merge_operation_non_merge_operation_exclude() {
            // Test that Exclude operations return an error
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "content").unwrap();

            let operation = Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_err());

            if let Err(Error::Filesystem { message }) = result {
                assert!(message.contains("Unexpected non-merge operation"));
            } else {
                panic!("Expected Filesystem error");
            }
        }

        #[test]
        fn test_execute_merge_operation_missing_source_file() {
            // Test error handling when source file doesn't exist
            let mut fs = MemoryFS::new();
            fs.add_file_string("config.yaml", "existing: value")
                .unwrap();
            // Note: fragment.yaml does NOT exist

            let operation = Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("nonexistent.yaml".to_string()),
                    dest: Some("config.yaml".to_string()),
                    ..Default::default()
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            assert!(result.is_err());
        }

        #[test]
        fn test_execute_merge_operation_creates_dest_if_missing() {
            // Test that YAML merge creates destination file if it doesn't exist
            let mut fs = MemoryFS::new();
            fs.add_file_string("fragment.yaml", "new: value").unwrap();
            // Note: nonexistent.yaml does NOT exist initially

            let operation = Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("fragment.yaml".to_string()),
                    dest: Some("nonexistent.yaml".to_string()),
                    ..Default::default()
                },
            };

            let result = execute_merge_operation(&mut fs, &operation);
            // YAML merge creates the destination file if it doesn't exist
            assert!(result.is_ok());
            assert!(fs.exists("nonexistent.yaml"));

            // Verify the content was merged
            let content = fs.get_file("nonexistent.yaml").unwrap();
            let content_str = String::from_utf8(content.content.clone()).unwrap();
            assert!(content_str.contains("new"));
        }
    }

    // ========================================================================
    // merge_filesystem Tests
    // ========================================================================

    mod merge_filesystem_tests {
        use super::*;
        use crate::phases::composite::merge_filesystem;

        #[test]
        fn test_merge_filesystem_empty_source() {
            let mut target = MemoryFS::new();
            target.add_file_string("existing.txt", "content").unwrap();

            let source = MemoryFS::new();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 1);
            assert!(target.exists("existing.txt"));
        }

        #[test]
        fn test_merge_filesystem_empty_target() {
            let mut target = MemoryFS::new();

            let mut source = MemoryFS::new();
            source.add_file_string("new.txt", "new content").unwrap();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 1);
            assert!(target.exists("new.txt"));
        }

        #[test]
        fn test_merge_filesystem_no_conflicts() {
            let mut target = MemoryFS::new();
            target.add_file_string("file1.txt", "content1").unwrap();

            let mut source = MemoryFS::new();
            source.add_file_string("file2.txt", "content2").unwrap();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 2);
            assert!(target.exists("file1.txt"));
            assert!(target.exists("file2.txt"));
        }

        #[test]
        fn test_merge_filesystem_with_conflicts_source_wins() {
            let mut target = MemoryFS::new();
            target
                .add_file_string("common.txt", "target version")
                .unwrap();

            let mut source = MemoryFS::new();
            source
                .add_file_string("common.txt", "source version")
                .unwrap();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 1);

            // Source should overwrite target (last-write-wins)
            let file = target.get_file("common.txt").unwrap();
            assert_eq!(
                String::from_utf8(file.content.clone()).unwrap(),
                "source version"
            );
        }

        #[test]
        fn test_merge_filesystem_nested_directories() {
            let mut target = MemoryFS::new();
            target
                .add_file_string("dir1/file.txt", "target content")
                .unwrap();

            let mut source = MemoryFS::new();
            source
                .add_file_string("dir2/file.txt", "source content")
                .unwrap();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 2);
            assert!(target.exists("dir1/file.txt"));
            assert!(target.exists("dir2/file.txt"));
        }

        #[test]
        fn test_merge_filesystem_multiple_files() {
            let mut target = MemoryFS::new();
            target.add_file_string("a.txt", "a").unwrap();
            target.add_file_string("b.txt", "b").unwrap();

            let mut source = MemoryFS::new();
            source.add_file_string("c.txt", "c").unwrap();
            source.add_file_string("d.txt", "d").unwrap();
            source.add_file_string("e.txt", "e").unwrap();

            let result = merge_filesystem(&mut target, &source);
            assert!(result.is_ok());
            assert_eq!(target.len(), 5);
        }
    }

    #[test]
    fn test_phase4_collects_deferred_merge_ops_without_executing() {
        use crate::config::{JsonMergeOp, Operation};

        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("fragment.json", r#"{"newKey": "newValue"}"#)
            .unwrap();
        fs1.add_file_string("package.json", r#"{"name": "test-package"}"#)
            .unwrap();

        let json_merge_op = JsonMergeOp {
            source: Some("fragment.json".to_string()),
            dest: Some("package.json".to_string()),
            ..Default::default()
        };
        let merge_operations = vec![Operation::Json {
            json: json_merge_op,
        }];

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/repo-a.git@main".to_string(),
            IntermediateFS::new_with_vars_and_merges(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                HashMap::new(),
                merge_operations,
            ),
        );

        let order = OperationOrder::new(vec!["https://github.com/repo-a.git@main".to_string()]);

        let (composite, deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        // Deferred ops should be collected, not executed
        assert_eq!(deferred_ops.len(), 1);
        assert!(matches!(deferred_ops[0], Operation::Json { .. }));

        // package.json should NOT have been merged (still original content)
        let package_file = composite.get_file("package.json").unwrap();
        let content = String::from_utf8(package_file.content.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["name"], "test-package");
        assert!(json.get("newKey").is_none()); // NOT merged
    }

    #[test]
    fn test_phase4_source_template_vars_overridden_by_consumer() {
        // Simulates: upstream repo declares template + template-vars defaults,
        // consumer overrides some vars via with: clause
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string(
            "workflow.yaml",
            "app_id: __COMMON_REPO__GH_APP_ID_VAR__\napp_key: __COMMON_REPO__GH_APP_KEY_SECRET__\nowner: __COMMON_REPO__GH_APP_OWNER__",
        )
        .unwrap();

        // Mark as template (upstream repo declared template:)
        let template_op = crate::config::TemplateOp {
            patterns: vec!["workflow.yaml".to_string()],
        };
        crate::operators::template::mark(&template_op, &mut fs1).unwrap();

        // Upstream repo's default template-vars
        let mut source_vars = HashMap::new();
        source_vars.insert(
            "GH_APP_ID_VAR".to_string(),
            "CHRISTMAS_ISLAND_APP_ID".to_string(),
        );
        source_vars.insert(
            "GH_APP_KEY_SECRET".to_string(),
            "CHRISTMAS_ISLAND_PRIVATE_KEY".to_string(),
        );
        source_vars.insert("GH_APP_OWNER".to_string(), "christmas-island".to_string());

        // Consumer's override vars (only overrides owner)
        let mut consumer_vars = HashMap::new();
        consumer_vars.insert("GH_APP_OWNER".to_string(), "my-org".to_string());

        let mut intermediate_fss = HashMap::new();
        // Upstream repo provides the filesystem + default vars
        intermediate_fss.insert(
            "https://github.com/upstream-repo.git@main".to_string(),
            IntermediateFS::new_with_vars(
                fs1,
                "https://github.com/upstream-repo.git".to_string(),
                "main".to_string(),
                source_vars,
            ),
        );
        // Consumer provides override vars (empty filesystem, just vars)
        intermediate_fss.insert(
            "local@local".to_string(),
            IntermediateFS::new_with_vars(
                MemoryFS::new(),
                "local".to_string(),
                "local".to_string(),
                consumer_vars,
            ),
        );

        // Upstream first, then consumer (consumer overrides)
        let order = OperationOrder::new(vec![
            "https://github.com/upstream-repo.git@main".to_string(),
            "local@local".to_string(),
        ]);

        let (composite, _deferred_ops) = execute(&order, &intermediate_fss).unwrap();

        let file = composite.get_file("workflow.yaml").unwrap();
        let content = String::from_utf8(file.content.clone()).unwrap();

        // Upstream defaults preserved for non-overridden vars
        assert!(content.contains("CHRISTMAS_ISLAND_APP_ID"));
        assert!(content.contains("CHRISTMAS_ISLAND_PRIVATE_KEY"));
        // Consumer override applied
        assert!(content.contains("my-org"));
        assert!(!content.contains("christmas-island"));
    }

    // ===================================================================
    // Auto-merge composition tests
    // ===================================================================

    mod auto_merge_composition_tests {
        use super::*;
        use crate::config::{ArrayMergeMode, Operation, YamlMergeOp};

        #[test]
        fn test_auto_merge_combines_yaml_from_two_repos() {
            // Simulates: pre-commit provides builtin hooks, conventional-commits
            // provides the conventional hook. Both declare auto-merge for
            // .pre-commit-config.yaml. Consumer should get both.
            let pre_commit_yaml = r#"repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: trailing-whitespace
      - id: check-json
"#;
            let conventional_yaml = r#"repos:
  - repo: https://github.com/compilerla/conventional-pre-commit
    rev: v3.1.0
    hooks:
      - id: conventional-pre-commit
"#;

            let mut fs_pre_commit = MemoryFS::new();
            fs_pre_commit
                .add_file_string(".pre-commit-config.yaml", pre_commit_yaml)
                .unwrap();

            let mut fs_conventional = MemoryFS::new();
            fs_conventional
                .add_file_string(".pre-commit-config.yaml", conventional_yaml)
                .unwrap();

            // pre-commit repo's intermediate FS with auto-merge op
            let mut ifs_pre_commit = IntermediateFS::new(
                fs_pre_commit,
                "https://github.com/common-repo/pre-commit.git".to_string(),
                "v1.0.0".to_string(),
            );
            ifs_pre_commit.merge_operations.push(Operation::Yaml {
                yaml: YamlMergeOp::new()
                    .auto_merge(".pre-commit-config.yaml")
                    .array_mode(ArrayMergeMode::Append),
            });

            // conventional-commits repo's intermediate FS with auto-merge op
            let mut ifs_conventional = IntermediateFS::new(
                fs_conventional,
                "https://github.com/common-repo/conventional-commits.git".to_string(),
                "v1.0.0".to_string(),
            );
            ifs_conventional.merge_operations.push(Operation::Yaml {
                yaml: YamlMergeOp::new()
                    .auto_merge(".pre-commit-config.yaml")
                    .array_mode(ArrayMergeMode::Append),
            });

            let mut intermediate_fss = HashMap::new();
            let key_pre = "https://github.com/common-repo/pre-commit.git@v1.0.0".to_string();
            let key_conv =
                "https://github.com/common-repo/conventional-commits.git@v1.0.0".to_string();
            intermediate_fss.insert(key_pre.clone(), ifs_pre_commit);
            intermediate_fss.insert(key_conv.clone(), ifs_conventional);

            // pre-commit first (deepest in chain), then conventional-commits
            let order = OperationOrder::new(vec![key_pre, key_conv]);

            let (composite, deferred_ops) = execute(&order, &intermediate_fss).unwrap();

            // The composite should have .pre-commit-config.yaml with BOTH repos' hooks
            assert!(composite.exists(".pre-commit-config.yaml"));
            let content = String::from_utf8(
                composite
                    .get_file(".pre-commit-config.yaml")
                    .unwrap()
                    .content
                    .clone(),
            )
            .unwrap();

            assert!(
                content.contains("pre-commit-hooks"),
                "Should contain pre-commit-hooks repo: {}",
                content
            );
            assert!(
                content.contains("conventional-pre-commit"),
                "Should contain conventional-pre-commit repo: {}",
                content
            );

            // Deferred ops should still be collected for Phase 5
            assert_eq!(deferred_ops.len(), 2);

            // No temp files should remain
            assert!(!composite.exists(".__common_repo_auto_merge_temp__.pre-commit-config.yaml"));
        }

        #[test]
        fn test_no_auto_merge_still_uses_last_write_wins() {
            // When there's no auto-merge declaration, last-write-wins as before
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("shared.txt", "from-repo-a").unwrap();
            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("shared.txt", "from-repo-b").unwrap();

            let ifs1 = IntermediateFS::new(
                fs1,
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
            );
            let ifs2 = IntermediateFS::new(
                fs2,
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
            );

            let mut intermediate_fss = HashMap::new();
            let key_a = "https://github.com/repo-a.git@main".to_string();
            let key_b = "https://github.com/repo-b.git@main".to_string();
            intermediate_fss.insert(key_a.clone(), ifs1);
            intermediate_fss.insert(key_b.clone(), ifs2);

            let order = OperationOrder::new(vec![key_a, key_b]);
            let (composite, _) = execute(&order, &intermediate_fss).unwrap();

            let content =
                String::from_utf8(composite.get_file("shared.txt").unwrap().content.clone())
                    .unwrap();
            assert_eq!(content, "from-repo-b");
        }

        #[test]
        fn test_auto_merge_three_repos_accumulate() {
            // Three repos in chain all provide the same file with auto-merge
            let yaml_a = "repos:\n  - repo: repo-a\n    hooks:\n      - id: hook-a\n";
            let yaml_b = "repos:\n  - repo: repo-b\n    hooks:\n      - id: hook-b\n";
            let yaml_c = "repos:\n  - repo: repo-c\n    hooks:\n      - id: hook-c\n";

            let mut fs_a = MemoryFS::new();
            fs_a.add_file_string("config.yaml", yaml_a).unwrap();
            let mut fs_b = MemoryFS::new();
            fs_b.add_file_string("config.yaml", yaml_b).unwrap();
            let mut fs_c = MemoryFS::new();
            fs_c.add_file_string("config.yaml", yaml_c).unwrap();

            let make_ifs = |fs: MemoryFS, url: &str| {
                let mut ifs = IntermediateFS::new(fs, url.to_string(), "v1".to_string());
                ifs.merge_operations.push(Operation::Yaml {
                    yaml: YamlMergeOp::new()
                        .auto_merge("config.yaml")
                        .array_mode(ArrayMergeMode::Append),
                });
                ifs
            };

            let mut intermediate_fss = HashMap::new();
            let key_a = "a@v1".to_string();
            let key_b = "b@v1".to_string();
            let key_c = "c@v1".to_string();
            intermediate_fss.insert(key_a.clone(), make_ifs(fs_a, "a"));
            intermediate_fss.insert(key_b.clone(), make_ifs(fs_b, "b"));
            intermediate_fss.insert(key_c.clone(), make_ifs(fs_c, "c"));

            let order = OperationOrder::new(vec![key_a, key_b, key_c]);
            let (composite, _) = execute(&order, &intermediate_fss).unwrap();

            let content =
                String::from_utf8(composite.get_file("config.yaml").unwrap().content.clone())
                    .unwrap();

            assert!(content.contains("repo-a"), "Missing repo-a: {}", content);
            assert!(content.contains("repo-b"), "Missing repo-b: {}", content);
            assert!(content.contains("repo-c"), "Missing repo-c: {}", content);
        }

        #[test]
        fn test_first_repo_no_conflict_just_adds() {
            // First repo in order has auto-merge but no conflict (first occurrence)
            // Should just add the file normally
            let yaml = "repos:\n  - repo: first\n";

            let mut fs = MemoryFS::new();
            fs.add_file_string("config.yaml", yaml).unwrap();

            let mut ifs = IntermediateFS::new(
                fs,
                "https://github.com/first.git".to_string(),
                "v1".to_string(),
            );
            ifs.merge_operations.push(Operation::Yaml {
                yaml: YamlMergeOp::new()
                    .auto_merge("config.yaml")
                    .array_mode(ArrayMergeMode::Append),
            });

            let mut intermediate_fss = HashMap::new();
            let key = "https://github.com/first.git@v1".to_string();
            intermediate_fss.insert(key.clone(), ifs);

            let order = OperationOrder::new(vec![key]);
            let (composite, _) = execute(&order, &intermediate_fss).unwrap();

            let content =
                String::from_utf8(composite.get_file("config.yaml").unwrap().content.clone())
                    .unwrap();
            assert!(content.contains("first"));
        }
    }
}
