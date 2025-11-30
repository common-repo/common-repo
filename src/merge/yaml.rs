//! YAML merge operations
//!
//! This module provides functionality for merging YAML documents with support
//! for path navigation and various merge strategies for different value types.
//!
//! ## Features
//!
//! - Deep merging of YAML mappings with recursive descent
//! - Array merge modes: append, replace, append-unique
//! - Path-based targeting to merge at specific locations
//! - Type mismatch handling with warnings
//!
//! ## Example
//!
//! ```ignore
//! use common_repo::merge::yaml::apply_yaml_merge_operation;
//! use common_repo::config::YamlMergeOp;
//! use common_repo::filesystem::MemoryFS;
//!
//! let mut fs = MemoryFS::new();
//! // ... populate fs with source and dest files ...
//! let op = YamlMergeOp { /* ... */ };
//! apply_yaml_merge_operation(&mut fs, &op)?;
//! ```

use log::warn;
use serde_yaml::Value as YamlValue;

use super::PathSegment;
use crate::config::{ArrayMergeMode, YamlMergeOp};
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Navigate to a specific path within a YAML value, creating intermediate
/// structures as needed.
///
/// Supports both mapping (object) and sequence (array) navigation. For mappings,
/// uses string keys; for sequences, uses numeric indices. Creates missing
/// intermediate structures automatically.
///
/// # Arguments
///
/// * `value` - The root YAML value to navigate
/// * `path` - Slice of path segments to follow
///
/// # Returns
///
/// A mutable reference to the target value, or an error if the path is invalid
/// (e.g., trying to index into a scalar value).
///
/// # Errors
///
/// Returns `Error::Merge` if the path cannot be navigated due to type mismatches.
pub fn navigate_yaml_value<'a>(
    value: &'a mut YamlValue,
    path: &[PathSegment],
) -> Result<&'a mut YamlValue> {
    let mut current = value;
    for segment in path {
        match segment {
            PathSegment::Key(key) => {
                if !current.is_mapping() && !current.is_null() {
                    return Err(Error::Merge {
                        operation: "yaml merge".to_string(),
                        message: format!("Expected mapping while navigating to '{}'", key),
                    });
                }

                if current.is_null() {
                    *current = YamlValue::Mapping(Default::default());
                }

                let map = current.as_mapping_mut().unwrap();
                current = map
                    .entry(YamlValue::String(key.clone()))
                    .or_insert(YamlValue::Mapping(Default::default()));
            }
            PathSegment::Index(idx) => {
                if !current.is_sequence() && !current.is_null() {
                    return Err(Error::Merge {
                        operation: "yaml merge".to_string(),
                        message: format!("Expected sequence while navigating to index {}", idx),
                    });
                }

                if current.is_null() {
                    *current = YamlValue::Sequence(Vec::new());
                }

                let seq = current.as_sequence_mut().unwrap();
                while seq.len() <= *idx {
                    seq.push(YamlValue::Null);
                }
                current = &mut seq[*idx];
            }
        }
    }

    Ok(current)
}

