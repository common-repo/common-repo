//! INI file merge operations
//!
//! This module provides functionality for merging INI configuration files with support
//! for sections, key-value pairs, and various merge modes.
//!
//! ## Features
//!
//! - Section-aware merging of INI files
//! - Append mode: preserve existing keys when merging
//! - Replace mode: overwrite existing keys with new values
//! - Option to allow or prevent duplicate keys
//! - Root-level entry support (entries without section headers)
//!
//! ## Example
//!
//! ```ignore
//! use common_repo::merge::ini::apply_ini_merge_operation;
//! use common_repo::config::IniMergeOp;
//! use common_repo::filesystem::MemoryFS;
//!
//! let mut fs = MemoryFS::new();
//! // ... populate fs with source and dest files ...
//! let op = IniMergeOp { /* ... */ };
//! apply_ini_merge_operation(&mut fs, &op)?;
//! ```

use std::collections::HashSet;

use crate::config::IniMergeOp;
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Represents a key-value entry in an INI file
#[derive(Clone, Debug)]
struct IniEntry {
    key: String,
    value: String,
}

/// Represents a section in an INI file
///
/// A section contains a name (empty string for root-level entries) and
/// a list of key-value entries.
#[derive(Clone, Debug)]
struct IniSection {
    name: String,
    entries: Vec<IniEntry>,
}

/// Parse an INI file content into a list of sections
///
/// Supports:
/// - Section headers: `[section_name]`
/// - Key-value pairs: `key = value`
/// - Comments: lines starting with `#` or `;`
/// - Root-level entries (entries before any section header)
///
/// # Arguments
///
/// * `content` - The INI file content as a string
///
/// # Returns
///
/// A vector of `IniSection` containing all parsed sections
fn parse_ini(content: &str) -> Vec<IniSection> {
    let mut sections = Vec::new();
    let mut current = IniSection {
        name: String::new(),
        entries: Vec::new(),
    };
    let mut has_entries = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if !current.name.is_empty() || has_entries {
                sections.push(current);
            }
            current = IniSection {
                name: trimmed[1..trimmed.len() - 1].trim().to_string(),
                entries: Vec::new(),
            };
            has_entries = false;
        } else if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            current.entries.push(IniEntry { key, value });
            has_entries = true;
        }
    }

    if !current.name.is_empty() || has_entries {
        sections.push(current);
    }

    sections
}

/// Find a section by name in a list of sections
///
/// # Arguments
///
/// * `sections` - The list of sections to search
/// * `name` - The section name to find (empty string for root-level entries)
///
/// # Returns
///
/// An optional reference to the found section
fn find_ini_section<'a>(sections: &'a [IniSection], name: &str) -> Option<&'a IniSection> {
    sections.iter().find(|section| section.name == name)
}

/// Find a section by name in a list of sections, creating it if it doesn't exist
///
/// # Arguments
///
/// * `sections` - The list of sections to search and potentially modify
/// * `name` - The section name to find or create
///
/// # Returns
///
/// A mutable reference to the found or created section
fn find_ini_section_mut<'a>(sections: &'a mut Vec<IniSection>, name: &str) -> &'a mut IniSection {
    if let Some(pos) = sections.iter().position(|section| section.name == name) {
        &mut sections[pos]
    } else {
        sections.push(IniSection {
            name: name.to_string(),
            entries: Vec::new(),
        });
        sections.last_mut().unwrap()
    }
}

/// Serialize a list of sections back to INI file format
///
/// # Arguments
///
/// * `sections` - The list of sections to serialize
///
/// # Returns
///
/// The serialized INI content as a string
fn serialize_ini(sections: &[IniSection]) -> String {
    let mut output = String::new();

    for (index, section) in sections.iter().enumerate() {
        if !section.name.is_empty() {
            output.push('[');
            output.push_str(&section.name);
            output.push_str("]\n");
        }

        for entry in &section.entries {
            output.push_str(&entry.key);
            output.push('=');
            output.push_str(&entry.value);
            output.push('\n');
        }

        if index + 1 < sections.len() {
            output.push('\n');
        }
    }

    output
}

