//! Property-based tests for path manipulation functions.
//!
//! These tests use proptest to generate random inputs and verify that
//! invariants hold for all possible inputs.

#[cfg(test)]
mod proptest_tests {
    use crate::path::{encode_url_path, glob_match, regex_rename};
    use proptest::prelude::*;

    // ============================================================================
    // encode_url_path property tests
    // ============================================================================

    proptest! {
        /// Property: encode_url_path never produces filesystem-unsafe characters
        #[test]
        fn encode_url_path_never_produces_unsafe_chars(input in ".*") {
            let result = encode_url_path(&input);
            // These characters are problematic on various filesystems
            let unsafe_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
            for ch in unsafe_chars {
                prop_assert!(
                    !result.contains(ch),
                    "encode_url_path produced unsafe character '{}' from input '{}'",
                    ch,
                    input
                );
            }
        }

        /// Property: encode_url_path is deterministic (same input = same output)
        #[test]
        fn encode_url_path_is_deterministic(input in ".*") {
            let result1 = encode_url_path(&input);
            let result2 = encode_url_path(&input);
            prop_assert_eq!(result1, result2);
        }

        /// Property: encode_url_path preserves alphanumeric characters
        #[test]
        fn encode_url_path_preserves_alphanumeric(input in "[a-zA-Z0-9]+") {
            let result = encode_url_path(&input);
            prop_assert_eq!(result, input);
        }

        /// Property: encode_url_path output character count equals input character count
        /// (characters are replaced 1:1, though byte lengths may differ for Unicode)
        #[test]
        fn encode_url_path_preserves_char_count(input in ".+") {
            let result = encode_url_path(&input);
            prop_assert_eq!(
                result.chars().count(),
                input.chars().count(),
                "Character count should be preserved"
            );
        }

        /// Property: encode_url_path of ASCII-only input preserves byte length
        #[test]
        fn encode_url_path_preserves_ascii_length(input in "[[:ascii:]]+") {
            let result = encode_url_path(&input);
            prop_assert_eq!(result.len(), input.len());
        }
    }

    // ============================================================================
    // glob_match property tests
    // ============================================================================

    proptest! {
        /// Property: glob pattern "*" matches any non-empty single path component
        #[test]
        fn glob_star_matches_single_component(path in "[a-zA-Z0-9_.]+") {
            // Pattern "*" should match any single component (no slashes)
            let result = glob_match("*", &path);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap(), "Pattern '*' should match '{}'", path);
        }

        /// Property: glob_match is deterministic
        #[test]
        fn glob_match_is_deterministic(
            pattern in "[a-zA-Z0-9*?_.]+",
            path in "[a-zA-Z0-9_.]+",
        ) {
            let result1 = glob_match(&pattern, &path);
            let result2 = glob_match(&pattern, &path);

            // Both should either succeed or fail
            prop_assert_eq!(result1.is_ok(), result2.is_ok());
            if result1.is_ok() {
                prop_assert_eq!(result1.unwrap(), result2.unwrap());
            }
        }

        /// Property: exact pattern matches only identical path
        #[test]
        fn glob_exact_match_works(path in "[a-zA-Z0-9_]{1,20}") {
            let result = glob_match(&path, &path);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap(), "Exact pattern '{}' should match itself", path);
        }

        /// Property: pattern "**" matches any path
        #[test]
        fn glob_double_star_matches_all(path in "[a-zA-Z0-9_./]+") {
            let result = glob_match("**", &path);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap(), "Pattern '**' should match '{}'", path);
        }
    }

    // ============================================================================
    // regex_rename property tests
    // ============================================================================

    proptest! {
        /// Property: regex_rename is deterministic
        #[test]
        fn regex_rename_is_deterministic(
            path in "[a-zA-Z0-9_.]{1,20}",
            replacement in "[a-zA-Z0-9_$]{1,10}",
        ) {
            // Use a simple valid pattern
            let pattern = r"\w+";

            let result1 = regex_rename(pattern, &replacement, &path);
            let result2 = regex_rename(pattern, &replacement, &path);

            prop_assert_eq!(result1.is_ok(), result2.is_ok());
            if result1.is_ok() {
                prop_assert_eq!(result1.unwrap(), result2.unwrap());
            }
        }

        /// Property: regex_rename with non-matching pattern returns None
        #[test]
        fn regex_rename_non_match_returns_none(path in "[a-zA-Z]+") {
            // Pattern that looks for digits won't match alphabetic path
            let result = regex_rename(r"[0-9]+", "replacement", &path);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), None);
        }

        /// Property: regex_rename with matching pattern returns Some
        #[test]
        fn regex_rename_match_returns_some(path in "[a-zA-Z0-9_]+") {
            // Pattern that matches word characters
            if !path.is_empty() {
                let result = regex_rename(r"\w+", "replaced", &path);
                prop_assert!(result.is_ok());
                prop_assert!(result.unwrap().is_some());
            }
        }

        /// Property: regex_rename captures work correctly
        #[test]
        fn regex_rename_captures_preserve_content(
            prefix in "[a-zA-Z]{1,5}",
            suffix in "[a-zA-Z]{1,5}",
        ) {
            let path = format!("{}.{}", prefix, suffix);
            let result = regex_rename(r"(\w+)\.(\w+)", "$1_$2", &path);

            prop_assert!(result.is_ok());
            let renamed = result.unwrap();
            prop_assert!(renamed.is_some());

            // The result should contain both prefix and suffix
            let renamed_str = renamed.unwrap();
            prop_assert!(
                renamed_str.contains(&prefix),
                "Result '{}' should contain prefix '{}'",
                renamed_str,
                prefix
            );
            prop_assert!(
                renamed_str.contains(&suffix),
                "Result '{}' should contain suffix '{}'",
                renamed_str,
                suffix
            );
        }
    }
}
