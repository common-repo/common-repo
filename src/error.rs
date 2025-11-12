//! Error handling types for the common-repo application

use thiserror::Error;

/// Main error type for common-repo operations
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    #[error("Configuration parsing error: {message}")]
    ConfigParse { message: String },

    #[error("Git clone error for {url}@{r#ref}: {message}")]
    GitClone {
        url: String,
        r#ref: String,
        message: String,
    },

    #[error("Git command failed for {url}: {command} - {stderr}")]
    GitCommand {
        command: String,
        url: String,
        stderr: String,
    },

    #[error("Cache operation error: {message}")]
    Cache { message: String },

    #[error("Operator execution error: {operator} - {message}")]
    Operator { operator: String, message: String },

    #[error("Cycle detected in repository dependencies: {cycle}")]
    CycleDetected { cycle: String },

    #[error("Merge conflict warning: {src} -> {dst}: {message}")]
    MergeConflict {
        src: String,
        dst: String,
        message: String,
    },

    #[error("Filesystem operation error: {message}")]
    Filesystem { message: String },

    #[error("Path operation error: {message}")]
    Path { message: String },

    #[error("Tool validation error: {tool} - {message}")]
    ToolValidation { tool: String, message: String },

    #[error("Template processing error: {message}")]
    Template { message: String },

    #[error("Network operation error: {url} - {message}")]
    Network { url: String, message: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Semver parsing error: {0}")]
    Semver(#[from] semver::Error),

    #[error("Lock poisoned: {context}")]
    LockPoisoned { context: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: String },
}

/// Result type alias for common-repo operations
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
