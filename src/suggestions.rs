//! # Error Suggestions
//!
//! This module provides helper functions for generating helpful error
//! messages with hints and suggestions. Following CLI recommendations,
//! errors should tell users what went wrong AND how to fix it.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::suggestions;
//!
//! // Instead of:
//! anyhow::bail!("Configuration file not found: {}", path.display());
//!
//! // Use:
//! return Err(suggestions::config_not_found(path));
//! ```

use std::path::Path;

/// Generate an error for when the configuration file is not found.
///
/// Includes hints about:
/// - Creating a new config file
/// - Using the -c/--config flag
/// - Using the COMMON_REPO_CONFIG environment variable
pub fn config_not_found(path: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "Configuration file not found: {path}\n\n\
         hint: Create a .common-repo.yaml file in your project root\n\
         hint: Use -c/--config to specify a different path\n\
         hint: Set COMMON_REPO_CONFIG environment variable",
        path = path.display()
    )
}

/// Generate an error for when cache clean is called without filters.
///
/// Includes hints about available filter options.
pub fn cache_clean_no_filter() -> anyhow::Error {
    anyhow::anyhow!(
        "At least one filter must be specified for cache clean\n\n\
         hint: Use --all to remove all cached repositories\n\
         hint: Use --unused to remove repositories not in current config\n\
         hint: Use --older-than <DURATION> to remove old entries (e.g., '30d', '1w')"
    )
}

/// Generate an error for an invalid regex pattern.
///
/// Includes hints about common regex mistakes and validation.
pub fn invalid_regex(pattern: &str, error: &regex::Error) -> anyhow::Error {
    let hint = match error {
        regex::Error::Syntax(msg) if msg.contains("unclosed") => {
            "hint: Check for unclosed brackets, parentheses, or braces"
        }
        regex::Error::Syntax(msg) if msg.contains("repetition") => {
            "hint: Repetition operators (+, *, ?) must follow a pattern"
        }
        _ => "hint: Run 'common-repo validate' to check your configuration",
    };

    anyhow::anyhow!(
        "Invalid regex pattern: {pattern}\n\
         error: {error}\n\n\
         {hint}\n\
         hint: Test patterns at https://regex101.com (select Rust flavor)"
    )
}

/// Generate an error for an invalid glob pattern.
///
/// Includes hints about glob syntax.
pub fn invalid_glob(pattern: &str, error: &glob::PatternError) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid glob pattern: {pattern}\n\
         error: {error}\n\n\
         hint: Use * for single path component, ** for recursive matching\n\
         hint: Use [abc] for character classes, [!abc] to negate\n\
         hint: Escape special characters with backslash"
    )
}

/// Generate an error for an unknown operator.
///
/// Includes the list of valid operators.
pub fn unknown_operator(operator: &str) -> anyhow::Error {
    let valid_operators = [
        "include", "exclude", "rename", "repo", "yaml", "toml", "json", "ini", "markdown",
    ];

    // Check for common typos
    let suggestion = find_similar(operator, &valid_operators);
    let did_you_mean = suggestion
        .map(|s| format!("\nhint: Did you mean '{s}'?"))
        .unwrap_or_default();

    anyhow::anyhow!(
        "Unknown operator: {operator}{did_you_mean}\n\n\
         Valid operators are: {ops}\n\
         hint: See documentation at https://common-repo.dev/operators",
        ops = valid_operators.join(", ")
    )
}

/// Generate an error for a cycle detected in repository dependencies.
///
/// Includes hints about how to resolve the cycle.
pub fn cycle_detected(cycle: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Cycle detected in repository dependencies: {cycle}\n\n\
         hint: Remove one of the 'repo:' entries to break the cycle\n\
         hint: Consider extracting shared config into a separate repository"
    )
}

/// Generate an error for tool version validation failure.
///
/// Includes hints about version format.
pub fn tool_version_invalid(tool: &str, version: &str, error: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid version for tool '{tool}': {version}\n\
         error: {error}\n\n\
         hint: Use semantic versioning format (e.g., '1.0.0', '>=1.2.3', '^2.0')\n\
         hint: Prefix with >= for minimum version, ^ for compatible versions"
    )
}

/// Find a similar string from a list of candidates using edit distance.
///
/// Returns Some(candidate) if a close match is found (edit distance <= 2).
fn find_similar<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .filter_map(|&candidate| {
            let distance = edit_distance(input, candidate);
            if distance <= 2 && distance < input.len() {
                Some((candidate, distance))
            } else {
                None
            }
        })
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| candidate)
}

/// Calculate the Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate() {
        *cell = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_not_found_includes_hints() {
        let path = Path::new("/some/path/.common-repo.yaml");
        let error = config_not_found(path);
        let message = error.to_string();

        assert!(message.contains("Configuration file not found"));
        assert!(message.contains("/some/path/.common-repo.yaml"));
        assert!(message.contains("hint:"));
        assert!(message.contains("-c/--config"));
        assert!(message.contains("COMMON_REPO_CONFIG"));
    }

    #[test]
    fn test_cache_clean_no_filter_includes_hints() {
        let error = cache_clean_no_filter();
        let message = error.to_string();

        assert!(message.contains("filter must be specified"));
        assert!(message.contains("--all"));
        assert!(message.contains("--unused"));
        assert!(message.contains("--older-than"));
    }

    #[test]
    fn test_unknown_operator_suggests_similar() {
        let error = unknown_operator("includ");
        let message = error.to_string();

        assert!(message.contains("Unknown operator: includ"));
        assert!(message.contains("Did you mean 'include'?"));
        assert!(message.contains("Valid operators are:"));
    }

    #[test]
    fn test_unknown_operator_no_suggestion_for_very_different() {
        let error = unknown_operator("foobar");
        let message = error.to_string();

        assert!(message.contains("Unknown operator: foobar"));
        assert!(!message.contains("Did you mean"));
        assert!(message.contains("Valid operators are:"));
    }

    #[test]
    fn test_cycle_detected_includes_hints() {
        let error = cycle_detected("repo-a -> repo-b -> repo-a");
        let message = error.to_string();

        assert!(message.contains("Cycle detected"));
        assert!(message.contains("repo-a -> repo-b -> repo-a"));
        assert!(message.contains("hint:"));
        assert!(message.contains("break the cycle"));
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("include", "include"), 0);
        assert_eq!(edit_distance("includ", "include"), 1);
        assert_eq!(edit_distance("incude", "include"), 1);
        assert_eq!(edit_distance("exclude", "include"), 2);
        assert_eq!(edit_distance("foobar", "include"), 7);
    }

    #[test]
    fn test_find_similar() {
        let candidates = ["include", "exclude", "rename"];

        assert_eq!(find_similar("includ", &candidates), Some("include"));
        assert_eq!(find_similar("exclud", &candidates), Some("exclude"));
        assert_eq!(find_similar("renam", &candidates), Some("rename"));
        assert_eq!(find_similar("foobar", &candidates), None);
    }
}
