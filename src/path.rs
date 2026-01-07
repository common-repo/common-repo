//! # Path Manipulation Utilities
//!
//! This module provides a set of utility functions for path manipulation,
//! including glob matching, regex-based renaming, and URL encoding for
//! filesystem-safe paths. These utilities are used throughout the application,
//! particularly in the implementation of the various operators.
//!
//! ## Key Functions
//!
//! - **`glob_match`**: A simple wrapper around the `glob` crate to check if a
//!   given path matches a glob pattern.
//!
//! - **`regex_rename`**: A powerful function for renaming files using regular
//!   expressions with capture groups. It supports a `$1`, `$2`, etc., syntax for
//!   referencing captured parts of the path in the replacement string.
//!
//! - **`encode_url_path`**: A function to convert a URL into a string that is
//!   safe to use as a directory or file name on the filesystem. This is
//!   important for creating predictable and safe cache directory names.

use crate::error::{Error, Result};
use glob::Pattern;
use regex::Regex;

/// Match a path against a glob pattern
///
/// # Examples
///
/// ```
/// use common_repo::path::glob_match;
///
/// assert!(glob_match("*.rs", "main.rs").unwrap());
/// assert!(glob_match("src/*.rs", "src/main.rs").unwrap());
/// assert!(!glob_match("*.rs", "main.js").unwrap());
/// ```
pub fn glob_match(pattern: &str, path: &str) -> Result<bool> {
    let pattern = Pattern::new(pattern).map_err(Error::Glob)?;
    Ok(pattern.matches(path))
}