/// Recursively merge source YAML value into target
///
/// Handles different YAML types appropriately:
/// - Mappings: Recursively merge keys, with source values taking precedence for conflicts
/// - Sequences: Apply array merge mode (append, replace, or append-unique)
/// - Scalars: Replace target with source (with warning)
///
/// # Arguments
///
/// * `target` - The target value to merge into (modified in place)
/// * `source` - The source value to merge from
/// * `mode` - How to handle array merging
/// * `path` - Current path for logging purposes
/// * `src_file` - Source file path for logging
/// * `dst_file` - Destination file path for logging
pub fn merge_yaml_values(
    target: &mut YamlValue,
    source: &YamlValue,
    mode: ArrayMergeMode,
    path: &str,
    src_file: &str,
    dst_file: &str,
) {
    match target {
        YamlValue::Mapping(target_map) => {
            if let YamlValue::Mapping(source_map) = source {
                for (key, value) in source_map {
                    let key_str = match key {
                        YamlValue::String(s) => s.clone(),
                        _ => format!("{:?}", key),
                    };
                    let new_path = if path.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{}.{}", path, key_str)
                    };

                    if let Some(existing) = target_map.get_mut(key) {
                        if existing.as_mapping().is_some() && value.as_mapping().is_some() {
                            merge_yaml_values(existing, value, mode, &new_path, src_file, dst_file);
                        } else if let Some(source_seq) = value.as_sequence() {
                            if let Some(target_seq) = existing.as_sequence_mut() {
                                match mode {
                                    ArrayMergeMode::Append => {
                                        target_seq.extend(source_seq.iter().cloned());
                                    }
                                    ArrayMergeMode::Replace => {
                                        warn!(
                                            "{} -> {}: Replacing array at path '{}' (old size: {}, new size: {})",
                                            src_file, dst_file, new_path, target_seq.len(), source_seq.len()
                                        );
                                        *existing = YamlValue::Sequence(source_seq.clone());
                                    }
                                    ArrayMergeMode::AppendUnique => {
                                        for item in source_seq {
                                            if !target_seq.contains(item) {
                                                target_seq.push(item.clone());
                                            }
                                        }
                                    }
                                }
                            } else {
                                warn!(
                                    "{} -> {}: Type mismatch at path '{}': replacing {:?} with Sequence",
                                    src_file, dst_file, new_path, get_yaml_type_name(existing)
                                );
                                *existing = YamlValue::Sequence(source_seq.clone());
                            }
                        } else {
                            warn!(
                                "{} -> {}: Overwriting value at path '{}': {:?} -> {:?}",
                                src_file,
                                dst_file,
                                new_path,
                                get_yaml_type_name(existing),
                                get_yaml_type_name(value)
                            );
                            *existing = value.clone();
                        }
                    } else {
                        target_map.insert(key.clone(), value.clone());
                    }
                }
            } else {
                warn!(
                    "{} -> {}: Type mismatch at path '{}': replacing Mapping with {:?}",
                    src_file,
                    dst_file,
                    path,
                    get_yaml_type_name(source)
                );
                *target = source.clone();
            }
        }
        YamlValue::Sequence(target_seq) => {
            if let YamlValue::Sequence(source_seq) = source {
                match mode {
                    ArrayMergeMode::Append => {
                        target_seq.extend(source_seq.clone());
                    }
                    ArrayMergeMode::Replace => {
                        warn!(
                            "{} -> {}: Replacing array at path '{}' (old size: {}, new size: {})",
                            src_file,
                            dst_file,
                            path,
                            target_seq.len(),
                            source_seq.len()
                        );
                        *target = YamlValue::Sequence(source_seq.clone());
                    }
                    ArrayMergeMode::AppendUnique => {
                        for item in source_seq {
                            if !target_seq.contains(item) {
                                target_seq.push(item.clone());
                            }
                        }
                    }
                }
            } else {
                warn!(
                    "{} -> {}: Type mismatch at path '{}': replacing Sequence with {:?}",
                    src_file,
                    dst_file,
                    path,
                    get_yaml_type_name(source)
                );
                *target = source.clone();
            }
        }
        _ => {
            warn!(
                "{} -> {}: Overwriting scalar at path '{}': {:?} -> {:?}",
                src_file,
                dst_file,
                path,
                get_yaml_type_name(target),
                get_yaml_type_name(source)
            );
            *target = source.clone();
        }
    }
}

/// Get a human-readable type name for a YAML value
///
/// Used for logging and error messages to describe the type of a value.
pub fn get_yaml_type_name(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "Null",
        YamlValue::Bool(_) => "Bool",
        YamlValue::Number(_) => "Number",
        YamlValue::String(_) => "String",
        YamlValue::Sequence(_) => "Sequence",
        YamlValue::Mapping(_) => "Mapping",
        YamlValue::Tagged(_) => "Tagged",
    }
}

