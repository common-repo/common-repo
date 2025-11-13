//! # Error Handling
//!
//! This module defines the centralized error handling mechanism for the
//! `common-repo` application. It uses the `thiserror` library to create a
//! comprehensive `Error` enum that covers all anticipated failure modes,
//! providing clear and descriptive error messages.
//!
//! ## Key Components
//!
//! - **`Error`**: The main enum that represents all possible errors that can
//!   occur within the application. Each variant corresponds to a specific
//!   type of error and includes contextual information to aid in debugging.
//!
//! - **`Result<T>`**: A type alias for `std::result::Result<T, Error>`, used
//!   throughout the application to simplify function signatures and ensure
//!   type safety.
//!
//! The `Error` enum is designed to be exhaustive and cover all possible
//! failure scenarios, including:
//!
//! - Configuration parsing errors.
//! - Git repository cloning issues.
//! - Git command execution failures.
//! - Cache operation errors.
//! - Operator execution errors.
//! - Cycle detection in repository dependencies.
//! - Merge conflicts.
//! - Filesystem operations.
//! - Path operations.
//! - Tool validation errors.
//! - Template processing errors.
//! - Merge operation errors.
//! - Network errors.
//! - I/O errors.
//! - YAML parsing errors.
//! - Regex errors.
//! - Glob pattern errors.
//! - URL parsing errors.
//! - Semver parsing errors.
//! - Lock poisoning.
//! - Serialization errors.
//! - Feature not implemented.
//!
//! Each error variant includes a `message` field and potentially other
//! contextual information (e.g., `url`, `command`, `stderr`, `cycle`,
//! `operator`, `tool`, `src`, `dst`, `operation`, `context`, `feature`).
//!
//! The `Result` type alias is used to return `Result<T, Error>` from
//! functions, making it easy to handle errors and propagate them up the
//! call stack.

use thiserror::Error;

/// Main error type for common-repo operations
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    /// An error occurred while parsing the `.common-repo.yaml` configuration file.
    #[error("Configuration parsing error: {message}")]
    ConfigParse { message: String },

    /// An error occurred while cloning a Git repository.
    #[error("Git clone error for {url}@{r#ref}: {message}")]
    GitClone {
        url: String,
        r#ref: String,
        message: String,
    },

    /// An error occurred while executing a Git command.
    #[error("Git command failed for {url}: {command} - {stderr}")]
    GitCommand {
        command: String,
        url: String,
        stderr: String,
    },

    /// An error occurred with a cache operation.
    #[error("Cache operation error: {message}")]
    Cache { message: String },

    /// An error occurred during the execution of an operator.
    #[error("Operator execution error: {operator} - {message}")]
    Operator { operator: String, message: String },

    /// A circular dependency was detected in the repository inheritance chain.
    #[error("Cycle detected in repository dependencies: {cycle}")]
    CycleDetected { cycle: String },

    /// A warning for a merge conflict, typically when a file would be
    /// overwritten.
    #[error("Merge conflict warning: {src} -> {dst}: {message}")]
    MergeConflict {
        src: String,
        dst: String,
        message: String,
    },

    /// An error occurred with an in-memory filesystem operation.
    #[error("Filesystem operation error: {message}")]
    Filesystem { message: String },

    /// An error occurred with a path-related operation.
    #[error("Path operation error: {message}")]
    Path { message: String },

    /// An error occurred during tool validation.
    #[error("Tool validation error: {tool} - {message}")]
    ToolValidation { tool: String, message: String },

    /// An error occurred during template processing.
    #[error("Template processing error: {message}")]
    Template { message: String },

    /// An error occurred during a merge operation.
    #[error("Merge operation error: {operation} - {message}")]
    Merge { operation: String, message: String },

    /// An error occurred during a network operation.
    #[error("Network operation error: {url} - {message}")]
    Network { url: String, message: String },

    /// An I/O error, wrapped from `std::io::Error`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A YAML parsing error, wrapped from `serde_yaml::Error`.
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// A regular expression error, wrapped from `regex::Error`.
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// A glob pattern error, wrapped from `glob::PatternError`.
    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    /// A URL parsing error, wrapped from `url::ParseError`.
    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// A semantic versioning parsing error, wrapped from `semver::Error`.
    #[error("Semver parsing error: {0}")]
    Semver(#[from] semver::Error),

    /// An error indicating that a mutex or other lock has been poisoned.
    #[error("Lock poisoned: {context}")]
    LockPoisoned { context: String },

    /// An error occurred during serialization.
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// An error for a feature that has not yet been implemented.
    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: String },
}

