//! Markdown merge operations
//!
//! This module provides functionality for merging content into Markdown documents
//! by targeting specific sections identified by their headings.
//!
//! ## Features
//!
//! - Section-based targeting using heading levels (h1-h6)
//! - Append or replace content within sections
//! - Create new sections if they don't exist
//! - Position control for new sections (start or end of document)
//! - Preserves document structure and formatting
//!
//! ## Example
//!
//! ```ignore
//! use common_repo::merge::markdown::apply_markdown_merge_operation;
//! use common_repo::config::MarkdownMergeOp;
//! use common_repo::filesystem::MemoryFS;
//!
//! let mut fs = MemoryFS::new();
//! // ... populate fs with source and dest files ...
//! let op = MarkdownMergeOp { /* ... */ };
//! apply_markdown_merge_operation(&mut fs, &op)?;
//! ```

use crate::config::MarkdownMergeOp;
use crate::error::{Error, Result};
use crate::filesystem::{File, MemoryFS};

/// Split content into lines while preserving trailing newline information
///
/// If the content ends with a newline, an empty string is appended to the
/// result to preserve this when rejoining with newlines.
///
/// # Arguments
///
/// * `content` - The content to split into lines
///
/// # Returns
///
/// A vector of lines, with an empty string appended if content ends with newline
fn split_lines_preserve(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
    if content.ends_with('\n') {
        lines.push(String::new());
    }
    lines
}

/// Generate a markdown heading for a given level and section name
///
/// Creates a heading string with the appropriate number of '#' characters
/// followed by the section name. Level is clamped to 1-6.
///
/// # Arguments
///
/// * `level` - The heading level (1-6)
/// * `section` - The section name
///
/// # Returns
///
/// A heading string like "## Section Name"
fn heading_for(level: u8, section: &str) -> String {
    let level = level.clamp(1, 6);
    format!("{} {}", "#".repeat(level as usize), section.trim())
}

/// Find the start and end indices of a section in a markdown document
///
/// Searches for a heading matching the given level and section name, then
/// finds the end of that section (either the next heading of equal or lesser
/// level, or the end of the document).
///
/// # Arguments
///
/// * `lines` - The document lines to search
/// * `level` - The heading level to match
/// * `section` - The section name to find
///
/// # Returns
///
/// `Some((start, end))` if found, where `start` is the heading line index
/// and `end` is one past the last line of the section. `None` if not found.
fn find_section_bounds(lines: &[String], level: u8, section: &str) -> Option<(usize, usize)> {
    let heading = heading_for(level, section);
    let mut start_index = None;

    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == heading {
            start_index = Some(idx);
            break;
        }
    }

    let start = start_index?;
    let mut end = lines.len();

    for (idx, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let next_level = trimmed.chars().take_while(|c| *c == '#').count() as u8;
            if next_level <= level {
                end = idx;
                break;
            }
        }
    }

    Some((start, end))
}

/// Normalize position string to a canonical form
///
/// Accepts "start" (case-insensitive) to insert at the beginning,
/// any other value defaults to "end".
///
/// # Arguments
///
/// * `position` - The position string to normalize
///
/// # Returns
///
/// Either "start" or "end"
fn normalize_position(position: &str) -> &str {
    match position.to_lowercase().as_str() {
        "start" => "start",
        _ => "end",
    }
}

