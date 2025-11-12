//! Path manipulation utilities for common-repo

use crate::error::{Error, Result};
use glob::Pattern;
use regex::Regex;

/// Match a path against a glob pattern
#[allow(dead_code)]
pub fn glob_match(pattern: &str, path: &str) -> Result<bool> {
    let pattern = Pattern::new(pattern).map_err(Error::Glob)?;
    Ok(pattern.matches(path))
}

/// Apply a regex-based rename operation with capture groups
///
/// The `pattern` is a regex that may contain capture groups.
/// The `replacement` is a string that can reference capture groups using $1, $2, etc.
///
/// Returns the new path if the pattern matches, None if it doesn't match.
#[allow(dead_code)]
pub fn regex_rename(pattern: &str, replacement: &str, path: &str) -> Result<Option<String>> {
    let regex = Regex::new(pattern).map_err(Error::Regex)?;
    if let Some(captures) = regex.captures(path) {
        // Use captures to expand $1, $2, etc. in the replacement string
        let mut result = String::new();
        let mut chars = replacement.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$'
                && let Some(digit_char) = chars.peek()
                && digit_char.is_ascii_digit()
            {
                let digit = digit_char.to_digit(10).unwrap() as usize;
                chars.next(); // consume the digit
                if let Some(capture) = captures.get(digit) {
                    result.push_str(capture.as_str());
                }
                continue;
            }
            result.push(ch);
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Encode a URL path to be filesystem-safe
///
/// This converts URL characters that are problematic for filesystems
/// into safe alternatives.
#[allow(dead_code)]
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
    }
}
