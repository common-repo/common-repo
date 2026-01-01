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
//! use common_repo::merge::apply_toml_merge_operation;
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
/// use common_repo::merge::parse_toml_path;
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
    op.validate()?;
    let source_path = op.get_source().expect("source validated");
    let dest_path = op.get_dest().expect("dest validated");

    let source_content = read_file_as_string(fs, source_path)?;
    let dest_content = read_file_as_string_optional(fs, dest_path)?.unwrap_or_default();

    let mut dest_value: TomlValue =
        toml::from_str(&dest_content).unwrap_or_else(|_| TomlValue::Table(toml::map::Map::new()));
    let source_value: TomlValue = toml::from_str(&source_content).map_err(|err| Error::Merge {
        operation: "toml merge".to_string(),
        message: format!("Failed to parse source TOML: {}", err),
    })?;

    let path_str = op.path.as_deref().unwrap_or("");
    let path = parse_toml_path(path_str);
    let target = navigate_toml_value(&mut dest_value, &path)?;
    let mode = op.get_array_mode();
    merge_toml_values(
        target,
        &source_value,
        mode,
        path_str,
        source_path,
        dest_path,
    );

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
                source: Some("source.toml".to_string()),
                dest: Some("Cargo.toml".to_string()),
                path: None, // root level
                ..Default::default()
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
                source: Some("config.toml".to_string()),
                dest: Some("merged.toml".to_string()),
                path: Some("server".to_string()),
                ..Default::default()
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
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(crate::config::ArrayMergeMode::Replace),
                ..Default::default()
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
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(crate::config::ArrayMergeMode::Append),
                ..Default::default()
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
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(crate::config::ArrayMergeMode::AppendUnique),
                ..Default::default()
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
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                append: true,
                ..Default::default()
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
                source: Some("source.toml".to_string()),
                dest: Some("new_dest.toml".to_string()),
                path: None,
                ..Default::default()
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

    mod deep_section_merging_tests {
        use super::*;

        #[test]
        fn test_merge_deeply_nested_tables() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[level1.level2.level3]
deep_key = "deep_value"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[level1.level2]
existing = "kept"
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(
                parsed["level1"]["level2"]["existing"].as_str(),
                Some("kept")
            );
            assert_eq!(
                parsed["level1"]["level2"]["level3"]["deep_key"].as_str(),
                Some("deep_value")
            );
        }

        #[test]
        fn test_merge_multiple_sections_simultaneously() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[database]
host = "localhost"
port = 5432

[logging]
level = "debug"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[server]
name = "myserver"
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(parsed["server"]["name"].as_str(), Some("myserver"));
            assert_eq!(parsed["database"]["host"].as_str(), Some("localhost"));
            assert_eq!(parsed["database"]["port"].as_integer(), Some(5432));
            assert_eq!(parsed["logging"]["level"].as_str(), Some("debug"));
        }

        #[test]
        fn test_merge_at_deeply_nested_path() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
option1 = true
option2 = "value"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[a.b.c]
existing = 1
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: Some("a.b.c".to_string()),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(parsed["a"]["b"]["c"]["existing"].as_integer(), Some(1));
            assert_eq!(parsed["a"]["b"]["c"]["option1"].as_bool(), Some(true));
            assert_eq!(parsed["a"]["b"]["c"]["option2"].as_str(), Some("value"));
        }

        #[test]
        fn test_merge_creates_intermediate_tables() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
new_value = 42
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();
            fs.add_file_string("dest.toml", "").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: Some("deeply.nested.path".to_string()),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(
                parsed["deeply"]["nested"]["path"]["new_value"].as_integer(),
                Some(42)
            );
        }

        #[test]
        fn test_merge_preserves_sibling_sections() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
new_field = "added"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[section1]
a = 1

[section2]
b = 2

