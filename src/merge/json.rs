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
//! use common_repo::merge::json::apply_json_merge_operation;
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
    let source_content = read_file_as_string(fs, &op.source)?;
    let dest_content =
        read_file_as_string_optional(fs, &op.dest)?.unwrap_or_else(|| "{}".to_string());

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
                source: "source.json".to_string(),
                dest: "dest.json".to_string(),
                path: None,
                append: false,
                position: None,
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
                source: "source.json".to_string(),
                dest: "dest.json".to_string(),
                path: Some("database.connection".to_string()),
                append: false,
                position: None,
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
                source: "source.json".to_string(),
                dest: "new_dest.json".to_string(),
                path: None,
                append: false,
                position: None,
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
                source: "source.json".to_string(),
                dest: "dest.json".to_string(),
                path: None,
                append: true,
                position: None,
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
                source: "source.json".to_string(),
                dest: "dest.json".to_string(),
                path: None,
                append: false,
                position: None,
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
                source: "source.json".to_string(),
                dest: "dest.json".to_string(),
                path: None,
                append: false,
                position: None,
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
}
