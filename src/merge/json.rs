//! JSON merge operations
//!
//! This module provides functionality for merging JSON documents with support
//! for path navigation and recursive deep merging.
//!
//! ## Features
//!
//! - Deep merging of JSON objects with recursive descent
//! - Array handling with append or replace modes
//! - Path-based targeting to merge at specific locations
//!
//! ## Example
//!
//! ```ignore
//! use common_repo::merge::apply_json_merge_operation;
//! use common_repo::config::JsonMergeOp;
//! use common_repo::filesystem::MemoryFS;
//!
//! let mut fs = MemoryFS::new();
//! // ... populate fs with source and dest files ...
//! let op = JsonMergeOp { /* ... */ };
//! apply_json_merge_operation(&mut fs, &op)?;
//! ```

use serde_json::Value as JsonValue;

use super::PathSegment;
use crate::config::JsonMergeOp;
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Navigate to a specific path within a JSON value, creating intermediate
/// structures as needed.
///
/// Supports both object and array navigation. For objects, uses string keys;
/// for arrays, uses numeric indices. Creates missing intermediate structures
/// automatically.
///
/// # Arguments
///
/// * `value` - The root JSON value to navigate
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
pub fn navigate_json_value<'a>(
    value: &'a mut JsonValue,
    path: &[PathSegment],
) -> Result<&'a mut JsonValue> {
    let mut current = value;
    for segment in path {
        match segment {
            PathSegment::Key(key) => {
                if !current.is_object() && !current.is_null() {
                    return Err(Error::Merge {
                        operation: "json merge".to_string(),
                        message: format!("Expected object while navigating to '{}'", key),
                    });
                }

                if current.is_null() {
                    *current = JsonValue::Object(serde_json::Map::new());
                }

                let map = current.as_object_mut().unwrap();
                current = map
                    .entry(key.clone())
                    .or_insert(JsonValue::Object(serde_json::Map::new()));
            }
            PathSegment::Index(idx) => {
                if !current.is_array() && !current.is_null() {
                    return Err(Error::Merge {
                        operation: "json merge".to_string(),
                        message: format!("Expected array while navigating to index {}", idx),
                    });
                }

                if current.is_null() {
                    *current = JsonValue::Array(Vec::new());
                }

                let array = current.as_array_mut().unwrap();
                while array.len() <= *idx {
                    array.push(JsonValue::Null);
                }
                current = &mut array[*idx];
            }
        }
    }

    Ok(current)
}

/// Recursively merge source JSON value into target
///
/// Handles different JSON types appropriately:
/// - Objects: Recursively merge keys, with source values taking precedence for conflicts
/// - Arrays: Either append/prepend source items to target or replace entirely based on flags
/// - Scalars: Replace target with source
///
/// # Arguments
///
/// * `target` - The target value to merge into (modified in place)
/// * `source` - The source value to merge from
/// * `append` - If true, append/prepend array items; if false, replace arrays entirely
/// * `position` - Position for array insertion: "start" prepends, anything else appends
pub fn merge_json_values(
    target: &mut JsonValue,
    source: &JsonValue,
    append: bool,
    position: Option<&str>,
) {
    let prepend = position.map(|p| p == "start").unwrap_or(false);

    match target {
        JsonValue::Object(target_map) => {
            if let JsonValue::Object(source_map) = source {
                for (key, value) in source_map {
                    if let Some(existing) = target_map.get_mut(key) {
                        if existing.is_object() && value.is_object() {
                            merge_json_values(existing, value, append, position);
                        } else if let Some(source_array) = value.as_array() {
                            if let Some(target_array) = existing.as_array_mut() {
                                if append {
                                    if prepend {
                                        // Insert source items at the beginning
                                        let mut new_array = source_array.clone();
                                        new_array.append(target_array);
                                        *target_array = new_array;
                                    } else {
                                        target_array.extend(source_array.iter().cloned());
                                    }
                                } else {
                                    *existing = JsonValue::Array(source_array.clone());
                                }
                            } else if !append {
                                *existing = JsonValue::Array(source_array.clone());
                            }
                        } else if !append {
                            *existing = value.clone();
                        }
                    } else {
                        target_map.insert(key.clone(), value.clone());
                    }
                }
            } else {
                *target = source.clone();
            }
        }
        JsonValue::Array(target_array) => {
            if let JsonValue::Array(source_array) = source {
                if append {
                    if prepend {
                        // Insert source items at the beginning
                        let mut new_array = source_array.clone();
                        new_array.append(target_array);
                        *target_array = new_array;
                    } else {
                        target_array.extend(source_array.clone());
                    }
                } else {
                    *target = JsonValue::Array(source_array.clone());
                }
            } else {
                *target = source.clone();
            }
        }
        _ => *target = source.clone(),
    }
}

