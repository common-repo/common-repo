//! Merge operations for various file formats
//!
//! This module provides merge functionality for different file formats used in
//! configuration inheritance. Each file format has its own submodule with
//! specialized merge logic.
//!
//! ## Supported Formats
//!
//! - YAML (yaml.rs) - Structured data with anchors and aliases
//! - JSON (json.rs) - JavaScript Object Notation
//! - TOML (toml.rs) - Tom's Obvious Minimal Language
//! - INI (ini.rs) - Simple key-value sections
//! - Markdown (markdown.rs) - Documentation with sections
//!
//! ## Common Types
//!
//! The `PathSegment` enum and path parsing functions are shared across formats
//! to navigate nested data structures during merge operations.

// Merge format modules (extracted from phase5)
pub mod yaml;
// pub mod json;
// pub mod toml;
// pub mod ini;
// pub mod markdown;

/// Represents a segment in a path expression for navigating nested structures
///
/// Path expressions like "servers[0].host" or "database.connection.timeout"
/// are parsed into a sequence of PathSegments for navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    /// A named key for accessing object/map members
    Key(String),
    /// A numeric index for accessing array/sequence elements
    Index(usize),
}

/// Parse a path string into segments for YAML/JSON navigation
///
/// Supports:
/// - Dot notation: `foo.bar.baz`
/// - Bracket notation: `foo["bar"]` or `foo['bar']`
/// - Array indices: `foo[0]` or `items[1].name`
/// - Escaped characters: `foo\.bar` (literal dot)
/// - Mixed: `servers[0].config["special.key"]`
///
/// # Examples
///
/// ```
/// use common_repo::merge::parse_path;
///
/// let segments = parse_path("servers[0].host");
/// assert_eq!(segments.len(), 3);
/// ```
pub fn parse_path(path: &str) -> Vec<PathSegment> {
    if path.trim().is_empty() || path == "/" {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                escaped = true;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_simple_dot_notation() {
        let segments = parse_path("foo.bar.baz");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], PathSegment::Key("foo".to_string()));
        assert_eq!(segments[1], PathSegment::Key("bar".to_string()));
        assert_eq!(segments[2], PathSegment::Key("baz".to_string()));
    }

    #[test]
    fn test_parse_path_array_index() {
        let segments = parse_path("items[0]");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Key("items".to_string()));
        assert_eq!(segments[1], PathSegment::Index(0));
    }

    #[test]
    fn test_parse_path_mixed() {
        let segments = parse_path("servers[0].host");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], PathSegment::Key("servers".to_string()));
        assert_eq!(segments[1], PathSegment::Index(0));
        assert_eq!(segments[2], PathSegment::Key("host".to_string()));
    }

    #[test]
    fn test_parse_path_quoted_key() {
        let segments = parse_path(r#"config["special.key"]"#);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Key("config".to_string()));
        assert_eq!(segments[1], PathSegment::Key("special.key".to_string()));
    }

    #[test]
    fn test_parse_path_empty() {
        assert!(parse_path("").is_empty());
        assert!(parse_path("/").is_empty());
    }

    #[test]
    fn test_parse_path_escaped_dot() {
        let segments = parse_path(r"foo\.bar.baz");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Key("foo.bar".to_string()));
        assert_eq!(segments[1], PathSegment::Key("baz".to_string()));
    }
}
