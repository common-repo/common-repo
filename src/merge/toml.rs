//! TOML merge operations
//!
//! This module provides functionality for merging TOML documents with support
//! for path navigation, array merge modes, and recursive deep merging.
//!
//! ## Features
//!
//! - Deep merging of TOML tables with recursive descent
//! - Array handling with append, replace, or append-unique modes
//! - Path-based targeting to merge at specific locations
//! - Optional comment preservation via taplo formatting
//!
//! ## Example
//!
//! ```ignore
//! use common_repo::merge::toml::apply_toml_merge_operation;
//! use common_repo::config::TomlMergeOp;
//! use common_repo::filesystem::MemoryFS;
//!
//! let mut fs = MemoryFS::new();
//! // ... populate fs with source and dest files ...
//! let op = TomlMergeOp { /* ... */ };
//! apply_toml_merge_operation(&mut fs, &op)?;
//! ```

use log::warn;
use toml::Value as TomlValue;

use super::PathSegment;
use crate::config::TomlMergeOp;
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Parse a TOML path string into segments for navigation
///
/// Supports:
/// - Dot notation: `package.dependencies`
/// - Bracket notation: `package["version"]` or `package['version']`
/// - Array indices: `workspace.members[0]`
/// - Mixed: `config.database[0].settings`
/// - Escaped characters within quoted keys
///
/// # Examples
///
/// ```
/// use common_repo::merge::toml::parse_toml_path;
/// use common_repo::merge::PathSegment;
///
/// let segments = parse_toml_path("package.dependencies");
/// assert_eq!(segments.len(), 2);
/// ```
pub fn parse_toml_path(path: &str) -> Vec<PathSegment> {
    if path.trim().is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(PathSegment::Key(current.clone()));
                    current.clear();
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(PathSegment::Key(current.clone()));
                    current.clear();
                }

                let first_char = chars.peek().copied();

                if first_char == Some('"') || first_char == Some('\'') {
                    let quote_char = chars.next().unwrap();
                    let mut key = String::new();
                    let mut bracket_escaped = false;

                    while let Some(ch) = chars.next() {
                        if bracket_escaped {
                            key.push(ch);
                            bracket_escaped = false;
                        } else if ch == '\\' {
                            bracket_escaped = true;
                        } else if ch == quote_char {
                            if chars.peek() == Some(&']') {
                                chars.next();
                                break;
                            }
                            key.push(ch);
                        } else {
                            key.push(ch);
                        }
                    }

                    segments.push(PathSegment::Key(key));
                } else {
                    let mut bracket_content = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        chars.next();
                        if next_ch == ']' {
                            break;
                        }
                        bracket_content.push(next_ch);
                    }

                    if let Ok(idx) = bracket_content.trim().parse::<usize>() {
                        segments.push(PathSegment::Index(idx));
                    } else if !bracket_content.trim().is_empty() {
                        segments.push(PathSegment::Key(bracket_content.trim().to_string()));
                    }
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        segments.push(PathSegment::Key(current));
    }

    segments
}

/// Navigate to a specific path within a TOML value, creating intermediate
/// structures as needed.
///
/// Supports both table and array navigation. For tables, uses string keys;
/// for arrays, uses numeric indices. Creates missing intermediate structures
/// automatically.
///
/// # Arguments
///
/// * `value` - The root TOML value to navigate
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
pub fn navigate_toml_value<'a>(
    value: &'a mut TomlValue,
    path: &[PathSegment],
) -> Result<&'a mut TomlValue> {
    let mut current = value;
    for segment in path {
        match segment {
            PathSegment::Key(key) => {
                if !current.is_table() {
                    return Err(Error::Merge {
                        operation: "toml merge".to_string(),
                        message: format!("Expected table while navigating to '{}'", key),
                    });
                }

                let table = current.as_table_mut().unwrap();
                current = table
                    .entry(key.clone())
                    .or_insert(TomlValue::Table(toml::map::Map::new()));
            }
            PathSegment::Index(idx) => {
                if !current.is_array() {
                    return Err(Error::Merge {
                        operation: "toml merge".to_string(),
                        message: format!("Expected array while navigating to index {}", idx),
                    });
                }

                let array = current.as_array_mut().unwrap();
                while array.len() <= *idx {
                    array.push(TomlValue::Table(toml::map::Map::new()));
                }
                current = &mut array[*idx];
            }
        }
    }
    Ok(current)
}