/// Apply a JSON merge operation to the filesystem
///
/// Reads the source and destination JSON files, merges them according to the
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
/// - Source JSON is invalid
/// - Path navigation fails
/// - Result cannot be serialized
pub fn apply_json_merge_operation(fs: &mut MemoryFS, op: &JsonMergeOp) -> Result<()> {
    op.validate()?;
    let source_path = op.get_source().expect("source validated");
    let dest_path = op.get_dest().expect("dest validated");

    let source_content = read_file_as_string(fs, source_path)?;
    let dest_content =
        read_file_as_string_optional(fs, dest_path)?.unwrap_or_else(|| "{}".to_string());

    let mut dest_value: JsonValue =
        serde_json::from_str(&dest_content).unwrap_or(JsonValue::Object(serde_json::Map::new()));
    let source_value: JsonValue =
        serde_json::from_str(&source_content).map_err(|err| Error::Merge {
            operation: "json merge".to_string(),
            message: format!("Failed to parse source JSON: {}", err),
        })?;

    let path = super::parse_path(op.path.as_deref().unwrap_or(""));
    let target = navigate_json_value(&mut dest_value, &path)?;
    merge_json_values(target, &source_value, op.append, op.position.as_deref());

    let serialized = serde_json::to_string_pretty(&dest_value).map_err(|err| Error::Merge {
        operation: "json merge".to_string(),
        message: format!("Failed to serialize JSON: {}", err),
    })?;

    write_string_to_file(fs, dest_path, serialized)
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

    mod navigate_json_value_tests {
        use super::*;
        use crate::merge::parse_path;

        #[test]
        fn test_navigate_to_nested_key() {
            let mut value: JsonValue = serde_json::from_str(r#"{"foo":{"bar":1}}"#).unwrap();
            let path = parse_path("foo.bar");
            let target = navigate_json_value(&mut value, &path).unwrap();
            assert_eq!(target, &JsonValue::Number(1.into()));
        }

        #[test]
        fn test_navigate_creates_missing_path() {
            let mut value = JsonValue::Null;
            let path = parse_path("foo.bar");
            let target = navigate_json_value(&mut value, &path).unwrap();
            // Should have created the path and returned an empty object
            assert!(target.is_object());
        }

        #[test]
        fn test_navigate_to_array_index() {
            let mut value: JsonValue = serde_json::from_str(r#"{"items":["a","b"]}"#).unwrap();
            let path = parse_path("items[1]");
            let target = navigate_json_value(&mut value, &path).unwrap();
            assert_eq!(target, &JsonValue::String("b".to_string()));
        }

        #[test]
        fn test_navigate_type_error() {
            let mut value: JsonValue = serde_json::from_str(r#"{"foo":42}"#).unwrap();
            let path = parse_path("foo.bar");
            let result = navigate_json_value(&mut value, &path);
            assert!(result.is_err());
        }
    }

    mod json_merge_integration_tests {
        use super::*;

        #[test]
        fn test_json_merge_at_root() {
            let mut fs = MemoryFS::new();
            fs.add_file(
                "source.json",
                File::from_string(r#"{"new_key": "new_value", "existing_key": "updated"}"#),
            )
            .unwrap();
            fs.add_file(
                "dest.json",
                File::from_string(r#"{"existing_key": "original", "other": "data"}"#),
            )
            .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();

            assert_eq!(
                parsed.get("new_key").unwrap(),
                &JsonValue::String("new_value".to_string())
            );
            assert_eq!(
                parsed.get("existing_key").unwrap(),
                &JsonValue::String("updated".to_string())
            );
            assert_eq!(
                parsed.get("other").unwrap(),
                &JsonValue::String("data".to_string())
            );
        }

        #[test]
        fn test_json_merge_at_path() {
            let mut fs = MemoryFS::new();
            fs.add_file(
                "source.json",
                File::from_string(r#"{"timeout": 30, "retries": 3}"#),
            )
            .unwrap();
            fs.add_file(
                "dest.json",
                File::from_string(r#"{"database": {"host": "localhost", "port": 5432}}"#),
            )
            .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                path: Some("database.connection".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();

            // Original keys preserved
            assert_eq!(
                parsed["database"]["host"],
                JsonValue::String("localhost".to_string())
            );
            // New nested path created
            assert_eq!(
                parsed["database"]["connection"]["timeout"],
                JsonValue::Number(30.into())
            );
        }

        #[test]
        fn test_json_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("new_dest.json".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            assert_eq!(parsed["key"], JsonValue::String("value".to_string()));
        }

        #[test]
        fn test_json_merge_array_append() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"items": ["x", "y"]}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"items": ["a", "b"]}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                append: true,
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            let items = parsed["items"].as_array().unwrap();
            assert_eq!(items.len(), 4);
        }

        #[test]
        fn test_json_merge_array_replace() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"items": ["x", "y"]}"#))
                .unwrap();
            fs.add_file(
                "dest.json",
                File::from_string(r#"{"items": ["a", "b", "c"]}"#),
            )
            .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            let items = parsed["items"].as_array().unwrap();
            assert_eq!(items.len(), 2);
        }

        #[test]
        fn test_json_merge_nested_objects() {
            let mut fs = MemoryFS::new();
            fs.add_file(
                "source.json",
                File::from_string(r#"{"config": {"nested": {"deep": "value"}}}"#),
            )
            .unwrap();
            fs.add_file(
                "dest.json",
                File::from_string(r#"{"config": {"existing": "data"}}"#),
            )
            .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();

            // Both existing and new data should be present
            assert_eq!(
                parsed["config"]["existing"],
                JsonValue::String("data".to_string())
            );
            assert_eq!(
                parsed["config"]["nested"]["deep"],
                JsonValue::String("value".to_string())
            );
        }
    }

    mod deep_merging_tests {
        use super::*;

        #[test]
        fn test_deep_merge_multiple_levels() {
            let mut target: JsonValue =
                serde_json::from_str(r#"{"a": {"b": {"c": {"d": 1}}, "e": 2}}"#).unwrap();
            let source: JsonValue =
                serde_json::from_str(r#"{"a": {"b": {"c": {"f": 3}}, "g": 4}}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            // Original value preserved
            assert_eq!(target["a"]["b"]["c"]["d"], JsonValue::Number(1.into()));
            assert_eq!(target["a"]["e"], JsonValue::Number(2.into()));
            // New values merged
            assert_eq!(target["a"]["b"]["c"]["f"], JsonValue::Number(3.into()));
            assert_eq!(target["a"]["g"], JsonValue::Number(4.into()));
        }

        #[test]
        fn test_deep_merge_preserves_sibling_keys() {
            let mut target: JsonValue = serde_json::from_str(
                r#"{"level1": {"keep": "original", "modify": {"old": "value"}}}"#,
            )
            .unwrap();
            let source: JsonValue =
                serde_json::from_str(r#"{"level1": {"modify": {"new": "data"}}}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(
                target["level1"]["keep"],
                JsonValue::String("original".to_string())
            );
            assert_eq!(
                target["level1"]["modify"]["old"],
                JsonValue::String("value".to_string())
            );
            assert_eq!(
                target["level1"]["modify"]["new"],
                JsonValue::String("data".to_string())
            );
        }

        #[test]
        fn test_deep_merge_array_inside_nested_objects() {
            let mut target: JsonValue =
                serde_json::from_str(r#"{"config": {"items": [1, 2, 3]}}"#).unwrap();
            let source: JsonValue =
                serde_json::from_str(r#"{"config": {"items": [4, 5]}}"#).unwrap();

            // With append=true
            merge_json_values(&mut target, &source, true, None);

            let items = target["config"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], JsonValue::Number(1.into()));
            assert_eq!(items[4], JsonValue::Number(5.into()));
        }

        #[test]
        fn test_deep_merge_empty_objects() {
            let mut target: JsonValue = serde_json::from_str(r#"{"a": {"b": {}}}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"a": {"b": {"c": 1}}}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target["a"]["b"]["c"], JsonValue::Number(1.into()));
        }

        #[test]
        fn test_deep_merge_with_null_values() {
            let mut target: JsonValue =
                serde_json::from_str(r#"{"a": null, "b": {"c": 1}}"#).unwrap();
            let source: JsonValue =
                serde_json::from_str(r#"{"a": {"nested": "value"}, "b": null}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            // Source overrides target with non-object values
            assert_eq!(
                target["a"]["nested"],
                JsonValue::String("value".to_string())
            );
            assert!(target["b"].is_null());
        }
    }

    mod type_conflict_tests {
        use super::*;

        #[test]
        fn test_type_conflict_object_replaces_scalar() {
            let mut target: JsonValue = serde_json::from_str(r#"{"key": "string_value"}"#).unwrap();
            let source: JsonValue =
                serde_json::from_str(r#"{"key": {"nested": "object"}}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert!(target["key"].is_object());
            assert_eq!(
                target["key"]["nested"],
                JsonValue::String("object".to_string())
            );
        }

        #[test]
        fn test_type_conflict_scalar_replaces_object() {
            let mut target: JsonValue =
                serde_json::from_str(r#"{"key": {"nested": "object"}}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"key": "new_string"}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target["key"], JsonValue::String("new_string".to_string()));
        }

        #[test]
        fn test_type_conflict_array_replaces_scalar_without_append() {
            let mut target: JsonValue = serde_json::from_str(r#"{"items": "not_array"}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert!(target["items"].is_array());
            assert_eq!(target["items"].as_array().unwrap().len(), 3);
        }

        #[test]
        fn test_type_conflict_array_vs_scalar_with_append_preserves_target() {
            let mut target: JsonValue = serde_json::from_str(r#"{"items": "not_array"}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();

            // With append=true, non-array target is preserved (line 142-144 in the source)
            merge_json_values(&mut target, &source, true, None);

            // Target remains unchanged because we can't append to a non-array
            assert_eq!(target["items"], JsonValue::String("not_array".to_string()));
        }

        #[test]
        fn test_type_conflict_scalar_replaces_array_without_append() {
            let mut target: JsonValue = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"items": "now_a_string"}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(
                target["items"],
                JsonValue::String("now_a_string".to_string())
            );
        }

        #[test]
        fn test_type_conflict_scalar_on_append_preserves_target_array() {
            let mut target: JsonValue = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"items": "now_a_string"}"#).unwrap();

            // With append=true, scalar source is NOT appended to array
            // Looking at the code: the source is NOT an array (line 128), so it falls through
            // to the else block (line 145-147), which checks !append and overwrites
            merge_json_values(&mut target, &source, true, None);

            // With append=true and scalar source on array target, target is preserved
            // Actually looking at the code more closely: line 145-147 handles this
            // It only updates if !append, so with append=true, target is preserved
            let items = target["items"].as_array().unwrap();
            assert_eq!(items.len(), 3);
        }

        #[test]
        fn test_source_object_replaces_target_completely() {
            let mut target: JsonValue = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"new": "object"}"#).unwrap();

            // Top-level replacement: array target, object source
            merge_json_values(&mut target, &source, false, None);

            assert!(target.is_object());
            assert_eq!(target["new"], JsonValue::String("object".to_string()));
        }

        #[test]
        fn test_source_array_replaces_target_object() {
            let mut target: JsonValue = serde_json::from_str(r#"{"key": "value"}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();

            // Top-level: object target, array source -> line 152-154 in Object arm
            merge_json_values(&mut target, &source, false, None);

            assert!(target.is_array());
            assert_eq!(target.as_array().unwrap().len(), 3);
        }
    }

    mod error_path_tests {
        use super::*;

        #[test]
        fn test_source_file_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("dest.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("nonexistent.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("File not found"));
        }

        #[test]
        fn test_invalid_source_json() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"invalid": json}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("parse source JSON"));
        }

        #[test]
        fn test_invalid_dest_json_defaults_to_empty_object() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"not valid json"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            // Invalid dest JSON defaults to empty object (line 205)
            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&content).unwrap();
            assert_eq!(parsed["key"], JsonValue::String("value".to_string()));
        }

        #[test]
        fn test_navigation_error_key_into_scalar() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"new": "data"}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"scalar": 42}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                path: Some("scalar.nested".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("Expected object"));
        }

        #[test]
        fn test_navigation_error_index_into_object() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"new": "data"}"#))
                .unwrap();
            fs.add_file(
                "dest.json",
                File::from_string(r#"{"obj": {"key": "value"}}"#),
            )
            .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                path: Some("obj[0]".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("Expected array"));
        }

        #[test]
        fn test_utf8_error_in_source() {
            let mut fs = MemoryFS::new();
            // Add file with invalid UTF-8 bytes
            fs.add_file("source.json", File::new(vec![0xFF, 0xFE, 0x00, 0x01]))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("UTF-8"));
        }

        #[test]
        fn test_utf8_error_in_dest() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();
            // Add dest file with invalid UTF-8 bytes
            fs.add_file("dest.json", File::new(vec![0xFF, 0xFE, 0x00, 0x01]))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                ..Default::default()
            };

            let result = apply_json_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(err_msg.contains("UTF-8"));
        }
    }

    mod array_position_tests {
        use super::*;

        #[test]
        fn test_array_prepend_with_position_start() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"items": ["x", "y"]}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"items": ["a", "b"]}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                append: true,
                position: Some("start".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            let items = parsed["items"].as_array().unwrap();

            // Source items should be at the start
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], JsonValue::String("x".to_string()));
            assert_eq!(items[1], JsonValue::String("y".to_string()));
            assert_eq!(items[2], JsonValue::String("a".to_string()));
            assert_eq!(items[3], JsonValue::String("b".to_string()));
        }

        #[test]
        fn test_array_append_with_position_end() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"items": ["x", "y"]}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"items": ["a", "b"]}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                append: true,
                position: Some("end".to_string()),
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            let items = parsed["items"].as_array().unwrap();

            // Source items should be at the end
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], JsonValue::String("a".to_string()));
            assert_eq!(items[1], JsonValue::String("b".to_string()));
            assert_eq!(items[2], JsonValue::String("x".to_string()));
            assert_eq!(items[3], JsonValue::String("y".to_string()));
        }

        #[test]
        fn test_top_level_array_prepend() {
            let mut target: JsonValue = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"[4, 5]"#).unwrap();

            merge_json_values(&mut target, &source, true, Some("start"));

            let arr = target.as_array().unwrap();
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], JsonValue::Number(4.into()));
            assert_eq!(arr[1], JsonValue::Number(5.into()));
            assert_eq!(arr[2], JsonValue::Number(1.into()));
        }

        #[test]
        fn test_top_level_array_append() {
            let mut target: JsonValue = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"[4, 5]"#).unwrap();

            merge_json_values(&mut target, &source, true, None);

            let arr = target.as_array().unwrap();
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], JsonValue::Number(1.into()));
            assert_eq!(arr[4], JsonValue::Number(5.into()));
        }

        #[test]
        fn test_position_ignored_when_append_false() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.json", File::from_string(r#"{"items": ["x"]}"#))
                .unwrap();
            fs.add_file("dest.json", File::from_string(r#"{"items": ["a", "b"]}"#))
                .unwrap();

            let op = JsonMergeOp {
                source: Some("source.json".to_string()),
                dest: Some("dest.json".to_string()),
                position: Some("start".to_string()), // Should be ignored
                ..Default::default()
            };

            apply_json_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.json").unwrap();
            let parsed: JsonValue = serde_json::from_str(&result).unwrap();
            let items = parsed["items"].as_array().unwrap();

            // Replace mode: only source items
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], JsonValue::String("x".to_string()));
        }
    }

    mod navigate_edge_case_tests {
        use super::*;
        use crate::merge::parse_path;

        #[test]
        fn test_navigate_extends_array_for_large_index() {
            let mut value: JsonValue = serde_json::from_str(r#"{"arr": [1]}"#).unwrap();
            let path = parse_path("arr[5]");

            {
                let target = navigate_json_value(&mut value, &path).unwrap();
                // Target should be null (auto-created element)
                assert!(target.is_null());
            }

            // Array should have been extended with nulls
            let arr = value["arr"].as_array().unwrap();
            assert_eq!(arr.len(), 6);
            assert!(arr[1].is_null());
            assert!(arr[4].is_null());
        }

        #[test]
        fn test_navigate_creates_array_from_null() {
            let mut value = JsonValue::Null;
            let path = parse_path("[0]");

            {
                let target = navigate_json_value(&mut value, &path).unwrap();
                assert!(target.is_null()); // First element
            }

            assert!(value.is_array());
            assert_eq!(value.as_array().unwrap().len(), 1);
        }

        #[test]
        fn test_navigate_nested_array_path_fails_when_object_at_intermediate() {
            // After key navigation, entry defaults to object, not array
            // So trying to index into that object should fail
            let mut value = JsonValue::Null;
            let path = parse_path("data[0][1]");

            let result = navigate_json_value(&mut value, &path);
            assert!(result.is_err());

            // Verify the structure that was created before failure
            assert!(value.is_object());
            assert!(value["data"].is_object()); // Default is object, not array
        }

        #[test]
        fn test_navigate_mixed_key_and_index_fails_when_object_at_intermediate() {
            // Similar: servers entry defaults to object, not array
            let mut value = JsonValue::Null;
            let path = parse_path("servers[0].host");

            let result = navigate_json_value(&mut value, &path);
            assert!(result.is_err());

            // Verify structure
            assert!(value.is_object());
            assert!(value["servers"].is_object()); // Default is object, not array
        }

        #[test]
        fn test_navigate_array_then_key_succeeds() {
            // Start with [0] to create an array, then navigate into first element
            let mut value = JsonValue::Null;
            let path = parse_path("[0].name");

            {
                let target = navigate_json_value(&mut value, &path).unwrap();
                assert!(target.is_object()); // name entry is an empty object
            }

            assert!(value.is_array());
            assert!(value[0].is_object());
            assert!(value[0]["name"].is_object());
        }

        #[test]
        fn test_navigate_empty_path_returns_root() {
            let mut value: JsonValue = serde_json::from_str(r#"{"key": "value"}"#).unwrap();
            let path: Vec<PathSegment> = Vec::new();

            let target = navigate_json_value(&mut value, &path).unwrap();

            assert!(target.is_object());
            assert_eq!(target["key"], JsonValue::String("value".to_string()));
        }
    }

    mod merge_values_direct_tests {
        use super::*;

        #[test]
        fn test_merge_scalar_into_scalar() {
            let mut target = JsonValue::String("old".to_string());
            let source = JsonValue::String("new".to_string());

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target, JsonValue::String("new".to_string()));
        }

        #[test]
        fn test_merge_number_types() {
            let mut target = JsonValue::Number(42.into());
            let source = JsonValue::Number(serde_json::Number::from_f64(2.5).unwrap());

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target.as_f64().unwrap(), 2.5);
        }

        #[test]
        fn test_merge_boolean_values() {
            let mut target = JsonValue::Bool(true);
            let source = JsonValue::Bool(false);

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target, JsonValue::Bool(false));
        }

        #[test]
        fn test_merge_null_source_overwrites() {
            let mut target = JsonValue::String("value".to_string());
            let source = JsonValue::Null;

            merge_json_values(&mut target, &source, false, None);

            assert!(target.is_null());
        }

        #[test]
        fn test_merge_into_null_target() {
            let mut target = JsonValue::Null;
            let source: JsonValue = serde_json::from_str(r#"{"key": "value"}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert!(target.is_object());
            assert_eq!(target["key"], JsonValue::String("value".to_string()));
        }

        #[test]
        fn test_merge_new_keys_added() {
            let mut target: JsonValue = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let source: JsonValue = serde_json::from_str(r#"{"b": 2, "c": 3}"#).unwrap();

            merge_json_values(&mut target, &source, false, None);

            assert_eq!(target["a"], JsonValue::Number(1.into()));
            assert_eq!(target["b"], JsonValue::Number(2.into()));
            assert_eq!(target["c"], JsonValue::Number(3.into()));
        }
    }

    mod file_io_helper_tests {
        use super::*;

        #[test]
        fn test_ensure_trailing_newline_adds_newline() {
            let content = "no newline".to_string();
            let result = ensure_trailing_newline(content);
            assert!(result.ends_with('\n'));
            assert_eq!(result, "no newline\n");
        }

        #[test]
        fn test_ensure_trailing_newline_preserves_existing() {
            let content = "has newline\n".to_string();
            let result = ensure_trailing_newline(content);
            assert_eq!(result, "has newline\n");
        }

        #[test]
        fn test_read_file_as_string_optional_returns_none() {
            let fs = MemoryFS::new();
            let result = read_file_as_string_optional(&fs, "nonexistent.json").unwrap();
            assert!(result.is_none());
        }

        #[test]
        fn test_read_file_as_string_optional_returns_content() {
            let mut fs = MemoryFS::new();
            fs.add_file("test.json", File::from_string(r#"{"key": "value"}"#))
                .unwrap();

            let result = read_file_as_string_optional(&fs, "test.json").unwrap();
            assert!(result.is_some());
            assert!(result.unwrap().contains("key"));
        }

        #[test]
        fn test_write_string_to_file_adds_newline() {
            let mut fs = MemoryFS::new();
            write_string_to_file(&mut fs, "test.txt", "content".to_string()).unwrap();

            let file = fs.get_file("test.txt").unwrap();
            let content = String::from_utf8(file.content.clone()).unwrap();
            assert!(content.ends_with('\n'));
        }
    }
}