/// Renames a path using a regular expression with capture groups.
///
/// The `pattern` is a regular expression that is matched against the `path`.
/// The `replacement` string can reference capture groups from the pattern
/// using a `$1`, `$2`, etc., syntax.
///
/// If the pattern matches the path, the function returns `Some` with the new,
/// renamed path. If the pattern does not match, it returns `None`.
///
/// # Examples
///
/// ```
/// use common_repo::path::regex_rename;
///
/// // Simple replacement
/// assert_eq!(
///     regex_rename(r"(\w+)\.rs", "$1_backup.rs", "main.rs").unwrap(),
///     Some("main_backup.rs".to_string())
/// );
///
/// // Multiple capture groups
/// assert_eq!(
///     regex_rename(r"(\w+)/(\w+)\.rs", "$2_$1.rs", "src/main.rs").unwrap(),
///     Some("main_src.rs".to_string())
/// );
///
/// // No match returns None
/// assert_eq!(
///     regex_rename(r"(\w+)\.js", "$1_backup.js", "main.rs").unwrap(),
///     None
/// );
/// ```
pub fn regex_rename(pattern: &str, replacement: &str, path: &str) -> Result<Option<String>> {
    let regex = Regex::new(pattern).map_err(Error::Regex)?;
    if let Some(captures) = regex.captures(path) {
        // Use captures to expand $1, $2, etc. in the replacement string
        let mut expanded_replacement = String::new();
        let mut chars = replacement.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(digit_char) = chars.peek() {
                    if digit_char.is_ascii_digit() {
                        let digit = digit_char.to_digit(10).unwrap() as usize;
                        chars.next(); // consume the digit
                        if let Some(capture) = captures.get(digit) {
                            expanded_replacement.push_str(capture.as_str());
                        }
                        continue;
                    }
                }
            }
            expanded_replacement.push(ch);
        }

        // Replace the matched portion in the original path
        let matched_range = captures.get(0).unwrap().range();
        let mut result = String::with_capacity(path.len() + expanded_replacement.len());
        result.push_str(&path[..matched_range.start]);
        result.push_str(&expanded_replacement);
        result.push_str(&path[matched_range.end..]);

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Encodes a URL into a filesystem-safe string.
///
/// This function replaces characters that are problematic in file paths (such as
/// `/`, `:`, `*`, etc.) with safe alternatives (`-` or `_`). This is crucial
/// for creating consistent and valid cache directory names from repository URLs.
///
/// # Examples
///
/// ```
/// use common_repo::path::encode_url_path;
///
/// // URL with protocol and slashes
/// assert_eq!(
///     encode_url_path("https://github.com/user/repo.git"),
///     "https_--github.com-user-repo.git"
/// );
///
/// // File URL with multiple slashes
/// assert_eq!(
///     encode_url_path("file:///path/to/repo"),
///     "file_---path-to-repo"
/// );
///
/// // Special characters are replaced with underscores
/// assert_eq!(
///     encode_url_path("test*file?name"),
///     "test_file_name"
/// );
///
/// // Alphanumeric and safe chars are preserved
/// assert_eq!(
///     encode_url_path("normal_file.txt"),
///     "normal_file.txt"
/// );
/// ```
pub fn encode_url_path(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            '/' => '-',
            '\\' => '-',
            ':' => '_',
            '*' => '_',
            '?' => '_',
            '"' => '_',
            '<' => '_',
            '>' => '_',
            '|' => '_',
            // Keep alphanumeric, dots, dashes, underscores as-is
            c if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' => c,
            // Replace other characters with underscores
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.rs", "main.rs").unwrap());
        assert!(glob_match("src/*.rs", "src/main.rs").unwrap());
        assert!(!glob_match("*.rs", "main.js").unwrap());
        assert!(glob_match("**/*.rs", "src/path.rs").unwrap());
    }

    #[test]
    fn test_regex_rename() {
        // Simple replacement
        assert_eq!(
            regex_rename(r"(\w+)\.rs", "$1_backup.rs", "main.rs").unwrap(),
            Some("main_backup.rs".to_string())
        );

        // No match
        assert_eq!(
            regex_rename(r"(\w+)\.js", "$1_backup.js", "main.rs").unwrap(),
            None
        );

        // Multiple capture groups
        assert_eq!(
            regex_rename(r"(\w+)/(\w+)\.rs", "$2_$1.rs", "src/main.rs").unwrap(),
            Some("main_src.rs".to_string())
        );
    }

    #[test]
    fn test_encode_url_path() {
        assert_eq!(
            encode_url_path("https://github.com/user/repo.git"),
            "https_--github.com-user-repo.git"
        );

        assert_eq!(
            encode_url_path("file:///path/to/repo"),
            "file_---path-to-repo"
        );

        assert_eq!(
            encode_url_path("git@host.com:user/repo.git"),
            "git_host.com_user-repo.git"
        );

        // Test special characters
        assert_eq!(encode_url_path("test*file?name"), "test_file_name");
        assert_eq!(encode_url_path("file<with>bars|"), "file_with_bars_");
        assert_eq!(encode_url_path("file\"with\"quotes"), "file_with_quotes");

        // Test that alphanumeric and safe chars are preserved
        assert_eq!(encode_url_path("normal_file.txt"), "normal_file.txt");
        assert_eq!(
            encode_url_path("file_with-dash.and_underscore"),
            "file_with-dash.and_underscore"
        );

        // Test replacement of other special chars
        assert_eq!(
            encode_url_path("file@with#special$chars%"),
            "file_with_special_chars_"
        );

        // Test backslash character specifically (line 66)
        assert_eq!(
            encode_url_path("path\\with\\backslashes"),
            "path-with-backslashes"
        );
    }

    #[test]
    fn test_regex_rename_edge_cases() {
        // Test with no capture groups
        assert_eq!(
            regex_rename(r"old", r"new", "oldfile").unwrap(),
            Some("newfile".to_string())
        );

        // Test replacement that doesn't change the whole string
        assert_eq!(
            regex_rename(r"old", r"new", "old").unwrap(),
            Some("new".to_string())
        );

        // Test with multiple replacements in same string
        assert_eq!(
            regex_rename(r"(\w+)", r"$1_backup", "file1 file2").unwrap(),
            Some("file1_backup file2".to_string())
        );

        // Test with invalid regex
        assert!(regex_rename(r"[invalid", r"replacement", "test").is_err());

        // Test with backreferences beyond available groups
        assert_eq!(
            regex_rename(r"(\w+)", r"$1_$2", "test").unwrap(),
            Some("test_".to_string()) // $2 should be empty
        );

        // Test with escaped dollar signs ($$ becomes $, then $1 becomes capture group)
        assert_eq!(
            regex_rename(r"(\w+)", r"$$1", "test").unwrap(),
            Some("$test".to_string())
        );

        // Test literal $ followed by number
        assert_eq!(
            regex_rename(r"(\w+)", r"prefix$1suffix", "test").unwrap(),
            Some("prefixtestsuffix".to_string())
        );

        // Test empty pattern
        assert_eq!(
            regex_rename(r"", r"prefix", "test").unwrap(),
            Some("prefixtest".to_string())
        );

        // Test pattern that matches empty string
        assert_eq!(
            regex_rename(r"^", r"prefix", "test").unwrap(),
            Some("prefixtest".to_string())
        );
    }

    #[test]
    fn test_glob_match_errors() {
        // Test invalid glob patterns
        assert!(glob_match("*[invalid", "test").is_err());
        assert!(glob_match("**", "test").is_ok()); // Valid glob
    }
}