[section3]
c = 3
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: Some("section2".to_string()),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            assert_eq!(parsed["section1"]["a"].as_integer(), Some(1));
            assert_eq!(parsed["section2"]["b"].as_integer(), Some(2));
            assert_eq!(parsed["section2"]["new_field"].as_str(), Some("added"));
            assert_eq!(parsed["section3"]["c"].as_integer(), Some(3));
        }
    }

    mod array_edge_case_tests {
        use super::*;
        use crate::config::ArrayMergeMode;

        #[test]
        fn test_merge_empty_source_array() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[package]
items = []
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = ["a", "b", "c"]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let items = parsed["package"]["items"].as_array().unwrap();
            // Empty append should keep original
            assert_eq!(items.len(), 3);
        }

        #[test]
        fn test_merge_empty_destination_array() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[package]
items = ["x", "y"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = []
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let items = parsed["package"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("x"));
        }

        #[test]
        fn test_merge_array_of_tables() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[[dependencies]]
name = "serde"
version = "1.0"

[[dependencies]]
name = "toml"
version = "0.8"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[[dependencies]]
name = "clap"
version = "4.0"
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let deps = parsed["dependencies"].as_array().unwrap();
            assert_eq!(deps.len(), 3);
        }

        #[test]
        fn test_merge_array_with_mixed_types() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[data]
items = [1, "string", 3.14, true]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[data]
items = [false, 42]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let items = parsed["data"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 6);
        }

        #[test]
        fn test_merge_arrays_with_nested_tables() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[[servers]]