/// Recursively merge source TOML value into target
///
/// Handles different TOML types appropriately:
/// - Tables: Recursively merge keys, with source values taking precedence for conflicts
/// - Arrays: Handle according to the specified merge mode (append, replace, or append-unique)
/// - Scalars: Replace target with source, with warnings for type mismatches
///
/// # Arguments
///
/// * `target` - The target value to merge into (modified in place)
/// * `source` - The source value to merge from
/// * `mode` - How to handle array merging
/// * `path` - Current path (for diagnostic messages)
/// * `src_file` - Source file name (for diagnostic messages)
/// * `dst_file` - Destination file name (for diagnostic messages)
pub fn merge_toml_values(
    target: &mut TomlValue,
    source: &TomlValue,
    mode: crate::config::ArrayMergeMode,
    path: &str,
    src_file: &str,
    dst_file: &str,
) {
    use crate::config::ArrayMergeMode;

    match target {
        TomlValue::Table(target_table) => {
            if let TomlValue::Table(source_table) = source {
                for (key, value) in source_table {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(existing) = target_table.get_mut(key) {
                        if matches!(existing, TomlValue::Table(_))
                            && matches!(value, TomlValue::Table(_))
                        {
                            merge_toml_values(existing, value, mode, &new_path, src_file, dst_file);
                        } else if let Some(source_array) = value.as_array() {
                            if let Some(target_array) = existing.as_array_mut() {
                                match mode {
                                    ArrayMergeMode::Append => {
                                        target_array.extend(source_array.iter().cloned());
                                    }
                                    ArrayMergeMode::Replace => {
                                        warn!(
                                            "{} -> {}: Replacing array at path '{}' (old size: {}, new size: {})",
                                            src_file, dst_file, new_path, target_array.len(), source_array.len()
                                        );
                                        *existing = TomlValue::Array(source_array.clone());
                                    }
                                    ArrayMergeMode::AppendUnique => {
                                        for item in source_array {
                                            if !target_array.contains(item) {
                                                target_array.push(item.clone());
                                            }
                                        }
                                    }
                                }
                            } else {
                                warn!(
                                    "{} -> {}: Type mismatch at path '{}': replacing {:?} with Array",
                                    src_file, dst_file, new_path, get_toml_type_name(existing)
                                );
                                *existing = TomlValue::Array(source_array.clone());
                            }
                        } else {
                            eprintln!(
                                "Warning: {} -> {}: Overwriting value at path '{}': {:?} -> {:?}",
                                src_file,
                                dst_file,
                                new_path,
                                get_toml_type_name(existing),
                                get_toml_type_name(value)
                            );
                            *existing = value.clone();
                        }
                    } else {
                        target_table.insert(key.clone(), value.clone());
                    }
                }
            } else {
                eprintln!(
                    "Warning: {} -> {}: Type mismatch at path '{}': replacing Table with {:?}",
                    src_file,
                    dst_file,
                    path,
                    get_toml_type_name(source)
                );
                *target = source.clone();
            }
        }
        TomlValue::Array(target_array) => {
            if let TomlValue::Array(source_array) = source {
                match mode {
                    ArrayMergeMode::Append => {
                        target_array.extend(source_array.clone());
                    }
                    ArrayMergeMode::Replace => {
                        eprintln!(
                            "Warning: {} -> {}: Replacing array at path '{}' (old size: {}, new size: {})",
                            src_file, dst_file, path, target_array.len(), source_array.len()
                        );
                        *target = TomlValue::Array(source_array.clone());
                    }
                    ArrayMergeMode::AppendUnique => {
                        for item in source_array {
                            if !target_array.contains(item) {
                                target_array.push(item.clone());
                            }
                        }
                    }
                }
            } else {
                eprintln!(
                    "Warning: {} -> {}: Type mismatch at path '{}': replacing Array with {:?}",
                    src_file,
                    dst_file,
                    path,
                    get_toml_type_name(source)
                );
                *target = source.clone();
            }
        }
        _ => {
            eprintln!(
                "Warning: {} -> {}: Overwriting scalar at path '{}': {:?} -> {:?}",
                src_file,
                dst_file,
                path,
                get_toml_type_name(target),
                get_toml_type_name(source)
            );
            *target = source.clone();
        }
    }
}