/// A convenient type alias for `Result<T, Error>`.
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_config_parse() {
        let error = Error::ConfigParse {
            message: "Invalid YAML".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Configuration parsing error"));
        assert!(display.contains("Invalid YAML"));
    }

    #[test]
    fn test_error_display_git_clone() {
        let error = Error::GitClone {
            url: "https://github.com/test/repo.git".to_string(),
            r#ref: "main".to_string(),
            message: "Authentication failed".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Git clone error"));
        assert!(display.contains("https://github.com/test/repo.git"));
        assert!(display.contains("main"));
        assert!(display.contains("Authentication failed"));
    }

    #[test]
    fn test_error_display_git_command() {
        let error = Error::GitCommand {
            command: "ls-remote".to_string(),
            url: "https://github.com/test/repo.git".to_string(),
            stderr: "Permission denied".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Git command failed"));
        assert!(display.contains("ls-remote"));
        assert!(display.contains("Permission denied"));
    }

    #[test]
    fn test_error_display_cycle_detected() {
        let error = Error::CycleDetected {
            cycle: "repo-a -> repo-b -> repo-a".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Cycle detected"));
        assert!(display.contains("repo-a -> repo-b -> repo-a"));
    }

    #[test]
    fn test_error_display_operator() {
        let error = Error::Operator {
            operator: "rename".to_string(),
            message: "Invalid pattern".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Operator execution error"));
        assert!(display.contains("rename"));
        assert!(display.contains("Invalid pattern"));
    }

    #[test]
    fn test_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();
        let display = format!("{}", error);
        assert!(display.contains("I/O error"));
        assert!(display.contains("File not found"));
    }

    #[test]
    fn test_error_from_regex_error() {
        let regex_error = regex::Error::Syntax("Invalid regex".to_string());
        let error: Error = regex_error.into();
        let display = format!("{}", error);
        assert!(display.contains("Regex error"));
    }

    #[test]
    fn test_error_from_yaml_error() {
        let yaml_str = "invalid: [unclosed";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(yaml_str).unwrap_err();
        let error: Error = yaml_error.into();
        let display = format!("{}", error);
        assert!(display.contains("YAML parsing error"));
    }

    #[test]
    fn test_error_cache() {
        let error = Error::Cache {
            message: "Cache operation failed".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Cache operation error"));
        assert!(display.contains("Cache operation failed"));
    }

    #[test]
    fn test_error_filesystem() {
        let error = Error::Filesystem {
            message: "File operation failed".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Filesystem operation error"));
        assert!(display.contains("File operation failed"));
    }

    #[test]
    fn test_error_path() {
        let error = Error::Path {
            message: "Invalid path".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Path operation error"));
        assert!(display.contains("Invalid path"));
    }

    #[test]
    fn test_error_network() {
        let error = Error::Network {
            url: "https://example.com".to_string(),
            message: "Connection timeout".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Network operation error"));
        assert!(display.contains("https://example.com"));
        assert!(display.contains("Connection timeout"));
    }

    #[test]
    fn test_error_template() {
        let error = Error::Template {
            message: "Template processing failed".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Template processing error"));
        assert!(display.contains("Template processing failed"));
    }

    #[test]
    fn test_error_tool_validation() {
        let error = Error::ToolValidation {
            tool: "my-tool".to_string(),
            message: "Tool not found".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Tool validation error"));
        assert!(display.contains("my-tool"));
        assert!(display.contains("Tool not found"));
    }

    #[test]
    fn test_error_merge_conflict() {
        let error = Error::MergeConflict {
            src: "source.txt".to_string(),
            dst: "dest.txt".to_string(),
            message: "File already exists".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Merge conflict warning"));
        assert!(display.contains("source.txt"));
        assert!(display.contains("dest.txt"));
        assert!(display.contains("File already exists"));
    }
}
