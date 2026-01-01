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
    op.validate()?;
    let source_path = op.get_source().expect("source validated");
    let dest_path = op.get_dest().expect("dest validated");

    let source_content = read_file_as_string(fs, source_path)?;
    let dest_content = read_file_as_string_optional(fs, dest_path)?.unwrap_or_default();

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
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
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
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                append: true,
                ..Default::default()
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
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "New Section".to_string(),
                create_section: true,
                ..Default::default()
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
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "First Section".to_string(),
                position: "start".to_string(),
                create_section: true,
                ..Default::default()
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
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Missing".to_string(),
                ..Default::default()
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
                source: Some("source.md".to_string()),
                dest: Some("new_dest.md".to_string()),
                section: "Section".to_string(),
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "new_dest.md").unwrap();
            assert!(result.contains("## Section"));
            assert!(result.contains("Content for new doc"));
        }
    }

    mod header_level_tests {
        use super::*;

        #[test]
        fn test_heading_level_1() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New h1 content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("# Main Title\n\nOld content\n\n# Another Title\n\nMore\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Main Title".to_string(),
                level: 1,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("# Main Title"));
            assert!(result.contains("New h1 content"));
            assert!(!result.contains("Old content"));
            assert!(result.contains("# Another Title"));
        }

        #[test]
        fn test_heading_level_3() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Level 3 content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("# Title\n\n## Section\n\n### Subsection\n\nOld\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Subsection".to_string(),
                level: 3,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("### Subsection"));
            assert!(result.contains("Level 3 content"));
            assert!(!result.contains("Old"));
        }

        #[test]
        fn test_heading_level_6() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Deepest content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("###### Deep\n\nOld deep\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Deep".to_string(),
                level: 6,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("###### Deep"));
            assert!(result.contains("Deepest content"));
            assert!(!result.contains("Old deep"));
        }

        #[test]
        fn test_create_section_with_level_1() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Main content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Existing\n\nText\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "New Main".to_string(),
                level: 1,
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("# New Main"));
            assert!(result.contains("Main content"));
        }

        #[test]
        fn test_create_section_with_level_4() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Sub-subsection"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Details".to_string(),
                level: 4,
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("#### Details"));
            assert!(result.contains("Sub-subsection"));
        }

        #[test]
        fn test_section_bounded_by_equal_level() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Replaced"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string(
                    "## First\n\nFirst content\nMore first\n\n## Second\n\nSecond content\n",
                ),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "First".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## First"));
            assert!(result.contains("Replaced"));
            assert!(!result.contains("First content"));
            assert!(!result.contains("More first"));
            // Second section should be preserved
            assert!(result.contains("## Second"));
            assert!(result.contains("Second content"));
        }

        #[test]
        fn test_section_bounded_by_higher_level() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New sub"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Parent\n\n### Child\n\nOld child\n\n## Sibling\n\nSib\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Child".to_string(),
                level: 3,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("### Child"));
            assert!(result.contains("New sub"));
            assert!(!result.contains("Old child"));
            // Parent should be preserved
            assert!(result.contains("## Parent"));
            // Sibling should be preserved
            assert!(result.contains("## Sibling"));
        }

        #[test]
        fn test_subsections_preserved_when_targeting_parent() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Parent intro"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Parent\n\nOld intro\n\n### Child1\n\nC1\n\n### Child2\n\nC2\n\n## Next\n\nN\n"),
            )
            .unwrap();

            // Note: replacing parent content will also replace children because
            // section bounds extend until the next same-or-higher level heading
            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Parent".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## Parent"));
            assert!(result.contains("Parent intro"));
            // Children get replaced since they were within the parent section
            assert!(!result.contains("### Child1"));
            // Next section preserved
            assert!(result.contains("## Next"));
        }

        #[test]
        fn test_different_section_same_name_different_level() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("For h3"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Notes\n\nLevel 2 notes\n\n### Notes\n\nLevel 3 notes\n"),
            )
            .unwrap();

            // Target the h3 Notes, not the h2 Notes
            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Notes".to_string(),
                level: 3,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // h2 Notes should still have original content
            assert!(result.contains("## Notes"));
            assert!(result.contains("Level 2 notes"));
            // h3 Notes should have new content
            assert!(result.contains("### Notes"));
            assert!(result.contains("For h3"));
            assert!(!result.contains("Level 3 notes"));
        }
    }

    mod content_insertion_tests {
        use super::*;

        #[test]
        fn test_append_to_empty_section() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Appended"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Empty\n\n## Next\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Empty".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## Empty"));
            assert!(result.contains("Appended"));
            assert!(result.contains("## Next"));
        }

        #[test]
        fn test_append_multiline_content() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Line 1\nLine 2\nLine 3\n"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Section\n\nExisting\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                append: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("Existing"));
            assert!(result.contains("Line 1"));
            assert!(result.contains("Line 2"));
            assert!(result.contains("Line 3"));
        }

        #[test]
        fn test_replace_with_multiline_content() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New line 1\nNew line 2\n"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Section\n\nOld line 1\nOld line 2\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("New line 1"));
            assert!(result.contains("New line 2"));
            assert!(!result.contains("Old line 1"));
            assert!(!result.contains("Old line 2"));
        }

        #[test]
        fn test_replace_preserves_heading() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Brand new"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## MySection\n\nOld stuff\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "MySection".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // Heading should be preserved
            assert!(result.contains("## MySection"));
            assert!(result.contains("Brand new"));
        }

        #[test]
        fn test_append_adds_blank_line_separator() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Added content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## Section\n\nExisting content\n## Other\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                append: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // Both contents should be present
            assert!(result.contains("Existing content"));
            assert!(result.contains("Added content"));
        }

        #[test]
        fn test_create_section_at_start_removes_leading_whitespace() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("First"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("\n\n\n# Existing\n\nBody\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "New First".to_string(),
                position: "start".to_string(),
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // Should start with the new section, not blank lines
            assert!(result.starts_with("## New First"));
        }

        #[test]
        fn test_create_section_at_end_adds_separator() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Last")).unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n\nContent here"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Final".to_string(),
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("# Title"));
            assert!(result.contains("Content here"));
            assert!(result.contains("## Final"));
            assert!(result.contains("Last"));
        }

        #[test]
        fn test_empty_source_content() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("")).unwrap();
            fs.add_file("dest.md", File::from_string("## Section\n\nOld content\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // Section should be present but empty content
            assert!(result.contains("## Section"));
            assert!(!result.contains("Old content"));
        }

        #[test]
        fn test_source_without_trailing_newline() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("No newline at end"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Section\n\nOld\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("No newline at end"));
            // Result should still end with newline
            assert!(result.ends_with('\n'));
        }

        #[test]
        fn test_position_case_insensitive() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "New".to_string(),
                position: "START".to_string(),
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // New section should be at start
            let new_pos = result.find("## New").unwrap();
            let title_pos = result.find("# Title").unwrap();
            assert!(new_pos < title_pos);
        }
    }

    mod error_path_tests {
        use super::*;

        #[test]
        fn test_source_file_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("dest.md", File::from_string("# Title\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("missing.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("not found") || err.contains("File not found"));
        }

        #[test]
        fn test_invalid_utf8_in_source() {
            let mut fs = MemoryFS::new();
            // Add file with invalid UTF-8
            fs.add_file("source.md", File::new(vec![0xFF, 0xFE, 0x00, 0x01]))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("UTF-8"));
        }

        #[test]
        fn test_invalid_utf8_in_dest() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            // Add dest file with invalid UTF-8
            fs.add_file("dest.md", File::new(vec![0xFF, 0xFE, 0x00, 0x01]))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("UTF-8"));
        }

        #[test]
        fn test_section_not_found_create_false() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("# Title\n\n## Other\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Missing Section".to_string(),
                ..Default::default()
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("not found"));
            assert!(err.contains("Missing Section"));
        }

        #[test]
        fn test_section_wrong_level_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            // Section exists at level 2, but we're looking for level 3
            fs.add_file("dest.md", File::from_string("## MySection\n\nContent\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "MySection".to_string(),
                level: 3, // Looking for ### MySection
                ..Default::default()
            };

            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("not found"));
        }
    }

    mod edge_case_tests {
        use super::*;

        #[test]
        fn test_empty_destination_file() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("New content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("")).unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                create_section: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## Section"));
            assert!(result.contains("New content"));
        }

        #[test]
        fn test_section_at_very_start() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Replaced"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## First\n\nOriginal\n\n## Second\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "First".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## First"));
            assert!(result.contains("Replaced"));
            assert!(!result.contains("Original"));
            assert!(result.contains("## Second"));
        }

        #[test]
        fn test_section_at_very_end_no_trailing_newline() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Added"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("# Title\n\n## Last\n\nContent"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Last".to_string(),
                append: true,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("Content"));
            assert!(result.contains("Added"));
            assert!(result.ends_with('\n'));
        }

        #[test]
        fn test_section_with_only_heading() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Empty Section\n## Next\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Empty Section".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("## Empty Section"));
            assert!(result.contains("Content"));
            assert!(result.contains("## Next"));
        }

        #[test]
        fn test_heading_with_standard_spacing() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Found it"))
                .unwrap();
            // Standard heading format
            fs.add_file("dest.md", File::from_string("## Spaced Heading\n\nOld\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Spaced Heading".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("Found it"));
            assert!(!result.contains("Old"));
        }

        #[test]
        fn test_heading_whitespace_mismatch_not_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Content"))
                .unwrap();
            // Heading with extra internal spaces does NOT match standard format
            fs.add_file("dest.md", File::from_string("##   Extra Spaces  \n\nOld\n"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Extra Spaces".to_string(),
                ..Default::default()
            };

            // Extra whitespace in heading means section won't be found
            let result = apply_markdown_merge_operation(&mut fs, &op);
            assert!(result.is_err());
        }

        #[test]
        fn test_multiple_sections_same_level() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Target content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## A\n\nA content\n\n## B\n\nB content\n\n## C\n\nC content\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "B".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            // A and C should be unchanged
            assert!(result.contains("A content"));
            assert!(result.contains("C content"));
            // B should be replaced
            assert!(result.contains("Target content"));
            assert!(!result.contains("B content"));
        }

        #[test]
        fn test_deeply_nested_document() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Deep content"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string(
                    "# H1\n\n## H2\n\n### H3\n\n#### H4\n\nOld H4\n\n##### H5\n\n###### H6\n\n",
                ),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "H4".to_string(),
                level: 4,
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("#### H4"));
            assert!(result.contains("Deep content"));
            assert!(!result.contains("Old H4"));
            // Deeper sections get replaced as they're within H4's bounds
        }

        #[test]
        fn test_special_characters_in_section_name() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("Special"))
                .unwrap();
            fs.add_file(
                "dest.md",
                File::from_string("## API: Get /users/{id}\n\nOld API\n"),
            )
            .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "API: Get /users/{id}".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.contains("Special"));
            assert!(!result.contains("Old API"));
        }

        #[test]
        fn test_result_always_ends_with_newline() {
            let mut fs = MemoryFS::new();
            fs.add_file("source.md", File::from_string("No newline"))
                .unwrap();
            fs.add_file("dest.md", File::from_string("## Section"))
                .unwrap();

            let op = MarkdownMergeOp {
                source: Some("source.md".to_string()),
                dest: Some("dest.md".to_string()),
                section: "Section".to_string(),
                ..Default::default()
            };

            apply_markdown_merge_operation(&mut fs, &op).unwrap();

            let result = read_file_as_string(&fs, "dest.md").unwrap();
            assert!(result.ends_with('\n'));
        }
    }

    mod file_io_helper_tests {
        use super::*;

        #[test]
        fn test_read_file_as_string_success() {
            let mut fs = MemoryFS::new();
            fs.add_file("test.txt", File::from_string("Hello World"))
                .unwrap();

            let result = read_file_as_string(&fs, "test.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "Hello World");
        }

        #[test]
        fn test_read_file_as_string_not_found() {
            let fs = MemoryFS::new();

            let result = read_file_as_string(&fs, "missing.txt");
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("not found") || err.contains("File not found"));
        }

        #[test]
        fn test_read_file_as_string_invalid_utf8() {
            let mut fs = MemoryFS::new();
            fs.add_file("binary.bin", File::new(vec![0xFF, 0xFE]))
                .unwrap();

            let result = read_file_as_string(&fs, "binary.bin");
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("UTF-8"));
        }

        #[test]
        fn test_read_file_as_string_optional_found() {
            let mut fs = MemoryFS::new();
            fs.add_file("exists.txt", File::from_string("Data"))
                .unwrap();

            let result = read_file_as_string_optional(&fs, "exists.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("Data".to_string()));
        }

        #[test]
        fn test_read_file_as_string_optional_not_found() {
            let fs = MemoryFS::new();

            let result = read_file_as_string_optional(&fs, "missing.txt");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        #[test]
        fn test_read_file_as_string_optional_invalid_utf8() {
            let mut fs = MemoryFS::new();
            fs.add_file("binary.bin", File::new(vec![0x80, 0x81]))
                .unwrap();

            let result = read_file_as_string_optional(&fs, "binary.bin");
            assert!(result.is_err());
            let err = format!("{:?}", result.unwrap_err());
            assert!(err.contains("UTF-8"));
        }

        #[test]
        fn test_ensure_trailing_newline_adds_newline() {
            let result = ensure_trailing_newline("content".to_string());
            assert_eq!(result, "content\n");
        }

        #[test]
        fn test_ensure_trailing_newline_preserves_existing() {
            let result = ensure_trailing_newline("content\n".to_string());
            assert_eq!(result, "content\n");
        }

        #[test]
        fn test_ensure_trailing_newline_empty_string() {
            let result = ensure_trailing_newline(String::new());
            assert_eq!(result, "\n");
        }

        #[test]
        fn test_write_string_to_file() {
            let mut fs = MemoryFS::new();

            let result = write_string_to_file(&mut fs, "output.txt", "Test content".to_string());
            assert!(result.is_ok());

            let content = read_file_as_string(&fs, "output.txt").unwrap();
            assert_eq!(content, "Test content\n");
        }

        #[test]
        fn test_write_string_to_file_adds_trailing_newline() {
            let mut fs = MemoryFS::new();

            write_string_to_file(&mut fs, "output.txt", "No newline".to_string()).unwrap();

            let content = read_file_as_string(&fs, "output.txt").unwrap();
            assert!(content.ends_with('\n'));
        }
    }
}