/// Apply a YAML merge operation to the filesystem
///
/// Reads the source and destination YAML files, merges them according to the
/// operation's configuration, and writes the result back to the destination.
///
/// # Arguments
///
/// * `fs` - The memory filesystem containing the files
/// * `op` - The merge operation configuration
///
/// # Returns
///
/// `Ok(())` on success, or an error if the merge fails.
///
/// # Errors
///
/// Returns `Error::Merge` if:
/// - Source file cannot be read
/// - Source YAML is invalid
/// - Path navigation fails
/// - Result cannot be serialized
pub fn apply_yaml_merge_operation(fs: &mut MemoryFS, op: &YamlMergeOp) -> Result<()> {
    let source_content = read_file_as_string(fs, &op.source)?;
    let dest_content =
        read_file_as_string_optional(fs, &op.dest)?.unwrap_or_else(|| "---\n".to_string());

    let mut dest_value: YamlValue =
        serde_yaml::from_str(&dest_content).unwrap_or(YamlValue::Mapping(Default::default()));
    let source_value: YamlValue =
        serde_yaml::from_str(&source_content).map_err(|err| Error::Merge {
            operation: "yaml merge".to_string(),
            message: format!("Failed to parse source YAML: {}", err),
        })?;

    let path_str = op.path.as_deref().unwrap_or("");
    let path = super::parse_path(path_str);
    let target = navigate_yaml_value(&mut dest_value, &path)?;
    let mode = op.get_array_mode();
    merge_yaml_values(target, &source_value, mode, path_str, &op.source, &op.dest);

    let serialized = serde_yaml::to_string(&dest_value).map_err(|err| Error::Merge {
        operation: "yaml merge".to_string(),
        message: format!("Failed to serialize YAML: {}", err),
    })?;

    write_string_to_file(fs, &op.dest, serialized)
}

// File I/O helpers

fn read_file_as_string(fs: &MemoryFS, path: &str) -> Result<String> {
    match fs.get_file(path) {
        Some(file) => String::from_utf8(file.content.clone()).map_err(|_| Error::Merge {
            operation: format!("read {}", path),
            message: "File content is not valid UTF-8".to_string(),
        }),
        None => Err(Error::Merge {
            operation: format!("read {}", path),
            message: "File not found in filesystem".to_string(),
        }),
    }
}

fn read_file_as_string_optional(fs: &MemoryFS, path: &str) -> Result<Option<String>> {
    if let Some(file) = fs.get_file(path) {
        Ok(Some(String::from_utf8(file.content.clone()).map_err(
            |_| Error::Merge {
                operation: format!("read {}", path),
                message: "File content is not valid UTF-8".to_string(),
            },
        )?))
    } else {
        Ok(None)
    }
}