[servers.config]
port = 8080
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[[servers]]
[servers.config]
host = "localhost"
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let servers = parsed["servers"].as_array().unwrap();
            assert_eq!(servers.len(), 2);
        }

        #[test]
        fn test_append_unique_with_duplicate_tables() {
            let mut fs = MemoryFS::new();

            // Tables are compared by structural equality
            let source_toml = r#"
items = [{name = "a"}, {name = "b"}]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
items = [{name = "a"}, {name = "c"}]
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::AppendUnique),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            let items = parsed["items"].as_array().unwrap();
            // {name = "a"} is duplicate, so only {name = "b"} should be added
            assert_eq!(items.len(), 3);
        }
    }

    mod type_overwrite_tests {
        use super::*;
        use crate::config::ArrayMergeMode;

        #[test]
        fn test_overwrite_scalar_with_table() {
            let mut target = TomlValue::Integer(42);
            let source = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("key".to_string(), TomlValue::String("value".to_string()));
                map
            });

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "test",
                "src",
                "dst",
            );

            assert!(target.is_table());
            assert_eq!(target["key"].as_str(), Some("value"));
        }

        #[test]
        fn test_overwrite_array_with_table() {
            let mut target = TomlValue::Array(vec![TomlValue::Integer(1)]);
            let source = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("key".to_string(), TomlValue::Integer(42));
                map
            });

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "test",
                "src",
                "dst",
            );

            assert!(target.is_table());
        }

        #[test]
        fn test_overwrite_table_with_scalar() {
            let mut target = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("key".to_string(), TomlValue::Integer(1));
                map
            });
            let source = TomlValue::String("replaced".to_string());

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "test",
                "src",
                "dst",
            );

            assert!(target.is_str());
            assert_eq!(target.as_str(), Some("replaced"));
        }

        #[test]
        fn test_overwrite_table_with_array() {
            let mut target = TomlValue::Table(toml::map::Map::new());
            let source = TomlValue::Array(vec![TomlValue::Integer(1), TomlValue::Integer(2)]);

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "test",
                "src",
                "dst",
            );

            assert!(target.is_array());
            assert_eq!(target.as_array().unwrap().len(), 2);
        }

        #[test]
        fn test_type_mismatch_non_array_to_array_in_table() {
            let mut fs = MemoryFS::new();

            let source_toml = r#"
[package]
items = ["new1", "new2"]
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            let dest_toml = r#"
[package]
items = 42
"#;
            fs.add_file_string("dest.toml", dest_toml).unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                array_mode: Some(ArrayMergeMode::Append),
                ..Default::default()
            };

            apply_toml_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = result.parse().unwrap();
            // Array should replace the integer
            assert!(parsed["package"]["items"].is_array());
        }

        #[test]
        fn test_all_toml_type_names() {
            assert_eq!(
                get_toml_type_name(&TomlValue::String("".to_string())),
                "String"
            );
            assert_eq!(get_toml_type_name(&TomlValue::Integer(0)), "Integer");
            assert_eq!(get_toml_type_name(&TomlValue::Float(0.0)), "Float");
            assert_eq!(get_toml_type_name(&TomlValue::Boolean(true)), "Boolean");
            assert_eq!(get_toml_type_name(&TomlValue::Array(vec![])), "Array");
            assert_eq!(
                get_toml_type_name(&TomlValue::Table(toml::map::Map::new())),
                "Table"
            );

            // Test datetime - need to parse a valid datetime
            let dt_value: TomlValue = toml::from_str("dt = 1979-05-27T07:32:00Z")
                .map(|v: TomlValue| v["dt"].clone())
                .unwrap();
            assert_eq!(get_toml_type_name(&dt_value), "Datetime");
        }
    }

    mod error_path_tests {
        use super::*;

        #[test]
        fn test_source_file_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("dest.toml", "[package]").unwrap();

            let op = TomlMergeOp {
                source: Some("nonexistent.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, Error::Merge { .. }));
            if let Error::Merge { message, .. } = err {
                assert!(message.contains("not found"));
            }
        }

        #[test]
        fn test_invalid_source_toml() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "invalid = [unclosed")
                .unwrap();
            fs.add_file_string("dest.toml", "[package]").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            if let Err(Error::Merge { message, .. }) = result {
                assert!(message.contains("parse"));
            }
        }

        #[test]
        fn test_navigate_to_index_in_non_array() {
            let mut value = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("key".to_string(), TomlValue::String("value".to_string()));
                map
            });

            let path = vec![PathSegment::Key("key".to_string()), PathSegment::Index(0)];

            let result = navigate_toml_value(&mut value, &path);
            assert!(result.is_err());
            if let Err(Error::Merge { message, .. }) = result {
                assert!(message.contains("array"));
            }
        }

        #[test]
        fn test_navigate_to_key_in_scalar() {
            let mut value = TomlValue::Integer(42);
            let path = vec![PathSegment::Key("key".to_string())];

            let result = navigate_toml_value(&mut value, &path);
            assert!(result.is_err());
            if let Err(Error::Merge { message, .. }) = result {
                assert!(message.contains("table"));
            }
        }

        #[test]
        fn test_source_file_invalid_utf8() {
            let mut fs = MemoryFS::new();
            // Add a file with invalid UTF-8 bytes
            let _ = fs.add_file("source.toml", File::new(vec![0xFF, 0xFE, 0x00, 0x01]));
            fs.add_file_string("dest.toml", "[package]").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            if let Err(Error::Merge { message, .. }) = result {
                assert!(message.contains("UTF-8"));
            }
        }

        #[test]
        fn test_destination_file_invalid_utf8() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "[package]\nname = \"test\"")
                .unwrap();
            let _ = fs.add_file("dest.toml", File::new(vec![0xFF, 0xFE, 0x00, 0x01]));

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            if let Err(Error::Merge { message, .. }) = result {
                assert!(message.contains("UTF-8"));
            }
        }

        #[test]
        fn test_navigate_creates_array_elements() {
            let mut value = TomlValue::Array(vec![TomlValue::Integer(1)]);

            // Navigate to index 5, which should create elements
            let path = vec![PathSegment::Index(5)];
            let result = navigate_toml_value(&mut value, &path);

            assert!(result.is_ok());
            let arr = value.as_array().unwrap();
            assert_eq!(arr.len(), 6); // Original 1 + 5 new empty tables
        }

        #[test]
        fn test_invalid_destination_toml_gets_replaced() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "[package]\nname = \"test\"")
                .unwrap();
            // Invalid TOML that will fail to parse - gets replaced with empty table
            fs.add_file_string("dest.toml", "invalid = [").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            // This should succeed because invalid dest is replaced with empty table
            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: TomlValue = content.parse().unwrap();
            assert_eq!(parsed["package"]["name"].as_str(), Some("test"));
        }
    }

    mod merge_values_direct_tests {
        use super::*;
        use crate::config::ArrayMergeMode;

        #[test]
        fn test_merge_with_empty_path() {
            let mut target = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("existing".to_string(), TomlValue::Integer(1));
                map
            });
            let source = TomlValue::Table({
                let mut map = toml::map::Map::new();
                map.insert("new".to_string(), TomlValue::Integer(2));
                map
            });

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "",
                "src",
                "dst",
            );

            assert_eq!(target["existing"].as_integer(), Some(1));
            assert_eq!(target["new"].as_integer(), Some(2));
        }

        #[test]
        fn test_merge_nested_tables_with_path_tracking() {
            let mut target = TomlValue::Table({
                let mut map = toml::map::Map::new();
                let mut inner = toml::map::Map::new();
                inner.insert("a".to_string(), TomlValue::Integer(1));
                map.insert("nested".to_string(), TomlValue::Table(inner));
                map
            });
            let source = TomlValue::Table({
                let mut map = toml::map::Map::new();
                let mut inner = toml::map::Map::new();
                inner.insert("b".to_string(), TomlValue::Integer(2));
                map.insert("nested".to_string(), TomlValue::Table(inner));
                map
            });

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "root",
                "src",
                "dst",
            );

            assert_eq!(target["nested"]["a"].as_integer(), Some(1));
            assert_eq!(target["nested"]["b"].as_integer(), Some(2));
        }

        #[test]
        fn test_top_level_array_append() {
            let mut target = TomlValue::Array(vec![TomlValue::Integer(1), TomlValue::Integer(2)]);
            let source = TomlValue::Array(vec![TomlValue::Integer(3)]);

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Append,
                "arr",
                "src",
                "dst",
            );

            let arr = target.as_array().unwrap();
            assert_eq!(arr.len(), 3);
        }

        #[test]
        fn test_top_level_array_replace() {
            let mut target = TomlValue::Array(vec![TomlValue::Integer(1), TomlValue::Integer(2)]);
            let source = TomlValue::Array(vec![TomlValue::Integer(99)]);

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "arr",
                "src",
                "dst",
            );

            let arr = target.as_array().unwrap();
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].as_integer(), Some(99));
        }

        #[test]
        fn test_top_level_array_append_unique() {
            let mut target = TomlValue::Array(vec![
                TomlValue::Integer(1),
                TomlValue::Integer(2),
                TomlValue::Integer(3),
            ]);
            let source = TomlValue::Array(vec![
                TomlValue::Integer(2),
                TomlValue::Integer(4),
                TomlValue::Integer(1),
            ]);

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::AppendUnique,
                "arr",
                "src",
                "dst",
            );

            let arr = target.as_array().unwrap();
            // Original [1,2,3] + unique [4] = [1,2,3,4]
            assert_eq!(arr.len(), 4);
        }

        #[test]
        fn test_scalar_overwrite() {
            let mut target = TomlValue::String("old".to_string());
            let source = TomlValue::String("new".to_string());

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "path",
                "src",
                "dst",
            );

            assert_eq!(target.as_str(), Some("new"));
        }

        #[test]
        fn test_scalar_type_change() {
            let mut target = TomlValue::String("text".to_string());
            let source = TomlValue::Integer(42);

            merge_toml_values(
                &mut target,
                &source,
                ArrayMergeMode::Replace,
                "path",
                "src",
                "dst",
            );

            assert_eq!(target.as_integer(), Some(42));
        }
    }

    mod helper_function_tests {
        use super::*;

        #[test]
        fn test_ensure_trailing_newline_without() {
            let content = "no newline".to_string();
            let result = ensure_trailing_newline(content);
            assert!(result.ends_with('\n'));
            assert_eq!(result, "no newline\n");
        }

        #[test]
        fn test_ensure_trailing_newline_with() {
            let content = "has newline\n".to_string();
            let result = ensure_trailing_newline(content);
            assert!(result.ends_with('\n'));
            assert_eq!(result, "has newline\n");
            // Should not add extra newline
            assert!(!result.ends_with("\n\n"));
        }

        #[test]
        fn test_read_file_as_string_success() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "hello world").unwrap();

            let result = read_file_as_string(&fs, "test.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "hello world");
        }

        #[test]
        fn test_read_file_as_string_not_found() {
            let fs = MemoryFS::new();
            let result = read_file_as_string(&fs, "missing.txt");
            assert!(result.is_err());
        }

        #[test]
        fn test_read_file_as_string_optional_exists() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "content").unwrap();

            let result = read_file_as_string_optional(&fs, "test.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("content".to_string()));
        }

        #[test]
        fn test_read_file_as_string_optional_missing() {
            let fs = MemoryFS::new();
            let result = read_file_as_string_optional(&fs, "missing.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_read_file_as_string_optional_invalid_utf8() {
            let mut fs = MemoryFS::new();
            let _ = fs.add_file("bad.txt", File::new(vec![0xFF, 0xFE]));

            let result = read_file_as_string_optional(&fs, "bad.txt");
            assert!(result.is_err());
        }

        #[test]
        fn test_write_string_to_file() {
            let mut fs = MemoryFS::new();
            let result = write_string_to_file(&mut fs, "output.txt", "content".to_string());
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "output.txt").unwrap();
            assert!(content.ends_with('\n'));
        }
    }

    mod parse_path_edge_cases {
        use super::*;

        #[test]
        fn test_parse_path_with_leading_dot() {
            // Leading dot should result in empty first segment being skipped
            let segments = parse_toml_path(".foo.bar");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "foo"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_with_trailing_dot() {
            let segments = parse_toml_path("foo.bar.");
            assert_eq!(segments.len(), 2);
        }

        #[test]
        fn test_parse_path_with_consecutive_dots() {
            let segments = parse_toml_path("foo..bar");
            // Empty segment between dots is skipped
            assert_eq!(segments.len(), 2);
        }

        #[test]
        fn test_parse_path_single_key() {
            let segments = parse_toml_path("single");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "single"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_only_index() {
            let segments = parse_toml_path("[0]");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
        }

        #[test]
        fn test_parse_path_bracket_key_without_quotes() {
            let segments = parse_toml_path("foo[bar]");
            assert_eq!(segments.len(), 2);
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "bar"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_single_quotes() {
            let segments = parse_toml_path("config['key']");
            assert_eq!(segments.len(), 2);
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "key"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_multiple_indices() {
            let segments = parse_toml_path("arr[0][1][2]");
            assert_eq!(segments.len(), 4);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "arr"),
                _ => panic!("Expected Key segment"),
            }
            for (i, expected) in [(1, 0), (2, 1), (3, 2)] {
                match &segments[i] {
                    PathSegment::Index(idx) => assert_eq!(*idx, expected),
                    _ => panic!("Expected Index segment"),
                }
            }
        }
    }

    mod preserve_comments_tests {
        use super::*;

        #[test]
        fn test_preserve_comments_mode_formats_output() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "[package]\nname = \"test\"")
                .unwrap();
            fs.add_file_string("dest.toml", "").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                preserve_comments: true,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "dest.toml").unwrap();
            // taplo formatter should produce valid TOML
            let parsed: std::result::Result<TomlValue, _> = content.parse();
            assert!(parsed.is_ok());
        }

        #[test]
        fn test_no_preserve_comments_mode() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.toml", "[package]\nname = \"test\"")
                .unwrap();
            fs.add_file_string("dest.toml", "").unwrap();

            let op = TomlMergeOp {
                source: Some("source.toml".to_string()),
                dest: Some("dest.toml".to_string()),
                path: None,
                ..Default::default()
            };

            let result = apply_toml_merge_operation(&mut fs, &op);
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "dest.toml").unwrap();
            let parsed: std::result::Result<TomlValue, _> = content.parse();
            assert!(parsed.is_ok());
        }
    }
}
