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
//!     `${VAR}` placeholders with their final values.
//!
//! 3.  **Filesystem Merging**: After template processing, the `MemoryFS` from
//!     each `IntermediateFS` is merged into the composite filesystem. The merge
//!     is performed in the `OperationOrder`, which again ensures a "last-write-wins"
//!     behavior, where files from more specific repositories overwrite those
//!     from their ancestors.
//!
//! This phase produces a single `MemoryFS` that represents the complete,
//! inherited configuration, with all templates processed and all files merged,
//! ready for the final local merge in the next phase.

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
) -> Result<MemoryFS> {
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
    // Later filesystems in the order take precedence (last-write-wins)
    let mut composite_fs = MemoryFS::new();
    for repo_key in &order.order {
        if let Some(processed_fs) = processed_fss.get(repo_key) {
            merge_filesystem(&mut composite_fs, processed_fs)?;

            // Execute merge operations for this repository after its filesystem is merged
            if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
                for merge_op in &intermediate_fs.merge_operations {
                    execute_merge_operation(&mut composite_fs, merge_op)?;
                }
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

    Ok(composite_fs)
}

/// Merge a source filesystem into a target filesystem
///
/// All files from source_fs are copied to target_fs. If a file already exists
/// in target_fs, it is overwritten (last-write-wins strategy).
/// This preserves file metadata from the source filesystem.
fn merge_filesystem(target_fs: &mut MemoryFS, source_fs: &MemoryFS) -> Result<()> {
    for (path, file) in source_fs.files() {
        target_fs.add_file(path, file.clone())?;
    }
    Ok(())
}

/// Execute a single merge operation on the composite filesystem
///
/// This function dispatches to the appropriate merge operation handler
/// based on the operation type (YAML, JSON, TOML, INI, or Markdown).
fn execute_merge_operation(fs: &mut MemoryFS, operation: &Operation) -> Result<()> {
    match operation {
        Operation::Yaml { yaml } => crate::merge::yaml::apply_yaml_merge_operation(fs, yaml),
        Operation::Json { json } => crate::merge::json::apply_json_merge_operation(fs, json),
        Operation::Toml { toml } => crate::merge::toml::apply_toml_merge_operation(fs, toml),
        Operation::Ini { ini } => crate::merge::ini::apply_ini_merge_operation(fs, ini),
        Operation::Markdown { markdown } => {
            crate::merge::markdown::apply_markdown_merge_operation(fs, markdown)
        }
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

        let composite = execute(&order, &intermediate_fss).unwrap();

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

        let composite = execute(&order, &intermediate_fss).unwrap();

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

        let composite = execute(&order, &intermediate_fss).unwrap();

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
        fs1.add_file_string("template.txt", "Hello ${NAME} from ${REPO}!")
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

        let composite = execute(&order, &intermediate_fss).unwrap();

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
        fs1.add_file_string("greeting.txt", "Hello ${USER}!")
            .unwrap();

        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("version.txt", "Version: ${VERSION}")
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

        let composite = execute(&order, &intermediate_fss).unwrap();

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
            source: "fragment.json".to_string(),
            dest: "package.json".to_string(),
            path: "/".to_string(),
            append: false,
            position: "end".to_string(),
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

        let composite = execute(&order, &intermediate_fss).unwrap();

        // Verify that the merge operation was executed
        assert!(composite.exists("package.json"));
        let package_file = composite.get_file("package.json").unwrap();
        let content = String::from_utf8(package_file.content.clone()).unwrap();

        // Parse the JSON to verify the merge
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["name"], "test-package");
        assert_eq!(json["newKey"], "newValue");
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
            source: "fragment.ini".to_string(),
            dest: "config.ini".to_string(),
            section: Some("database".to_string()),
            append: false,
            allow_duplicates: false,
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

        let composite = execute(&order, &intermediate_fss).unwrap();

        // Verify that the merge operation was executed
        assert!(composite.exists("config.ini"));
        let config_file = composite.get_file("config.ini").unwrap();
        let content = String::from_utf8(config_file.content.clone()).unwrap();

        // Verify INI content has both sections
        assert!(content.contains("[database]"));
        assert!(content.contains("host=localhost"));
        assert!(content.contains("port=5432"));
        assert!(content.contains("pool_size=10"));
        assert!(content.contains("timeout=30"));
        assert!(content.contains("[server]"));
        assert!(content.contains("port=8080"));
    }
}