fn ensure_trailing_newline(mut content: String) -> String {
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

fn write_string_to_file(fs: &mut MemoryFS, path: &str, content: String) -> Result<()> {
    let normalized = ensure_trailing_newline(content);
    fs.add_file(path, File::from_string(&normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod navigate_yaml_value_tests {
        use super::*;
        use crate::merge::parse_path;

        #[test]
        fn test_navigate_to_nested_key() {
            let mut value: YamlValue = serde_yaml::from_str("foo:\n  bar: 1").unwrap();
            let path = parse_path("foo.bar");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            assert_eq!(target, &YamlValue::Number(1.into()));
        }

        #[test]
        fn test_navigate_creates_missing_path() {
            let mut value = YamlValue::Null;
            let path = parse_path("foo.bar");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            // Should have created the path and returned an empty mapping
            assert!(target.is_mapping());
        }

        #[test]
        fn test_navigate_to_array_index() {
            let mut value: YamlValue = serde_yaml::from_str("items:\n  - a\n  - b").unwrap();
            let path = parse_path("items[1]");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            assert_eq!(target, &YamlValue::String("b".to_string()));
        }

        #[test]
        fn test_navigate_type_error() {
            let mut value: YamlValue = serde_yaml::from_str("foo: 42").unwrap();
            let path = parse_path("foo.bar");
            let result = navigate_yaml_value(&mut value, &path);
            assert!(result.is_err());
        }

        #[test]
        fn test_navigate_array_index_type_error() {
            // Trying to index into a scalar should fail
            let mut value: YamlValue = serde_yaml::from_str("foo: 42").unwrap();
            let path = parse_path("foo[0]");
            let result = navigate_yaml_value(&mut value, &path);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("Expected sequence"));
        }

        #[test]
        fn test_navigate_creates_array_from_null() {
            let mut value: YamlValue = serde_yaml::from_str("items: null").unwrap();
            let path = parse_path("items[2]");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            // Should have created an array with null values up to index 2
            assert!(target.is_null()); // The element at index 2
                                       // Verify parent is now an array
            let items = value.get("items").unwrap();
            assert!(items.is_sequence());
            assert_eq!(items.as_sequence().unwrap().len(), 3);
        }

        #[test]
        fn test_navigate_extends_array_to_index() {
            let mut value: YamlValue = serde_yaml::from_str("items:\n  - a").unwrap();
            let path = parse_path("items[5]");
            let _ = navigate_yaml_value(&mut value, &path).unwrap();
            let items = value.get("items").unwrap().as_sequence().unwrap();
            assert_eq!(items.len(), 6); // Extended to fit index 5
        }
    }

    mod yaml_merge_integration_tests {
        use super::*;

        #[test]
        fn test_yaml_merge_at_root() {
            let mut fs = MemoryFS::new();
            fs.add_file(
                "source.yaml",
                File::from_string("new_key: new_value\nexisting_key: updated"),
            )
            .unwrap();
            fs.add_file(
                "dest.yaml",
                File::from_string("existing_key: original\nother: data"),
            )
            .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();

            assert_eq!(
                parsed.get("new_key").unwrap(),
                &YamlValue::String("new_value".to_string())
            );
            assert_eq!(
                parsed.get("existing_key").unwrap(),
                &YamlValue::String("updated".to_string())
            );
            assert_eq!(
                parsed.get("other").unwrap(),
                &YamlValue::String("data".to_string())
            );
        }

        #[test]
        fn test_yaml_merge_at_path() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("timeout: 30\nretries: 3"))
                .unwrap();
            fs.add_file(
                "dest.yaml",
                File::from_string("database:\n  host: localhost\n  port: 5432"),
            )
            .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: Some("database.connection".to_string()),
                append: false,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();

            // Original keys preserved
            assert_eq!(
                parsed["database"]["host"],
                YamlValue::String("localhost".to_string())
            );
            // New nested path created
            assert_eq!(
                parsed["database"]["connection"]["timeout"],
                YamlValue::Number(30.into())
            );
        }

        #[test]
        fn test_yaml_merge_with_escaped_dots() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("value: merged"))
                .unwrap();
            fs.add_file(
                "dest.yaml",
                File::from_string("special.key:\n  original: data"),
            )
            .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: Some(r"special\.key".to_string()),
                append: false,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            assert_eq!(
                parsed["special.key"]["value"],
                YamlValue::String("merged".to_string())
            );
        }

        #[test]
        fn test_yaml_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("key: value"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "new_dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            assert_eq!(parsed["key"], YamlValue::String("value".to_string()));
        }

        #[test]
        fn test_yaml_merge_array_mode_replace() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - x\n  - y"))
                .unwrap();
            fs.add_file(
                "dest.yaml",
                File::from_string("items:\n  - a\n  - b\n  - c"),
            )
            .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::Replace),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed["items"].as_sequence().unwrap();
            assert_eq!(items.len(), 2);
        }

        #[test]
        fn test_yaml_merge_array_mode_append() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - x\n  - y"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("items:\n  - a\n  - b"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::Append),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed["items"].as_sequence().unwrap();
            assert_eq!(items.len(), 4);
        }

        #[test]
        fn test_yaml_merge_array_mode_append_unique() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - b\n  - c"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("items:\n  - a\n  - b"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::AppendUnique),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed["items"].as_sequence().unwrap();
            // a, b, c - b is not duplicated
            assert_eq!(items.len(), 3);
        }

        #[test]
        fn test_yaml_merge_backward_compatibility_append_bool() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - x"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("items:\n  - a"))
                .unwrap();

            // Using old append: true style
            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: true,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed["items"].as_sequence().unwrap();
            assert_eq!(items.len(), 2);
        }

        #[test]
        fn test_yaml_merge_nested_path_array_mode() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("- new_item"))
                .unwrap();
            fs.add_file(
                "dest.yaml",
                File::from_string("config:\n  list:\n    - old_item"),
            )
            .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: Some("config.list".to_string()),
                append: true,
                array_mode: None,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let list = parsed["config"]["list"].as_sequence().unwrap();
            assert_eq!(list.len(), 2);
        }

        #[test]
        fn test_yaml_merge_type_mismatch_array_to_scalar() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - a\n  - b"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("items: scalar_value"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::Append),
            };

            // Should succeed with a warning (type mismatch replaces value)
            apply_yaml_merge_operation(&mut fs, &op).unwrap();
        }

        #[test]
        fn test_yaml_merge_append_unique_non_string_items() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("items:\n  - 2\n  - 3"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("items:\n  - 1\n  - 2"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::AppendUnique),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed["items"].as_sequence().unwrap();
            // 1, 2, 3 - 2 is not duplicated even though it's a number
            assert_eq!(items.len(), 3);
        }

        #[test]
        fn test_yaml_merge_top_level_sequence_append() {
            let mut fs = MemoryFS::new();
            // Both source and dest are top-level arrays
            fs.add_file("source.yaml", File::from_string("- x\n- y"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("- a\n- b"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::Append),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed.as_sequence().unwrap();
            assert_eq!(items.len(), 4); // a, b, x, y
        }

        #[test]
        fn test_yaml_merge_top_level_sequence_replace() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("- new"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("- old1\n- old2\n- old3"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::Replace),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed.as_sequence().unwrap();
            assert_eq!(items.len(), 1); // replaced with just [new]
        }

        #[test]
        fn test_yaml_merge_top_level_sequence_append_unique() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("- b\n- c"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("- a\n- b"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: Some(ArrayMergeMode::AppendUnique),
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.yaml").unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&result).unwrap();
            let items = parsed.as_sequence().unwrap();
            assert_eq!(items.len(), 3); // a, b, c - b not duplicated
        }

        #[test]
        fn test_yaml_merge_source_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("dest.yaml", File::from_string("key: value"))
                .unwrap();

            let op = YamlMergeOp {
                source: "nonexistent.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: None,
            };

            let result = apply_yaml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("File not found"));
        }

        #[test]
        fn test_yaml_merge_invalid_source_yaml() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.yaml", File::from_string("invalid: [yaml: {broken"))
                .unwrap();
            fs.add_file("dest.yaml", File::from_string("key: value"))
                .unwrap();

            let op = YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
                array_mode: None,
            };

            let result = apply_yaml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("parse source YAML"));
        }
    }

    mod helper_function_tests {
        use super::*;

        #[test]
        fn test_get_yaml_type_name_all_types() {
            assert_eq!(get_yaml_type_name(&YamlValue::Null), "Null");
            assert_eq!(get_yaml_type_name(&YamlValue::Bool(true)), "Bool");
            assert_eq!(get_yaml_type_name(&YamlValue::Number(42.into())), "Number");
            assert_eq!(
                get_yaml_type_name(&YamlValue::String("test".to_string())),
                "String"
            );
            assert_eq!(get_yaml_type_name(&YamlValue::Sequence(vec![])), "Sequence");
            assert_eq!(
                get_yaml_type_name(&YamlValue::Mapping(Default::default())),
                "Mapping"
            );
        }

        #[test]
        fn test_ensure_trailing_newline_adds_when_missing() {
            let input = "content".to_string();
            let result = ensure_trailing_newline(input);
            assert!(result.ends_with('\n'));
            assert_eq!(result, "content\n");
        }

        #[test]
        fn test_ensure_trailing_newline_preserves_existing() {
            let input = "content\n".to_string();
            let result = ensure_trailing_newline(input);
            assert_eq!(result, "content\n");
        }
    }
}