/// Apply an INI merge operation to the filesystem
///
/// Merges the source INI file into the destination file according to the
/// operation configuration. Supports section-specific merging, append mode,
/// and duplicate key handling.
///
/// # Arguments
///
/// * `fs` - The memory filesystem containing the files
/// * `op` - The merge operation configuration
///
/// # Returns
///
/// `Ok(())` on success, or an error if the operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The source file cannot be read
/// - The file content is not valid UTF-8
/// - The result cannot be written to the destination
pub fn apply_ini_merge_operation(fs: &mut MemoryFS, op: &IniMergeOp) -> Result<()> {
    let source_content = read_file_as_string(fs, &op.source)?;
    let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

    let source_sections = parse_ini(&source_content);
    let mut dest_sections = parse_ini(&dest_content);

    // Helper function to merge a source section into a destination section
    fn merge_section(
        source_section: &IniSection,
        dest_section: &mut IniSection,
        append: bool,
        allow_duplicates: bool,
    ) {
        if append {
            if allow_duplicates {
                dest_section.entries.extend(source_section.entries.clone());
            } else {
                for entry in &source_section.entries {
                    if !dest_section
                        .entries
                        .iter()
                        .any(|existing| existing.key.eq_ignore_ascii_case(&entry.key))
                    {
                        dest_section.entries.push(entry.clone());
                    }
                }
            }
        } else {
            if !allow_duplicates {
                let keys: HashSet<String> = source_section
                    .entries
                    .iter()
                    .map(|entry| entry.key.to_lowercase())
                    .collect();
                dest_section
                    .entries
                    .retain(|entry| !keys.contains(&entry.key.to_lowercase()));
            }

            dest_section.entries.extend(source_section.entries.clone());
        }
    }

    match &op.section {
        Some(section_name) => {
            // Merge into specific section
            let dest_section = find_ini_section_mut(&mut dest_sections, section_name);

            // If the source has the same section, merge it
            if let Some(source_section) = find_ini_section(&source_sections, section_name) {
                merge_section(source_section, dest_section, op.append, op.allow_duplicates);
            }

            // Also merge any root-level entries from source into the target section
            if let Some(root_section) = find_ini_section(&source_sections, "") {
                if !root_section.entries.is_empty() {
                    merge_section(root_section, dest_section, op.append, op.allow_duplicates);
                }
            }
        }
        None => {
            // Merge all sections from source into destination
            for source_section in &source_sections {
                let dest_section = find_ini_section_mut(&mut dest_sections, &source_section.name);
                merge_section(source_section, dest_section, op.append, op.allow_duplicates);
            }
        }
    }

    let serialized = serialize_ini(&dest_sections);
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

    mod parse_ini_tests {
        use super::*;

        #[test]
        fn test_parse_ini_empty() {
            let sections = parse_ini("");
            assert!(sections.is_empty());
        }

        #[test]
        fn test_parse_ini_single_section() {
            let content = r#"
[database]
host = localhost
port = 5432
"#;
            let sections = parse_ini(content);
            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].name, "database");
            assert_eq!(sections[0].entries.len(), 2);
            assert_eq!(sections[0].entries[0].key, "host");
            assert_eq!(sections[0].entries[0].value, "localhost");
            assert_eq!(sections[0].entries[1].key, "port");
            assert_eq!(sections[0].entries[1].value, "5432");
        }

        #[test]
        fn test_parse_ini_multiple_sections() {
            let content = r#"
[server]
host = localhost

[database]
port = 5432
"#;
            let sections = parse_ini(content);
            assert_eq!(sections.len(), 2);
            assert_eq!(sections[0].name, "server");
            assert_eq!(sections[1].name, "database");
        }

        #[test]
        fn test_parse_ini_root_level_entries() {
            let content = r#"
key = value
another = entry

[section]
foo = bar
"#;
            let sections = parse_ini(content);
            assert_eq!(sections.len(), 2);
            assert_eq!(sections[0].name, "");
            assert_eq!(sections[0].entries.len(), 2);
            assert_eq!(sections[1].name, "section");
        }

        #[test]
        fn test_parse_ini_comments() {
            let content = r#"
# This is a comment
[section]
; Another comment
key = value
"#;
            let sections = parse_ini(content);
            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].entries.len(), 1);
        }
    }

    mod serialize_ini_tests {
        use super::*;

        #[test]
        fn test_serialize_ini_empty() {
            let sections: Vec<IniSection> = Vec::new();
            let output = serialize_ini(&sections);
            assert!(output.is_empty());
        }

        #[test]
        fn test_serialize_ini_single_section() {
            let sections = vec![IniSection {
                name: "database".to_string(),
                entries: vec![
                    IniEntry {
                        key: "host".to_string(),
                        value: "localhost".to_string(),
                    },
                    IniEntry {
                        key: "port".to_string(),
                        value: "5432".to_string(),
                    },
                ],
            }];
            let output = serialize_ini(&sections);
            assert!(output.contains("[database]"));
            assert!(output.contains("host=localhost"));
            assert!(output.contains("port=5432"));
        }

        #[test]
        fn test_serialize_ini_root_entries() {
            let sections = vec![IniSection {
                name: "".to_string(),
                entries: vec![IniEntry {
                    key: "key".to_string(),
                    value: "value".to_string(),
                }],
            }];
            let output = serialize_ini(&sections);
            assert!(!output.contains("["));
            assert!(output.contains("key=value"));
        }
    }

    mod apply_ini_merge_operation_tests {
        use super::*;

        #[test]
        fn test_ini_merge_operation_basic() {
            // Test INI merge with section
            let mut fs = MemoryFS::new();

            // Create source INI fragment
            let source_ini = r#"
[database]
driver = postgresql
port = 5432
"#;
            fs.add_file_string("db.ini", source_ini).unwrap();

            // Create destination INI file
            let dest_ini = r#"
[server]
host = localhost
port = 8080
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "db.ini".to_string(),
                dest: "config.ini".to_string(),
                section: Some("database".to_string()),
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "config.ini").unwrap();

            // Should contain both sections
            assert!(result.contains("[server]"));
            assert!(result.contains("host=localhost"));
            assert!(result.contains("port=8080"));
            assert!(result.contains("[database]"));
            assert!(result.contains("driver=postgresql"));
            assert!(result.contains("port=5432"));
        }

        #[test]
        fn test_ini_merge_operation_append_mode() {
            // Test INI merge in append mode (should not overwrite existing keys)
            let mut fs = MemoryFS::new();

            // Create source INI fragment
            let source_ini = r#"
[settings]
timeout = 60
debug = true
"#;
            fs.add_file_string("new.ini", source_ini).unwrap();

            // Create destination INI file with overlapping key
            let dest_ini = r#"
[settings]
timeout = 30
host = localhost
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "new.ini".to_string(),
                dest: "config.ini".to_string(),
                section: Some("settings".to_string()),
                append: true, // append mode
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "config.ini").unwrap();

            // Should contain merged content
            assert!(result.contains("[settings]"));
            assert!(result.contains("host=localhost"));
            assert!(result.contains("debug=true"));
            // In append mode, existing keys should not be overwritten
            assert!(result.contains("timeout=30"));
        }

        #[test]
        fn test_ini_merge_operation_optional_section() {
            // Test INI merge without section (merge all sections)
            let mut fs = MemoryFS::new();

            // Create source INI fragment with multiple sections
            let source_ini = r#"
[database]
driver = postgresql
port = 5432

[cache]
enabled = true
ttl = 3600
"#;
            fs.add_file_string("multi.ini", source_ini).unwrap();

            // Create destination INI file with existing section
            let dest_ini = r#"
[server]
host = localhost
port = 8080
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "multi.ini".to_string(),
                dest: "config.ini".to_string(),
                section: None, // No specific section
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "config.ini").unwrap();

            // Should contain all sections
            assert!(result.contains("[server]"));
            assert!(result.contains("host=localhost"));
            assert!(result.contains("port=8080"));
            assert!(result.contains("[database]"));
            assert!(result.contains("driver=postgresql"));
            assert!(result.contains("port=5432"));
            assert!(result.contains("[cache]"));
            assert!(result.contains("enabled=true"));
            assert!(result.contains("ttl=3600"));
        }

        #[test]
        fn test_ini_merge_operation_root_level_into_section() {
            // Test merging root-level entries into a specific section
            let mut fs = MemoryFS::new();

            // Create source INI fragment with root-level entries and a section
            let source_ini = r#"
host = postgres.example.com
port = 5432
ssl_mode = require

[advanced]
pool_size = 20
"#;
            fs.add_file_string("db.ini", source_ini).unwrap();

            // Create destination INI file
            let dest_ini = r#"
[database]
driver = postgresql
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "db.ini".to_string(),
                dest: "config.ini".to_string(),
                section: Some("database".to_string()), // Merge into database section
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "config.ini").unwrap();

            // Should contain the database section with root-level entries merged in
            assert!(result.contains("[database]"));
            assert!(result.contains("driver=postgresql"));
            assert!(result.contains("host=postgres.example.com"));
            assert!(result.contains("port=5432"));
            assert!(result.contains("ssl_mode=require"));
            // pool_size should NOT be merged since it's in [advanced] section, not root level
        }

        #[test]
        fn test_ini_merge_operation_empty_source() {
            // Test INI merge with empty source file
            let mut fs = MemoryFS::new();

            // Create empty source INI fragment
            fs.add_file_string("empty.ini", "").unwrap();

            // Create destination INI file
            let dest_ini = r#"
[server]
host = localhost
port = 8080
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "empty.ini".to_string(),
                dest: "config.ini".to_string(),
                section: None,
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "config.ini").unwrap();

            // Should contain original content unchanged
            assert!(result.contains("[server]"));
            assert!(result.contains("host=localhost"));
            assert!(result.contains("port=8080"));
        }

        #[test]
        fn test_ini_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();

            let source_ini = r#"
[settings]
key = value
"#;
            fs.add_file_string("source.ini", source_ini).unwrap();

            let ini_op = IniMergeOp {
                source: "source.ini".to_string(),
                dest: "new_dest.ini".to_string(),
                section: None,
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.ini").unwrap();
            assert!(result.contains("[settings]"));
            assert!(result.contains("key=value"));
        }
    }
}
