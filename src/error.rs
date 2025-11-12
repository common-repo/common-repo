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

    #[error("Generic error: {0}")]
    Generic(String),
}

/// Result type alias for common-repo operations
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Error>;