/// Apply a markdown merge operation to the filesystem
///
/// Reads the source content and merges it into the destination markdown file
/// under the specified section. If the section doesn't exist and `create_section`
/// is true, a new section is created at the specified position.
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
/// - Section not found and `create_section` is false
/// - Result cannot be written
pub fn apply_markdown_merge_operation(fs: &mut MemoryFS, op: &MarkdownMergeOp) -> Result<()> {
    let source_content = read_file_as_string(fs, &op.source)?;
    let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

    let mut dest_lines = split_lines_preserve(&dest_content);
    let source_lines = split_lines_preserve(&source_content);
    let position = normalize_position(&op.position);

    if let Some((start, end)) = find_section_bounds(&dest_lines, op.level, &op.section) {
        let insert_index = if op.append { end } else { start + 1 };

        if op.append {
            let mut payload = source_lines.clone();
            if payload.is_empty() || !payload.last().map(|line| line.is_empty()).unwrap_or(false) {
                payload.push(String::new());
            }
            if insert_index > start + 1 && !dest_lines[insert_index - 1].trim().is_empty() {
                payload.insert(0, String::new());
            }
            dest_lines.splice(insert_index..insert_index, payload);
        } else {
            let mut payload = source_lines.clone();
            if payload.is_empty() || !payload.last().map(|line| line.is_empty()).unwrap_or(false) {
                payload.push(String::new());
            }
            dest_lines.splice(start + 1..end, payload);
        }
    } else {
        if !op.create_section {
            return Err(Error::Merge {
                operation: "markdown merge".to_string(),
                message: format!(
                    "Section '{}' not found and create-section is false",
                    op.section
                ),
            });
        }

        let mut block = Vec::new();
        block.push(heading_for(op.level, &op.section));
        block.push(String::new());
        block.extend(source_lines.clone());
        if block.is_empty() || !block.last().map(|line| line.is_empty()).unwrap_or(false) {
            block.push(String::new());
        }

        match position {
            "start" => {
                while !dest_lines.is_empty() && dest_lines[0].trim().is_empty() {
                    dest_lines.remove(0);
                }
                dest_lines.splice(0..0, block);
            }
            _ => {
                if !dest_lines.is_empty() && !dest_lines.last().unwrap().trim().is_empty() {
                    dest_lines.push(String::new());
                }
                dest_lines.extend(block);
            }
        }
    }

    let mut serialized = dest_lines.join("\n");
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }

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

    mod helper_function_tests {
        use super::*;

        #[test]
        fn test_split_lines_preserve_with_trailing_newline() {
            let lines = split_lines_preserve("line1\nline2\n");
            assert_eq!(lines, vec!["line1", "line2", ""]);
        }

        #[test]
        fn test_split_lines_preserve_without_trailing_newline() {
            let lines = split_lines_preserve("line1\nline2");
            assert_eq!(lines, vec!["line1", "line2"]);
        }

        #[test]
        fn test_heading_for_levels() {
            assert_eq!(heading_for(1, "Title"), "# Title");
            assert_eq!(heading_for(2, "Section"), "## Section");
            assert_eq!(heading_for(6, "Deep"), "###### Deep");
        }

        #[test]
        fn test_heading_for_clamps_level() {
            assert_eq!(heading_for(0, "Zero"), "# Zero");
            assert_eq!(heading_for(10, "Ten"), "###### Ten");
        }

        #[test]
        fn test_heading_for_trims_section() {
            assert_eq!(heading_for(2, "  Spaced  "), "## Spaced");
        }

        #[test]
        fn test_find_section_bounds_found() {
            let lines: Vec<String> = vec![
                "# Title".to_string(),
                "Intro".to_string(),
                "## Section".to_string(),
                "Content".to_string(),
                "More content".to_string(),
                "## Next Section".to_string(),
                "Other".to_string(),
            ];
            let bounds = find_section_bounds(&lines, 2, "Section");
            assert_eq!(bounds, Some((2, 5)));
        }

        #[test]
        fn test_find_section_bounds_at_end() {
            let lines: Vec<String> = vec![
                "# Title".to_string(),
                "## Section".to_string(),
                "Content".to_string(),
            ];
            let bounds = find_section_bounds(&lines, 2, "Section");
            assert_eq!(bounds, Some((1, 3)));
        }

        #[test]
        fn test_find_section_bounds_not_found() {
            let lines: Vec<String> = vec!["# Title".to_string(), "Content".to_string()];
            let bounds = find_section_bounds(&lines, 2, "Missing");
            assert_eq!(bounds, None);
        }

        #[test]
        fn test_normalize_position() {
            assert_eq!(normalize_position("start"), "start");
            assert_eq!(normalize_position("START"), "start");
            assert_eq!(normalize_position("Start"), "start");
            assert_eq!(normalize_position("end"), "end");
            assert_eq!(normalize_position("anything"), "end");
            assert_eq!(normalize_position(""), "end");
        }
    }

    mod markdown_merge_integration_tests {
        use super::*;

        #[test]
        fn test_markdown_merge_replace_section() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New content here"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("# Doc\n\n## Section\n\nOld content\n\n## Other\n\nMore\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "dest.md".to_string(),
                section: "Section".to_string(),
                append: false,
                level: 2,
                position: String::new(),
                create_section: false,
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## Section"));
            assert!(result.contains("New content here"));
            assert!(!result.contains("Old content"));
            assert!(result.contains("## Other"));
        }

        #[test]
        fn test_markdown_merge_append_to_section() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Appended content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Section\n\nExisting content\n\n## Other\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "dest.md".to_string(),
                section: "Section".to_string(),
                append: true,
                level: 2,
                position: String::new(),
                create_section: false,
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("Existing content"));
            assert!(result.contains("Appended content"));
        }

        #[test]
        fn test_markdown_merge_create_section_at_end() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New section content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n\nIntro\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "dest.md".to_string(),
                section: "New Section".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true,
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("# Title"));
            assert!(result.contains("## New Section"));
            assert!(result.contains("New section content"));
        }

        #[test]
        fn test_markdown_merge_create_section_at_start() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("First content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Original Title\n\nBody\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "dest.md".to_string(),
                section: "First Section".to_string(),
                append: false,
                level: 2,
                position: "start".to_string(),
                create_section: true,
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // New section should be at the start
            let first_section_pos = result.find("## First Section").unwrap();
            let original_title_pos = result.find("# Original Title").unwrap();
            assert!(first_section_pos < original_title_pos);
        }

        #[test]
        fn test_markdown_merge_section_not_found_error() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "dest.md".to_string(),
                section: "Missing".to_string(),
                append: false,
                level: 2,
                position: String::new(),
                create_section: false,
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err_msg = format!("{:?}", result.unwrap_err());
            assert!(err_msg.contains("not found"));
        }

        #[test]
        fn test_markdown_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content for new doc"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: "source.md".to_string(),
                dest: "new_dest.md".to_string(),
                section: "Section".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true,
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.md").unwrap();
            assert!(result.contains("## Section"));
            assert!(result.contains("Content for new doc"));
        }
    }
}