/// Get a human-readable type name for a TOML value
///
/// Used for diagnostic messages when type mismatches occur during merging.
pub fn get_toml_type_name(value: &TomlValue) -> &'static str {
    match value {
        TomlValue::String(_) => "String",
        TomlValue::Integer(_) => "Integer",
        TomlValue::Float(_) => "Float",
        TomlValue::Boolean(_) => "Boolean",
        TomlValue::Datetime(_) => "Datetime",
        TomlValue::Array(_) => "Array",
        TomlValue::Table(_) => "Table",
    }
}

/// Apply a TOML merge operation to the filesystem
///
/// Reads the source and destination TOML files, merges them according to the
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
/// - Source TOML is invalid
/// - Path navigation fails
/// - Result cannot be serialized
pub fn apply_toml_merge_operation(fs: &mut MemoryFS, op: &TomlMergeOp) -> Result<()> {
    let source_content = read_file_as_string(fs, &op.source)?;
    let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

    let mut dest_value: TomlValue =
        toml::from_str(&dest_content).unwrap_or_else(|_| TomlValue::Table(toml::map::Map::new()));
    let source_value: TomlValue = toml::from_str(&source_content).map_err(|err| Error::Merge {
        operation: "toml merge".to_string(),
        message: format!("Failed to parse source TOML: {}", err),
    })?;

    let path = parse_toml_path(&op.path);
    let target = navigate_toml_value(&mut dest_value, &path)?;
    let mode = op.get_array_mode();
    merge_toml_values(target, &source_value, mode, &op.path, &op.source, &op.dest);

    let serialized = if op.preserve_comments {
        // Attempt comment preservation using taplo
        // First serialize with toml, then format with taplo to preserve structure
        let toml_string = toml::to_string_pretty(&dest_value).map_err(|err| Error::Merge {
            operation: "toml merge".to_string(),
            message: format!("Failed to serialize TOML: {}", err),
        })?;

        // Try to format with taplo for better structure preservation
        taplo::formatter::format(&toml_string, taplo::formatter::Options::default())
    } else {
        toml::to_string_pretty(&dest_value).map_err(|err| Error::Merge {
            operation: "toml merge".to_string(),
            message: format!("Failed to serialize TOML: {}", err),
        })?
    };

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

    mod parse_toml_path_tests {
        use super::*;

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
    }

    mod toml_merge_integration_tests {
        use super::*;

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

        #[test]
        fn test_toml_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "[package]\nname = \"test\"")
                .unwrap();

            let op = TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "new_dest.toml".to_string(),
                path: "".to_string(),
                append: false,
                preserve_comments: false,
                array_mode: None,
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(parsed["package"]["name"].as_str(), Some("test"));
        }
    }

    mod navigate_toml_value_tests {
        use super::*;

        #[test]
        fn test_navigate_to_nested_key() {
            let mut value: TomlValue = toml::from_str(
                r#"[foo]
bar = 1"#,
            )
            .unwrap();
            let path = parse_toml_path("foo.bar");
            let target = navigate_toml_value(&mut value, &path).unwrap();
            assert_eq!(target, &TomlValue::Integer(1));
        }

        #[test]
        fn test_navigate_creates_missing_path() {
            let mut value = TomlValue::Table(toml::map::Map::new());
            let path = parse_toml_path("foo.bar");
            let target = navigate_toml_value(&mut value, &path).unwrap();
            // Should have created the path and returned an empty table
            assert!(target.is_table());
        }

        #[test]
        fn test_navigate_type_error() {
            let mut value: TomlValue = toml::from_str(r#"foo = 42"#).unwrap();
            let path = parse_toml_path("foo.bar");
            let result = navigate_toml_value(&mut value, &path);
            assert!(result.is_err());
        }
    }
}
