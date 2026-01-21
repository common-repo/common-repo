//! # Configuration Schema and Parsing
//!
//! This module defines the data structures that represent the `.common-repo.yaml`
//! configuration file, as well as the logic for parsing it. The schema is
//! designed to be flexible and expressive, supporting a wide range of operations
//! for managing repository configurations.
//!
//! ## Key Components
//!
//! - **`Schema`**: A type alias for `Vec<Operation>`, representing the entire
//!   configuration as a sequence of operations.
//!
//! - **`Operation`**: An enum that encompasses all possible actions that can be
//!   defined in the configuration, such as `Repo`, `Include`, `Exclude`, `Rename`,
//!   and various merge operations.
//!
//! - **Operator Structs**: Each variant of the `Operation` enum has a corresponding
//!   struct (e.g., `RepoOp`, `IncludeOp`) that holds its specific configuration.
//!
//! ## Parsing
//!
//! The `parse` function is the main entry point for parsing a YAML string into a
//! `Schema`. It is designed to be backward compatible and supports two formats:
//!
//! 1.  **Current Format**: A more structured format where each operation is
//!     explicitly defined with its parameters. This is the recommended format.
//!
//! 2.  **Original Format**: A more concise, user-friendly format that is supported
//!     for backward compatibility.
//!
//! The parser will first attempt to parse the input using the current format, and
//! if that fails, it will fall back to the original format parser. This ensures
//! that older configuration files continue to work without modification.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a tool requirement with version constraint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g., "pre-commit", "rustc")
    pub name: String,
    /// Version constraint (e.g., "*", ">=1.70", "^3.9")
    pub version: String,
}

/// Template variables context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateVars {
    /// Variable name to value mapping
    pub vars: HashMap<String, String>,
}

/// Repo operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoOp {
    /// The URL of the Git repository to inherit from.
    pub url: String,
    /// The Git reference (e.g., a branch name, tag, or commit hash) to use.
    pub r#ref: String,
    /// An optional sub-path within the repository.
    ///
    /// If specified, this path will be treated as the effective root of the
    /// repository, and only files under this path will be included.
    #[serde(default)]
    pub path: Option<String>,
    /// A list of optional inline operations to apply to this repository before
    /// it is merged. This allows for fine-grained control over the inherited
    /// content.
    #[serde(default)]
    pub with: Vec<Operation>,
}

/// Include operator configuration
///
/// Deserializes directly from a list of patterns:
/// ```yaml
/// - include:
///     - "**/*"
///     - "*.md"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IncludeOp {
    /// A list of glob patterns specifying the files to include.
    pub patterns: Vec<String>,
}

/// Exclude operator configuration
///
/// Deserializes directly from a list of patterns:
/// ```yaml
/// - exclude:
///     - ".git/**"
///     - "*.tmp"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExcludeOp {
    /// A list of glob patterns specifying the files to exclude.
    pub patterns: Vec<String>,
}

/// Template operator configuration
///
/// Deserializes directly from a list of patterns:
/// ```yaml
/// - template:
///     - "templates/**"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateOp {
    /// A list of glob patterns specifying the files to mark as templates.
    pub patterns: Vec<String>,
}

/// Rename operation mapping
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameMapping {
    /// A regular expression used to match file paths.
    pub from: String,
    /// A replacement pattern that can include capture groups from the `from`
    /// regex (e.g., `$1`, `$2`).
    pub to: String,
}

/// Rename operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameOp {
    /// List of rename mappings
    pub mappings: Vec<RenameMapping>,
}

/// Tools operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolsOp {
    /// A list of required tools and their version constraints.
    pub tools: Vec<Tool>,
}

////// CONVERSION IMPLEMENTATIONS //////

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArrayMergeMode {
    #[default]
    Replace,
    Append,
    #[serde(rename = "append_unique")]
    AppendUnique,
}

impl ArrayMergeMode {
    /// Convert from legacy append boolean to ArrayMergeMode
    pub fn from_append_bool(append: bool) -> Self {
        if append {
            ArrayMergeMode::Append
        } else {
            ArrayMergeMode::Replace
        }
    }
}

/// YAML merge operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YamlMergeOp {
    /// Source fragment file (required unless auto_merge is set)
    #[serde(default)]
    pub source: Option<String>,
    /// Destination file to merge into (required unless auto_merge is set)
    #[serde(default)]
    pub dest: Option<String>,
    /// Path within the destination to merge at (optional - merges at root if omitted)
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to append (true) or replace (false) - deprecated, use array_mode instead
    #[serde(default)]
    pub append: bool,
    #[serde(default, rename = "array_mode")]
    pub array_mode: Option<ArrayMergeMode>,
    /// Mark this operation as deferred (applies when repo is used as a source)
    #[serde(default)]
    pub defer: Option<bool>,
    /// Shorthand: sets source=dest to this value and implies defer=true
    #[serde(default, rename = "auto-merge")]
    pub auto_merge: Option<String>,
}

impl YamlMergeOp {
    /// Get the effective array merge mode, considering both array_mode and append fields
    pub fn get_array_mode(&self) -> ArrayMergeMode {
        self.array_mode
            .unwrap_or_else(|| ArrayMergeMode::from_append_bool(self.append))
    }

    /// Validate the merge operation configuration
    pub fn validate(&self) -> Result<()> {
        // If auto_merge is set, source/dest must not be set
        if self.auto_merge.is_some() && (self.source.is_some() || self.dest.is_some()) {
            return Err(Error::ConfigParse {
                message: "Cannot use auto-merge with explicit source or dest".to_string(),
                hint: Some(
                    "Use either 'auto-merge: file.yaml' OR 'source' and 'dest', not both"
                        .to_string(),
                ),
            });
        }
        // If auto_merge is not set, both source and dest are required
        if self.auto_merge.is_none() && (self.source.is_none() || self.dest.is_none()) {
            return Err(Error::ConfigParse {
                message: "YAML merge requires source and dest (or use auto-merge)".to_string(),
                hint: Some(
                    "Add 'source:' and 'dest:' fields, or use 'auto-merge:' for same-name files"
                        .to_string(),
                ),
            });
        }
        Ok(())
    }

    /// Get the effective source path (from auto_merge or source field)
    pub fn get_source(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.source.as_deref())
    }

    /// Get the effective dest path (from auto_merge or dest field)
    pub fn get_dest(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.dest.as_deref())
    }

    /// Check if this operation is deferred
    pub fn is_deferred(&self) -> bool {
        // auto_merge implies defer=true
        self.auto_merge.is_some() || self.defer.unwrap_or(false)
    }

    // Builder methods for fluent construction

    /// Create a new YAML merge operation with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::config::YamlMergeOp;
    ///
    /// let op = YamlMergeOp::new()
    ///     .source("fragment.yaml")
    ///     .dest("config.yaml")
    ///     .path("metadata.labels");
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source file path
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the destination file path
    pub fn dest(mut self, dest: impl Into<String>) -> Self {
        self.dest = Some(dest.into());
        self
    }

    /// Set the path within the destination to merge at
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the array merge mode
    pub fn array_mode(mut self, mode: ArrayMergeMode) -> Self {
        self.array_mode = Some(mode);
        self
    }

    /// Set whether this operation is deferred
    pub fn defer(mut self, defer: bool) -> Self {
        self.defer = Some(defer);
        self
    }

    /// Set auto-merge mode (source=dest, implies defer)
    pub fn auto_merge(mut self, path: impl Into<String>) -> Self {
        self.auto_merge = Some(path.into());
        self
    }
}

/// JSON merge operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct JsonMergeOp {
    /// Source fragment file (required unless auto_merge is set)
    #[serde(default)]
    pub source: Option<String>,
    /// Destination file to merge into (required unless auto_merge is set)
    #[serde(default)]
    pub dest: Option<String>,
    /// Path within the destination to merge at (optional - merges at root if omitted)
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Position for appending ("end" or "start")
    #[serde(default)]
    pub position: Option<String>,
    /// Mark this operation as deferred (applies when repo is used as a source)
    #[serde(default)]
    pub defer: Option<bool>,
    /// Shorthand: sets source=dest to this value and implies defer=true
    #[serde(default, rename = "auto-merge")]
    pub auto_merge: Option<String>,
}

/// TOML merge operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TomlMergeOp {
    /// Source fragment file (required unless auto_merge is set)
    #[serde(default)]
    pub source: Option<String>,
    /// Destination file to merge into (required unless auto_merge is set)
    #[serde(default)]
    pub dest: Option<String>,
    /// Path within the destination to merge at (optional - merges at root if omitted)
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to append (true) or replace (false) - deprecated, use array_mode instead
    #[serde(default)]
    pub append: bool,
    /// Whether to preserve comments
    #[serde(default, rename = "preserve-comments")]
    pub preserve_comments: bool,
    #[serde(default, rename = "array_mode")]
    pub array_mode: Option<ArrayMergeMode>,
    /// Mark this operation as deferred (applies when repo is used as a source)
    #[serde(default)]
    pub defer: Option<bool>,
    /// Shorthand: sets source=dest to this value and implies defer=true
    #[serde(default, rename = "auto-merge")]
    pub auto_merge: Option<String>,
}

impl TomlMergeOp {
    /// Get the effective array merge mode, considering both array_mode and append fields
    pub fn get_array_mode(&self) -> ArrayMergeMode {
        self.array_mode
            .unwrap_or_else(|| ArrayMergeMode::from_append_bool(self.append))
    }

    /// Validate the merge operation configuration
    pub fn validate(&self) -> Result<()> {
        if self.auto_merge.is_some() && (self.source.is_some() || self.dest.is_some()) {
            return Err(Error::ConfigParse {
                message: "Cannot use auto-merge with explicit source or dest".to_string(),
                hint: Some(
                    "Use either 'auto-merge: file.toml' OR 'source' and 'dest', not both"
                        .to_string(),
                ),
            });
        }
        if self.auto_merge.is_none() && (self.source.is_none() || self.dest.is_none()) {
            return Err(Error::ConfigParse {
                message: "TOML merge requires source and dest (or use auto-merge)".to_string(),
                hint: Some(
                    "Add 'source:' and 'dest:' fields, or use 'auto-merge:' for same-name files"
                        .to_string(),
                ),
            });
        }
        Ok(())
    }

    /// Get the effective source path (from auto_merge or source field)
    pub fn get_source(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.source.as_deref())
    }

    /// Get the effective dest path (from auto_merge or dest field)
    pub fn get_dest(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.dest.as_deref())
    }

    /// Check if this operation is deferred
    pub fn is_deferred(&self) -> bool {
        self.auto_merge.is_some() || self.defer.unwrap_or(false)
    }

    // Builder methods for fluent construction

    /// Create a new TOML merge operation with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::config::{TomlMergeOp, ArrayMergeMode};
    ///
    /// let op = TomlMergeOp::new()
    ///     .source("fragment.toml")
    ///     .dest("Cargo.toml")
    ///     .path("dependencies")
    ///     .preserve_comments(true)
    ///     .array_mode(ArrayMergeMode::Append);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source file path
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the destination file path
    pub fn dest(mut self, dest: impl Into<String>) -> Self {
        self.dest = Some(dest.into());
        self
    }

    /// Set the path within the destination to merge at
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the array merge mode
    pub fn array_mode(mut self, mode: ArrayMergeMode) -> Self {
        self.array_mode = Some(mode);
        self
    }

    /// Set whether to preserve comments in the output
    pub fn preserve_comments(mut self, preserve: bool) -> Self {
        self.preserve_comments = preserve;
        self
    }

    /// Set whether this operation is deferred
    pub fn defer(mut self, defer: bool) -> Self {
        self.defer = Some(defer);
        self
    }

    /// Set auto-merge mode (source=dest, implies defer)
    pub fn auto_merge(mut self, path: impl Into<String>) -> Self {
        self.auto_merge = Some(path.into());
        self
    }
}

impl JsonMergeOp {
    /// Validate the merge operation configuration
    pub fn validate(&self) -> Result<()> {
        if self.auto_merge.is_some() && (self.source.is_some() || self.dest.is_some()) {
            return Err(Error::ConfigParse {
                message: "Cannot use auto-merge with explicit source or dest".to_string(),
                hint: Some(
                    "Use either 'auto-merge: file.json' OR 'source' and 'dest', not both"
                        .to_string(),
                ),
            });
        }
        if self.auto_merge.is_none() && (self.source.is_none() || self.dest.is_none()) {
            return Err(Error::ConfigParse {
                message: "JSON merge requires source and dest (or use auto-merge)".to_string(),
                hint: Some(
                    "Add 'source:' and 'dest:' fields, or use 'auto-merge:' for same-name files"
                        .to_string(),
                ),
            });
        }
        Ok(())
    }

    /// Get the effective source path (from auto_merge or source field)
    pub fn get_source(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.source.as_deref())
    }

    /// Get the effective dest path (from auto_merge or dest field)
    pub fn get_dest(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.dest.as_deref())
    }

    /// Check if this operation is deferred
    pub fn is_deferred(&self) -> bool {
        self.auto_merge.is_some() || self.defer.unwrap_or(false)
    }

    // Builder methods for fluent construction

    /// Create a new JSON merge operation with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::config::JsonMergeOp;
    ///
    /// let op = JsonMergeOp::new()
    ///     .source("fragment.json")
    ///     .dest("package.json")
    ///     .path("dependencies")
    ///     .position("end");
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source file path
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the destination file path
    pub fn dest(mut self, dest: impl Into<String>) -> Self {
        self.dest = Some(dest.into());
        self
    }

    /// Set the path within the destination to merge at
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the position for appending ("end" or "start")
    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }

    /// Set whether to append (true) or replace (false)
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Set whether this operation is deferred
    pub fn defer(mut self, defer: bool) -> Self {
        self.defer = Some(defer);
        self
    }

    /// Set auto-merge mode (source=dest, implies defer)
    pub fn auto_merge(mut self, path: impl Into<String>) -> Self {
        self.auto_merge = Some(path.into());
        self
    }
}

/// INI merge operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IniMergeOp {
    /// Source fragment file (required unless auto_merge is set)
    #[serde(default)]
    pub source: Option<String>,
    /// Destination file to merge into (required unless auto_merge is set)
    #[serde(default)]
    pub dest: Option<String>,
    /// Section to merge into (optional - if omitted, merge all sections)
    #[serde(default)]
    pub section: Option<String>,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Whether to allow duplicate keys
    #[serde(default, rename = "allow-duplicates")]
    pub allow_duplicates: bool,
    /// Mark this operation as deferred (applies when repo is used as a source)
    #[serde(default)]
    pub defer: Option<bool>,
    /// Shorthand: sets source=dest to this value and implies defer=true
    #[serde(default, rename = "auto-merge")]
    pub auto_merge: Option<String>,
}

impl IniMergeOp {
    /// Validate the merge operation configuration
    pub fn validate(&self) -> Result<()> {
        if self.auto_merge.is_some() && (self.source.is_some() || self.dest.is_some()) {
            return Err(Error::ConfigParse {
                message: "Cannot use auto-merge with explicit source or dest".to_string(),
                hint: Some(
                    "Use either 'auto-merge: file.ini' OR 'source' and 'dest', not both"
                        .to_string(),
                ),
            });
        }
        if self.auto_merge.is_none() && (self.source.is_none() || self.dest.is_none()) {
            return Err(Error::ConfigParse {
                message: "INI merge requires source and dest (or use auto-merge)".to_string(),
                hint: Some(
                    "Add 'source:' and 'dest:' fields, or use 'auto-merge:' for same-name files"
                        .to_string(),
                ),
            });
        }
        Ok(())
    }

    /// Get the effective source path (from auto_merge or source field)
    pub fn get_source(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.source.as_deref())
    }

    /// Get the effective dest path (from auto_merge or dest field)
    pub fn get_dest(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.dest.as_deref())
    }

    /// Check if this operation is deferred
    pub fn is_deferred(&self) -> bool {
        self.auto_merge.is_some() || self.defer.unwrap_or(false)
    }

    // Builder methods for fluent construction

    /// Create a new INI merge operation with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::config::IniMergeOp;
    ///
    /// let op = IniMergeOp::new()
    ///     .source("fragment.ini")
    ///     .dest("config.ini")
    ///     .section("database")
    ///     .allow_duplicates(true);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source file path
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the destination file path
    pub fn dest(mut self, dest: impl Into<String>) -> Self {
        self.dest = Some(dest.into());
        self
    }

    /// Set the section to merge into
    pub fn section(mut self, section: impl Into<String>) -> Self {
        self.section = Some(section.into());
        self
    }

    /// Set whether to append (true) or replace (false)
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Set whether to allow duplicate keys
    pub fn allow_duplicates(mut self, allow: bool) -> Self {
        self.allow_duplicates = allow;
        self
    }

    /// Set whether this operation is deferred
    pub fn defer(mut self, defer: bool) -> Self {
        self.defer = Some(defer);
        self
    }

    /// Set auto-merge mode (source=dest, implies defer)
    pub fn auto_merge(mut self, path: impl Into<String>) -> Self {
        self.auto_merge = Some(path.into());
        self
    }
}

/// Markdown merge operator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMergeOp {
    /// Source fragment file (required unless auto_merge is set)
    #[serde(default)]
    pub source: Option<String>,
    /// Destination file to merge into (required unless auto_merge is set)
    #[serde(default)]
    pub dest: Option<String>,
    /// Section header to merge under (empty string means merge entire file)
    #[serde(default)]
    pub section: String,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Header level (1-6)
    #[serde(default = "default_header_level")]
    pub level: u8,
    /// Position to insert ("end" or "start")
    #[serde(default)]
    pub position: String,
    /// Whether to create section if it doesn't exist
    #[serde(default, rename = "create-section")]
    pub create_section: bool,
    /// Mark this operation as deferred (applies when repo is used as a source)
    #[serde(default)]
    pub defer: Option<bool>,
    /// Shorthand: sets source=dest to this value and implies defer=true
    #[serde(default, rename = "auto-merge")]
    pub auto_merge: Option<String>,
}

/// Get the default header level for markdown operations
///
/// # Examples
///
/// ```
/// use common_repo::config::default_header_level;
///
/// assert_eq!(default_header_level(), 2);
/// ```
pub fn default_header_level() -> u8 {
    2
}

impl MarkdownMergeOp {
    /// Validate the merge operation configuration
    pub fn validate(&self) -> Result<()> {
        if self.auto_merge.is_some() && (self.source.is_some() || self.dest.is_some()) {
            return Err(Error::ConfigParse {
                message: "Cannot use auto-merge with explicit source or dest".to_string(),
                hint: Some(
                    "Use either 'auto-merge: file.md' OR 'source' and 'dest', not both".to_string(),
                ),
            });
        }
        if self.auto_merge.is_none() && (self.source.is_none() || self.dest.is_none()) {
            return Err(Error::ConfigParse {
                message: "Markdown merge requires source and dest (or use auto-merge)".to_string(),
                hint: Some(
                    "Add 'source:' and 'dest:' fields, or use 'auto-merge:' for same-name files"
                        .to_string(),
                ),
            });
        }
        Ok(())
    }

    /// Get the effective source path (from auto_merge or source field)
    pub fn get_source(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.source.as_deref())
    }

    /// Get the effective dest path (from auto_merge or dest field)
    pub fn get_dest(&self) -> Option<&str> {
        self.auto_merge.as_deref().or(self.dest.as_deref())
    }

    /// Check if this operation is deferred
    pub fn is_deferred(&self) -> bool {
        self.auto_merge.is_some() || self.defer.unwrap_or(false)
    }

    // Builder methods for fluent construction

    /// Create a new Markdown merge operation with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::config::MarkdownMergeOp;
    ///
    /// let op = MarkdownMergeOp::new()
    ///     .source("fragment.md")
    ///     .dest("README.md")
    ///     .section("Installation")
    ///     .level(2)
    ///     .position("end")
    ///     .create_section(true);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source file path
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the destination file path
    pub fn dest(mut self, dest: impl Into<String>) -> Self {
        self.dest = Some(dest.into());
        self
    }

    /// Set the section header to merge under (required)
    pub fn section(mut self, section: impl Into<String>) -> Self {
        self.section = section.into();
        self
    }

    /// Set whether to append (true) or replace (false)
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Set the header level (1-6)
    pub fn level(mut self, level: u8) -> Self {
        self.level = level;
        self
    }

    /// Set the position to insert ("end" or "start")
    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = position.into();
        self
    }

    /// Set whether to create section if it doesn't exist
    pub fn create_section(mut self, create: bool) -> Self {
        self.create_section = create;
        self
    }

    /// Set whether this operation is deferred
    pub fn defer(mut self, defer: bool) -> Self {
        self.defer = Some(defer);
        self
    }

    /// Set auto-merge mode (source=dest, implies defer)
    pub fn auto_merge(mut self, path: impl Into<String>) -> Self {
        self.auto_merge = Some(path.into());
        self
    }
}

impl Default for MarkdownMergeOp {
    fn default() -> Self {
        Self {
            source: None,
            dest: None,
            section: String::new(),
            append: false,
            level: default_header_level(),
            position: String::new(),
            create_section: false,
            defer: None,
            auto_merge: None,
        }
    }
}

/// All possible operation types in the configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    /// Inherit from another repository. This is the core operation for sharing
    /// configurations.
    Repo { repo: RepoOp },
    /// Include a set of files in the final output.
    Include { include: IncludeOp },
    /// Exclude a set of files from the final output.
    Exclude { exclude: ExcludeOp },
    /// Mark a set of files as templates to be processed for variable
    /// substitution.
    Template { template: TemplateOp },
    /// Rename files based on regular expression patterns.
    Rename { rename: RenameOp },
    /// Declare required tools and their versions, which can be validated.
    Tools { tools: ToolsOp },
    /// Define variables to be used in template substitution.
    TemplateVars { template_vars: TemplateVars },
    /// Merge the content of two YAML files.
    Yaml { yaml: YamlMergeOp },
    /// Merge the content of two JSON files.
    Json { json: JsonMergeOp },
    /// Merge the content of two TOML files.
    Toml { toml: TomlMergeOp },
    /// Merge the content of two INI files.
    Ini { ini: IniMergeOp },
    /// Merge fragments of two Markdown files.
    Markdown { markdown: MarkdownMergeOp },
}

impl Operation {
    /// Check if this operation is deferred (applies when repo is used as a source)
    ///
    /// Deferred operations have `defer: true` or `auto-merge` set.
    /// Only merge operators (yaml, json, toml, ini, markdown) can be deferred.
    pub fn is_deferred(&self) -> bool {
        match self {
            Operation::Yaml { yaml } => yaml.is_deferred(),
            Operation::Json { json } => json.is_deferred(),
            Operation::Toml { toml } => toml.is_deferred(),
            Operation::Ini { ini } => ini.is_deferred(),
            Operation::Markdown { markdown } => markdown.is_deferred(),
            // Non-merge operators cannot be deferred
            _ => false,
        }
    }
}

/// The complete configuration schema, represented as a list of operations.
///
/// The operations are executed in the order they are defined in the file.
pub type Schema = Vec<Operation>;

/// Parses a YAML string into a `Schema`.
///
/// This function supports both the current, more structured format and the
/// original, more concise format for backward compatibility. It will first
/// attempt to parse as the current format, and if that fails, it will fall back
/// to the original format parser.
pub fn parse(yaml_content: &str) -> Result<Schema> {
    // First try parsing as the current format
    match serde_yaml::from_str::<Schema>(yaml_content) {
        Ok(schema) => Ok(schema),
        Err(_) => {
            // If that fails, try parsing as the original user-friendly format
            parse_original_format(yaml_content)
        }
    }
}

/// Parse a YAML string using only the original user-friendly format
pub fn parse_original_format(yaml_content: &str) -> Result<Schema> {
    use serde_yaml::Value;

    // Parse as raw YAML values first
    let raw_values: Vec<Value> = serde_yaml::from_str(yaml_content).map_err(Error::Yaml)?;

    let mut operations = Vec::new();

    for value in raw_values {
        if let Value::Mapping(map) = value {
            // Convert the mapping to an Operation
            let operation = convert_yaml_mapping_to_operation(map)?;
            operations.push(operation);
        } else {
            return Err(Error::ConfigParse {
                message: "Expected YAML mapping for operation".to_string(),
                hint: None,
            });
        }
    }

    Ok(operations)
}

/// Convert a YAML mapping to an Operation (handles original format)
fn convert_yaml_mapping_to_operation(map: serde_yaml::Mapping) -> Result<Operation> {
    let mut iter = map.into_iter();
    let (key, value) = iter.next().ok_or_else(|| Error::ConfigParse {
        message: "Empty operation mapping".to_string(),
        hint: None,
    })?;

    let op_type = key.as_str().ok_or_else(|| Error::ConfigParse {
        message: "Operation key must be string".to_string(),
        hint: None,
    })?;

    match op_type {
        "repo" => {
            // Special handling for repo operations since they may contain 'with' operations
            // that are also in the original format
            let mut repo_map = match value {
                serde_yaml::Value::Mapping(m) => m,
                _ => {
                    return Err(Error::ConfigParse {
                        message: "Repo operation must be a mapping".to_string(),
                        hint: Some("Use 'repo: {url: ..., ref: ...}' format".to_string()),
                    });
                }
            };

            let url = repo_map
                .remove(serde_yaml::Value::String("url".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .ok_or_else(|| Error::ConfigParse {
                    message: "Repo operation missing url".to_string(),
                    hint: Some("Add 'url: https://github.com/...' to the repo block".to_string()),
                })?;

            let r#ref = repo_map
                .remove(serde_yaml::Value::String("ref".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .ok_or_else(|| Error::ConfigParse {
                    message: "Repo operation missing ref".to_string(),
                    hint: Some("Add 'ref: main' or 'ref: v1.0.0' to the repo block".to_string()),
                })?;

            let path = repo_map
                .remove(serde_yaml::Value::String("path".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            let with = if let Some(with_value) =
                repo_map.remove(serde_yaml::Value::String("with".to_string()))
            {
                match with_value {
                    serde_yaml::Value::Sequence(seq) => {
                        let mut with_ops = Vec::new();
                        for item in seq {
                            if let serde_yaml::Value::Mapping(map) = item {
                                let op = convert_yaml_mapping_to_operation(map)?;
                                with_ops.push(op);
                            } else {
                                return Err(Error::ConfigParse {
                                    message: "With clause items must be mappings".to_string(),
                                    hint: None,
                                });
                            }
                        }
                        with_ops
                    }
                    _ => {
                        return Err(Error::ConfigParse {
                            message: "With clause must be a sequence".to_string(),
                            hint: Some("Use 'with: [{ include: [...] }, ...]' format".to_string()),
                        })
                    }
                }
            } else {
                Vec::new()
            };

            Ok(Operation::Repo {
                repo: RepoOp {
                    url,
                    r#ref,
                    path,
                    with,
                },
            })
        }
        "include" => {
            let patterns: Vec<String> = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Include {
                include: IncludeOp { patterns },
            })
        }
        "exclude" => {
            let patterns: Vec<String> = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Exclude {
                exclude: ExcludeOp { patterns },
            })
        }
        "template" => {
            let patterns: Vec<String> = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Template {
                template: TemplateOp { patterns },
            })
        }
        "rename" => {
            // Try parsing as the current format first (Vec<RenameMapping>)
            match serde_yaml::from_value::<Vec<RenameMapping>>(value.clone()) {
                Ok(mappings) => Ok(Operation::Rename {
                    rename: RenameOp { mappings },
                }),
                Err(_) => {
                    // Fall back to original format: Vec<HashMap<String, String>>
                    // where each hashmap represents {"regex": "replacement"}
                    let original_mappings: Vec<std::collections::HashMap<String, String>> =
                        serde_yaml::from_value(value).map_err(Error::Yaml)?;

                    let mappings: Vec<RenameMapping> = original_mappings
                        .into_iter()
                        .map(|map| {
                            let mut iter = map.into_iter();
                            let (from, to) = iter.next().ok_or_else(|| Error::ConfigParse {
                                message: "Empty rename mapping".to_string(),
                                hint: Some("Use 'rename: [{ from: \"pattern\", to: \"replacement\" }]' format".to_string()),
                            })?;
                            Ok(RenameMapping { from, to })
                        })
                        .collect::<Result<Vec<_>>>()?;

                    Ok(Operation::Rename {
                        rename: RenameOp { mappings },
                    })
                }
            }
        }
        "tools" => {
            // Try parsing as the current format first (Vec<Tool>)
            match serde_yaml::from_value::<Vec<Tool>>(value.clone()) {
                Ok(tools) => Ok(Operation::Tools {
                    tools: ToolsOp { tools },
                }),
                Err(_) => {
                    // Fall back to original format: Vec<HashMap<String, String>>
                    // where each hashmap represents {"tool_name": "version"}
                    let original_tools: Vec<std::collections::HashMap<String, String>> =
                        serde_yaml::from_value(value).map_err(Error::Yaml)?;

                    let tools: Vec<Tool> = original_tools
                        .into_iter()
                        .map(|map| {
                            let mut iter = map.into_iter();
                            let (name, version) =
                                iter.next().ok_or_else(|| Error::ConfigParse {
                                    message: "Empty tool specification".to_string(),
                                    hint: Some("Use 'tools: [{ name: \"tool\", version: \">=1.0\" }]' format".to_string()),
                                })?;
                            Ok(Tool { name, version })
                        })
                        .collect::<Result<Vec<_>>>()?;

                    Ok(Operation::Tools {
                        tools: ToolsOp { tools },
                    })
                }
            }
        }
        "template-vars" => {
            // Try parsing as the current format first (TemplateVars with vars field)
            match serde_yaml::from_value::<TemplateVars>(value.clone()) {
                Ok(template_vars) => Ok(Operation::TemplateVars { template_vars }),
                Err(_) => {
                    // Fall back to original format: direct HashMap<String, String>
                    let vars: std::collections::HashMap<String, String> =
                        serde_yaml::from_value(value).map_err(Error::Yaml)?;

                    Ok(Operation::TemplateVars {
                        template_vars: TemplateVars { vars },
                    })
                }
            }
        }
        "yaml" => {
            let yaml: YamlMergeOp = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Yaml { yaml })
        }
        "json" => {
            let json: JsonMergeOp = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Json { json })
        }
        "toml" => {
            let toml: TomlMergeOp = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Toml { toml })
        }
        "ini" => {
            let ini: IniMergeOp = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Ini { ini })
        }
        "markdown" => {
            let markdown: MarkdownMergeOp = serde_yaml::from_value(value).map_err(Error::Yaml)?;
            Ok(Operation::Markdown { markdown })
        }
        _ => Err(Error::ConfigParse {
            message: format!("Unknown operation type: {}", op_type),
            hint: Some("Valid operations: repo, include, exclude, template, template-vars, rename, tools, yaml, json, toml, ini, markdown".to_string()),
        }),
    }
}

/// Parse a Schema from a YAML file path
pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Schema> {
    let content = std::fs::read_to_string(path).map_err(Error::Io)?;
    parse(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let yaml = r#"
- repo:
    url: https://github.com/example/repo
    ref: main
- include:
    - "**/*"
    - "*.md"
- exclude:
    - ".git/**"
    - "*.tmp"
"#;

        let schema = parse(yaml).unwrap();
        assert_eq!(schema.len(), 3);

        match &schema[0] {
            Operation::Repo { repo } => {
                assert_eq!(repo.url, "https://github.com/example/repo");
                assert_eq!(repo.r#ref, "main");
                assert!(repo.with.is_empty());
            }
            _ => panic!("Expected Repo operation"),
        }

        match &schema[1] {
            Operation::Include { include } => {
                assert_eq!(include.patterns, vec!["**/*", "*.md"]);
            }
            _ => panic!("Expected Include operation"),
        }

        match &schema[2] {
            Operation::Exclude { exclude } => {
                assert_eq!(exclude.patterns, vec![".git/**", "*.tmp"]);
            }
            _ => panic!("Expected Exclude operation"),
        }
    }

    #[test]
    fn test_parse_rename_operation() {
        let yaml = r#"
- rename:
    mappings:
      - from: "badname/(.*)"
        to: "goodname/$1"
      - from: "^files/(.*)"
        to: "$1"
"#;

        let schema = parse(yaml).unwrap();
        assert_eq!(schema.len(), 1);

        match &schema[0] {
            Operation::Rename { rename } => {
                assert_eq!(rename.mappings.len(), 2);
                assert_eq!(rename.mappings[0].from, "badname/(.*)");
                assert_eq!(rename.mappings[0].to, "goodname/$1");
                assert_eq!(rename.mappings[1].from, "^files/(.*)");
                assert_eq!(rename.mappings[1].to, "$1");
            }
            _ => panic!("Expected Rename operation"),
        }
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let invalid_yaml = r#"
- repo:
    url: https://github.com/example/repo
  ref: missing-colon
- include:
  patterns: ["valid"]
"#;

        let result = parse(invalid_yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_config() {
        let empty_yaml = "";
        let result = parse(empty_yaml);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_malformed_operation() {
        let malformed_yaml = r#"
- invalid_operation:
    some_field: value
"#;

        let result = parse(malformed_yaml);
        // This should fail because the YAML doesn't match any known operation variant
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_nonexistent() {
        let result = from_file("nonexistent_file.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complex_config_with_all_operations() {
        let complex_yaml = r#"
- repo:
    url: https://github.com/example/repo
    ref: main
    with:
      - include:
          - "src/**/*"
- include:
    - "**/*.md"
    - "docs/**"
- exclude:
    - ".git/**"
    - "*.tmp"
- template:
    - "*.template"
- rename:
    mappings:
      - from: "old_(.*)"
        to: "new_$1"
- tools:
    tools:
      - name: rustc
        version: ">=1.70"
- template_vars:
    vars:
      project_name: "my-project"
- yaml:
    source: "config.yaml"
    dest: "merged.yaml"
    path: "$.config"
    append: true
- json:
    source: "data.json"
    dest: "config.json"
    path: "$.data"
    append: false
- toml:
    source: "settings.toml"
    dest: "config.toml"
    path: "package"
    append: false
- ini:
    source: "config.ini"
    dest: "merged.ini"
    section: "main"
    append: true
- markdown:
    source: "readme.md"
    dest: "README.md"
    section: "Features"
    append: false
    level: 2
"#;

        let schema = parse(complex_yaml).unwrap();
        assert_eq!(schema.len(), 12);

        // Verify a few key operations
        match &schema[0] {
            Operation::Repo { repo } => {
                assert_eq!(repo.url, "https://github.com/example/repo");
                assert_eq!(repo.r#ref, "main");
                assert_eq!(repo.with.len(), 1);
            }
            _ => panic!("Expected Repo operation"),
        }

        match &schema[1] {
            Operation::Include { include } => {
                assert_eq!(include.patterns, vec!["**/*.md", "docs/**"]);
            }
            _ => panic!("Expected Include operation"),
        }

        match &schema[4] {
            Operation::Rename { rename } => {
                assert_eq!(rename.mappings.len(), 1);
                assert_eq!(rename.mappings[0].from, "old_(.*)");
                assert_eq!(rename.mappings[0].to, "new_$1");
            }
            _ => panic!("Expected Rename operation"),
        }
    }

    #[test]
    fn test_parse_nested_repo_operations() {
        let nested_yaml = r#"
- repo:
    url: https://github.com/parent/repo
    ref: v1.0.0
    with:
      - repo:
          url: https://github.com/child/repo
          ref: main
          with:
            - include:
                - "src/**"
      - exclude:
          - "tests/**"
"#;

        let schema = parse(nested_yaml).unwrap();
        assert_eq!(schema.len(), 1);

        match &schema[0] {
            Operation::Repo { repo } => {
                assert_eq!(repo.url, "https://github.com/parent/repo");
                assert_eq!(repo.with.len(), 2);

                match &repo.with[0] {
                    Operation::Repo { repo: child_repo } => {
                        assert_eq!(child_repo.url, "https://github.com/child/repo");
                        assert_eq!(child_repo.with.len(), 1);
                    }
                    _ => panic!("Expected nested Repo operation"),
                }

                match &repo.with[1] {
                    Operation::Exclude { exclude } => {
                        assert_eq!(exclude.patterns, vec!["tests/**"]);
                    }
                    _ => panic!("Expected Exclude operation"),
                }
            }
            _ => panic!("Expected Repo operation"),
        }
    }

    #[test]
    fn test_default_header_level() {
        assert_eq!(default_header_level(), 2);
    }

    #[test]
    fn test_parse_schema_yaml_examples() {
        // Test that all the YAML examples in schema.yaml can be parsed
        // This extracts the actual YAML content from the commented schema file

        // Test repo operation with with clause (original format)
        let repo_yaml = r#"
- repo:
    url: https://github.com/shakefu/commonrepo
    ref: v1.1.0
    with:
      - include: [.*]
      - exclude: [.gitignore]
      - rename: [{".*\\.md": "docs/%[1]s"}]
"#;
        let schema = parse(repo_yaml).expect("Failed to parse repo example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Repo { repo } => {
                assert_eq!(repo.url, "https://github.com/shakefu/commonrepo");
                assert_eq!(repo.r#ref, "v1.1.0");
                assert_eq!(repo.with.len(), 3);
            }
            _ => panic!("Expected Repo operation"),
        }

        // Test include operation
        let include_yaml = r#"
- include:
    - "**/*"
    - .*/**/*
    - .gitignore
"#;
        let schema = parse(include_yaml).expect("Failed to parse include example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Include { include } => {
                assert_eq!(include.patterns, vec!["**/*", ".*/**/*", ".gitignore"]);
            }
            _ => panic!("Expected Include operation"),
        }

        // Test exclude operation
        let exclude_yaml = r#"
- exclude:
    - .github/workflows/template_*
    - "**/*.md"
"#;
        let schema = parse(exclude_yaml).expect("Failed to parse exclude example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Exclude { exclude } => {
                assert_eq!(
                    exclude.patterns,
                    vec![".github/workflows/template_*", "**/*.md"]
                );
            }
            _ => panic!("Expected Exclude operation"),
        }

        // Test template operation
        let template_yaml = r#"
- template:
    - "templates/**"
"#;
        let schema =
            parse(template_yaml).expect("Failed to parse template example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Template { template } => {
                assert_eq!(template.patterns, vec!["templates/**"]);
            }
            _ => panic!("Expected Template operation"),
        }

        // Test rename operation (original compact format)
        let rename_yaml = r#"
- rename:
    - "badname/(.*)": "goodname/%[1]s"
    - "^files/(.*)": "%[1]s"
    - "parent/([^/]+)/dir/(.*)": "%[1]s/%[2]s"
    - "(.*\\.md)": "docs/%[1]s"
"#;
        let schema = parse(rename_yaml).expect("Failed to parse rename example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Rename { rename } => {
                assert_eq!(rename.mappings.len(), 4);
                assert_eq!(rename.mappings[0].from, "badname/(.*)");
                assert_eq!(rename.mappings[0].to, "goodname/%[1]s");
                assert_eq!(rename.mappings[1].from, "^files/(.*)");
                assert_eq!(rename.mappings[1].to, "%[1]s");
            }
            _ => panic!("Expected Rename operation"),
        }

        // Test tools operation
        let tools_yaml = r#"
- tools:
    - pre-commit: "*"
    - rustc: ">=1.70"
    - python: "^3.9"
    - node: "~18.0"
"#;
        let schema = parse(tools_yaml).expect("Failed to parse tools example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Tools { tools } => {
                assert_eq!(tools.tools.len(), 4);
                assert_eq!(tools.tools[0].name, "pre-commit");
                assert_eq!(tools.tools[0].version, "*");
                assert_eq!(tools.tools[1].name, "rustc");
                assert_eq!(tools.tools[1].version, ">=1.70");
            }
            _ => panic!("Expected Tools operation"),
        }

        // Test template-vars operation
        let template_vars_yaml = r#"
- template-vars:
    project: ${PROJECT_NAME:-myprojectname}
"#;
        let schema = parse(template_vars_yaml)
            .expect("Failed to parse template-vars example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::TemplateVars { template_vars } => {
                assert_eq!(
                    template_vars.vars.get("project"),
                    Some(&"${PROJECT_NAME:-myprojectname}".to_string())
                );
            }
            _ => panic!("Expected TemplateVars operation"),
        }

        // Test yaml merge operation
        let yaml_yaml = r#"
- yaml:
    source: fragment.yml
    dest: config.yml
    path: metadata.labels
    append: true
"#;
        let schema = parse(yaml_yaml).expect("Failed to parse yaml merge example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Yaml { yaml } => {
                assert_eq!(yaml.source.as_deref(), Some("fragment.yml"));
                assert_eq!(yaml.dest.as_deref(), Some("config.yml"));
                assert_eq!(yaml.path.as_deref(), Some("metadata.labels"));
                assert!(yaml.append);
            }
            _ => panic!("Expected Yaml operation"),
        }

        // Test json merge operation
        let json_yaml = r#"
- json:
    source: fragment.json
    dest: package.json
    path: dependencies
    append: true
    position: end
"#;
        let schema = parse(json_yaml).expect("Failed to parse json merge example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Json { json } => {
                assert_eq!(json.source.as_deref(), Some("fragment.json"));
                assert_eq!(json.dest.as_deref(), Some("package.json"));
                assert_eq!(json.path, Some("dependencies".to_string()));
                assert!(json.append);
                assert_eq!(json.position, Some("end".to_string()));
            }
            _ => panic!("Expected Json operation"),
        }

        // Test toml merge operation
        let toml_yaml = r#"
- toml:
    source: fragment.toml
    dest: Cargo.toml
    path: dependencies
    append: true
    preserve-comments: true
"#;
        let schema = parse(toml_yaml).expect("Failed to parse toml merge example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Toml { toml } => {
                assert_eq!(toml.source.as_deref(), Some("fragment.toml"));
                assert_eq!(toml.dest.as_deref(), Some("Cargo.toml"));
                assert_eq!(toml.path.as_deref(), Some("dependencies"));
                assert!(toml.append);
                assert!(toml.preserve_comments);
            }
            _ => panic!("Expected Toml operation"),
        }

        // Test ini merge operation
        let ini_yaml = r#"
- ini:
    source: fragment.ini
    dest: config.ini
    section: database
    append: true
    allow-duplicates: false
"#;
        let schema = parse(ini_yaml).expect("Failed to parse ini merge example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Ini { ini } => {
                assert_eq!(ini.source.as_deref(), Some("fragment.ini"));
                assert_eq!(ini.dest.as_deref(), Some("config.ini"));
                assert_eq!(ini.section, Some("database".to_string()));
                assert!(ini.append);
                assert!(!ini.allow_duplicates);
            }
            _ => panic!("Expected Ini operation"),
        }

        // Test markdown merge operation
        let markdown_yaml = r#"
- markdown:
    source: fragment.md
    dest: README.md
    section: "Installation"
    append: true
    level: 2
    position: end
    create-section: true
"#;
        let schema =
            parse(markdown_yaml).expect("Failed to parse markdown merge example from schema.yaml");
        assert_eq!(schema.len(), 1);
        match &schema[0] {
            Operation::Markdown { markdown } => {
                assert_eq!(markdown.source.as_deref(), Some("fragment.md"));
                assert_eq!(markdown.dest.as_deref(), Some("README.md"));
                assert_eq!(markdown.section, "Installation");
                assert!(markdown.append);
                assert_eq!(markdown.level, 2);
                assert_eq!(markdown.position, "end".to_string());
                assert!(markdown.create_section);
            }
            _ => panic!("Expected Markdown operation"),
        }

        println!("✅ All schema.yaml examples parse successfully!");
    }

    #[test]
    fn test_parse_original_format() {
        // Test parsing the original user-friendly format (without nested field names)
        let yaml = r#"
- repo:
    url: https://github.com/example/repo
    ref: main
    with:
    - include: [".*"]
    - exclude: [".git/**"]
    - rename: [{"old_(.*)": "new_$1"}]
    - tools: [{name: "rustc", version: ">=1.70"}]
- include: ["src/**", "tests/**"]
- template: ["templates/**"]
"#;

        let schema = parse_original_format(yaml).expect("Failed to parse original format");

        assert_eq!(schema.len(), 3);

        // Check repo operation
        match &schema[0] {
            Operation::Repo { repo } => {
                assert_eq!(repo.url, "https://github.com/example/repo");
                assert_eq!(repo.r#ref, "main");
                assert_eq!(repo.with.len(), 4);

                // Check include in with clause (original format)
                match &repo.with[0] {
                    Operation::Include { include } => {
                        assert_eq!(include.patterns, vec![".*"]);
                    }
                    _ => panic!("Expected Include operation"),
                }

                // Check exclude in with clause (original format)
                match &repo.with[1] {
                    Operation::Exclude { exclude } => {
                        assert_eq!(exclude.patterns, vec![".git/**"]);
                    }
                    _ => panic!("Expected Exclude operation"),
                }

                // Check rename in with clause (original format)
                match &repo.with[2] {
                    Operation::Rename { rename } => {
                        assert_eq!(rename.mappings.len(), 1);
                        assert_eq!(rename.mappings[0].from, "old_(.*)");
                        assert_eq!(rename.mappings[0].to, "new_$1");
                    }
                    _ => panic!("Expected Rename operation"),
                }

                // Check tools in with clause (original format)
                match &repo.with[3] {
                    Operation::Tools { tools } => {
                        assert_eq!(tools.tools.len(), 1);
                        assert_eq!(tools.tools[0].name, "rustc");
                        assert_eq!(tools.tools[0].version, ">=1.70");
                    }
                    _ => panic!("Expected Tools operation"),
                }
            }
            _ => panic!("Expected Repo operation"),
        }

        // Check include operation (original format)
        match &schema[1] {
            Operation::Include { include } => {
                assert_eq!(include.patterns, vec!["src/**", "tests/**"]);
            }
            _ => panic!("Expected Include operation"),
        }

        // Check template operation (original format)
        match &schema[2] {
            Operation::Template { template } => {
                assert_eq!(template.patterns, vec!["templates/**"]);
            }
            _ => panic!("Expected Template operation"),
        }
    }

    #[test]
    fn test_parse_yaml_value_not_mapping() {
        // Test parsing YAML where an operation is not a mapping (covers lines 295-296)
        let yaml = r#"
- "not a mapping"
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected YAML mapping for operation"));
    }

    #[test]
    fn test_parse_empty_operation_mapping() {
        // Test parsing YAML with empty operation mapping (covers lines 307-308)
        let yaml = r#"
- {}
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty operation mapping"));
    }

    #[test]
    fn test_parse_non_string_operation_key() {
        // Test parsing YAML where operation key is not a string (covers lines 311-312)
        let yaml = r#"
- 123: value
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Operation key must be string"));
    }

    #[test]
    fn test_parse_repo_not_mapping() {
        // Test parsing YAML where repo operation is not a mapping (covers lines 322-323)
        let yaml = r#"
- repo: "not a mapping"
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Repo operation must be a mapping"));
    }

    #[test]
    fn test_parse_with_clause_items_not_mappings() {
        // Test parsing YAML where with clause items are not mappings (covers lines 357-358)
        let yaml = r#"
- repo:
    url: https://github.com/example/repo
    ref: main
    with:
      - "not a mapping"
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("With clause items must be mappings"));
    }

    #[test]
    fn test_parse_with_clause_not_sequence() {
        // Test parsing YAML where with clause is not a sequence (covers lines 365-366)
        let yaml = r#"
- repo:
    url: https://github.com/example/repo
    ref: main
    with: "not a sequence"
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("With clause must be a sequence"));
    }

    #[test]
    fn test_parse_empty_rename_mapping() {
        // Test parsing YAML with empty rename mapping (covers line 418)
        let yaml = r#"
- rename: [{}]
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty rename mapping"));
    }

    #[test]
    fn test_parse_empty_tool_specification() {
        // Test parsing YAML with empty tool specification (covers line 448)
        let yaml = r#"
- tools: [{}]
"#;
        let result = parse(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty tool specification"));
    }

    // ========================================================================
    // ArrayMergeMode Tests
    // ========================================================================

    mod array_merge_mode_tests {
        use super::*;

        #[test]
        fn test_array_merge_mode_default() {
            // Default should be Replace
            let mode = ArrayMergeMode::default();
            assert_eq!(mode, ArrayMergeMode::Replace);
        }

        #[test]
        fn test_array_merge_mode_from_append_bool_true() {
            let mode = ArrayMergeMode::from_append_bool(true);
            assert_eq!(mode, ArrayMergeMode::Append);
        }

        #[test]
        fn test_array_merge_mode_from_append_bool_false() {
            let mode = ArrayMergeMode::from_append_bool(false);
            assert_eq!(mode, ArrayMergeMode::Replace);
        }

        #[test]
        fn test_yaml_merge_op_get_array_mode_default() {
            // When both array_mode and append are default, should return Replace
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                path: None,
                append: false,
                array_mode: None,
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::Replace);
        }

        #[test]
        fn test_yaml_merge_op_get_array_mode_with_append_true() {
            // Legacy append=true should return Append
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                path: None,
                append: true,
                array_mode: None,
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::Append);
        }

        #[test]
        fn test_yaml_merge_op_get_array_mode_explicit() {
            // Explicit array_mode should override append
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                path: None,
                append: true, // Would normally be Append
                array_mode: Some(ArrayMergeMode::AppendUnique), // But explicit overrides
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::AppendUnique);
        }

        #[test]
        fn test_toml_merge_op_get_array_mode_default() {
            let op = TomlMergeOp {
                source: Some("s.toml".to_string()),
                dest: Some("d.toml".to_string()),
                path: Some("section".to_string()),
                append: false,
                preserve_comments: false,
                array_mode: None,
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::Replace);
        }

        #[test]
        fn test_toml_merge_op_get_array_mode_with_append() {
            let op = TomlMergeOp {
                source: Some("s.toml".to_string()),
                dest: Some("d.toml".to_string()),
                path: Some("section".to_string()),
                append: true,
                preserve_comments: false,
                array_mode: None,
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::Append);
        }

        #[test]
        fn test_toml_merge_op_get_array_mode_explicit() {
            let op = TomlMergeOp {
                source: Some("s.toml".to_string()),
                dest: Some("d.toml".to_string()),
                path: Some("section".to_string()),
                append: false,
                preserve_comments: false,
                array_mode: Some(ArrayMergeMode::AppendUnique),
                defer: None,
                auto_merge: None,
            };
            assert_eq!(op.get_array_mode(), ArrayMergeMode::AppendUnique);
        }
    }

    // ========================================================================
    // Merge Configuration Parsing Tests
    // ========================================================================

    mod merge_config_parsing_tests {
        use super::*;

        #[test]
        fn test_parse_yaml_merge_with_array_mode() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    path: data.items
    array_mode: append_unique
"#;
            let schema = parse(yaml).expect("Should parse YAML merge with array_mode");
            assert_eq!(schema.len(), 1);
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.source.as_deref(), Some("fragment.yaml"));
                    assert_eq!(yaml.dest.as_deref(), Some("config.yaml"));
                    assert_eq!(yaml.path, Some("data.items".to_string()));
                    assert_eq!(yaml.array_mode, Some(ArrayMergeMode::AppendUnique));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_yaml_merge_with_replace_array_mode() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    array_mode: replace
"#;
            let schema = parse(yaml).expect("Should parse YAML merge");
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.array_mode, Some(ArrayMergeMode::Replace));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_yaml_merge_with_append_array_mode() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    array_mode: append
"#;
            let schema = parse(yaml).expect("Should parse YAML merge");
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.array_mode, Some(ArrayMergeMode::Append));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_yaml_merge_defaults() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
"#;
            let schema = parse(yaml).expect("Should parse YAML merge");
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.source.as_deref(), Some("fragment.yaml"));
                    assert_eq!(yaml.dest.as_deref(), Some("config.yaml"));
                    assert_eq!(yaml.path, None);
                    assert!(!yaml.append);
                    assert_eq!(yaml.array_mode, None);
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_json_merge_with_position() {
            let yaml = r#"
- json:
    source: fragment.json
    dest: package.json
    path: dependencies
    position: start
    append: true
"#;
            let schema = parse(yaml).expect("Should parse JSON merge");
            match &schema[0] {
                Operation::Json { json } => {
                    assert_eq!(json.source.as_deref(), Some("fragment.json"));
                    assert_eq!(json.dest.as_deref(), Some("package.json"));
                    assert_eq!(json.path, Some("dependencies".to_string()));
                    assert_eq!(json.position, Some("start".to_string()));
                    assert!(json.append);
                }
                _ => panic!("Expected Json operation"),
            }
        }

        #[test]
        fn test_parse_json_merge_defaults() {
            let yaml = r#"
- json:
    source: fragment.json
    dest: package.json
"#;
            let schema = parse(yaml).expect("Should parse JSON merge");
            match &schema[0] {
                Operation::Json { json } => {
                    assert_eq!(json.path, None);
                    assert_eq!(json.position, None);
                    assert!(!json.append);
                }
                _ => panic!("Expected Json operation"),
            }
        }

        #[test]
        fn test_parse_toml_merge_with_preserve_comments() {
            let yaml = r#"
- toml:
    source: fragment.toml
    dest: Cargo.toml
    path: dependencies
    preserve-comments: true
    array_mode: append
"#;
            let schema = parse(yaml).expect("Should parse TOML merge");
            match &schema[0] {
                Operation::Toml { toml } => {
                    assert_eq!(toml.source.as_deref(), Some("fragment.toml"));
                    assert_eq!(toml.dest.as_deref(), Some("Cargo.toml"));
                    assert_eq!(toml.path.as_deref(), Some("dependencies"));
                    assert!(toml.preserve_comments);
                    assert_eq!(toml.array_mode, Some(ArrayMergeMode::Append));
                }
                _ => panic!("Expected Toml operation"),
            }
        }

        #[test]
        fn test_parse_ini_merge_with_all_options() {
            let yaml = r#"
- ini:
    source: fragment.ini
    dest: config.ini
    section: database
    append: true
    allow-duplicates: true
"#;
            let schema = parse(yaml).expect("Should parse INI merge");
            match &schema[0] {
                Operation::Ini { ini } => {
                    assert_eq!(ini.source.as_deref(), Some("fragment.ini"));
                    assert_eq!(ini.dest.as_deref(), Some("config.ini"));
                    assert_eq!(ini.section, Some("database".to_string()));
                    assert!(ini.append);
                    assert!(ini.allow_duplicates);
                }
                _ => panic!("Expected Ini operation"),
            }
        }

        #[test]
        fn test_parse_ini_merge_defaults() {
            let yaml = r#"
- ini:
    source: fragment.ini
    dest: config.ini
"#;
            let schema = parse(yaml).expect("Should parse INI merge");
            match &schema[0] {
                Operation::Ini { ini } => {
                    assert_eq!(ini.section, None);
                    assert!(!ini.append);
                    assert!(!ini.allow_duplicates);
                }
                _ => panic!("Expected Ini operation"),
            }
        }

        #[test]
        fn test_parse_markdown_merge_with_all_options() {
            let yaml = r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: "## Installation"
    append: true
    level: 3
    position: start
    create-section: true
"###;
            let schema = parse(yaml).expect("Should parse Markdown merge");
            match &schema[0] {
                Operation::Markdown { markdown } => {
                    assert_eq!(markdown.source.as_deref(), Some("fragment.md"));
                    assert_eq!(markdown.dest.as_deref(), Some("README.md"));
                    assert_eq!(markdown.section, "## Installation");
                    assert!(markdown.append);
                    assert_eq!(markdown.level, 3);
                    assert_eq!(markdown.position, "start");
                    assert!(markdown.create_section);
                }
                _ => panic!("Expected Markdown operation"),
            }
        }

        #[test]
        fn test_parse_markdown_merge_defaults() {
            let yaml = r#"
- markdown:
    source: fragment.md
    dest: README.md
    section: Features
"#;
            let schema = parse(yaml).expect("Should parse Markdown merge");
            match &schema[0] {
                Operation::Markdown { markdown } => {
                    assert!(!markdown.append);
                    assert_eq!(markdown.level, 2); // default_header_level()
                    assert_eq!(markdown.position, "");
                    assert!(!markdown.create_section);
                }
                _ => panic!("Expected Markdown operation"),
            }
        }

        #[test]
        fn test_parse_merge_ops_with_special_path_characters() {
            // Test that special characters in paths are handled correctly
            let yaml = r#"
- yaml:
    source: data.yaml
    dest: config.yaml
    path: "metadata.labels[\"special.key\"]"
"#;
            let schema = parse(yaml).expect("Should parse path with special characters");
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(
                        yaml.path,
                        Some("metadata.labels[\"special.key\"]".to_string())
                    );
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_merge_ops_with_empty_path() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    path: ""
"#;
            let schema = parse(yaml).expect("Should parse empty path");
            match &schema[0] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.path, Some("".to_string()));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_multiple_merge_operations() {
            let yaml = r#"
- yaml:
    source: yaml-fragment.yaml
    dest: config.yaml
- json:
    source: json-fragment.json
    dest: package.json
- toml:
    source: toml-fragment.toml
    dest: Cargo.toml
    path: dependencies
- ini:
    source: ini-fragment.ini
    dest: config.ini
- markdown:
    source: md-fragment.md
    dest: README.md
    section: "Features"
"#;
            let schema = parse(yaml).expect("Should parse multiple merge operations");
            assert_eq!(schema.len(), 5);
            assert!(matches!(schema[0], Operation::Yaml { .. }));
            assert!(matches!(schema[1], Operation::Json { .. }));
            assert!(matches!(schema[2], Operation::Toml { .. }));
            assert!(matches!(schema[3], Operation::Ini { .. }));
            assert!(matches!(schema[4], Operation::Markdown { .. }));
        }
    }

    // ========================================================================
    // Invalid Merge Configuration Tests
    // ========================================================================

    mod invalid_merge_config_tests {
        use super::*;

        #[test]
        fn test_parse_yaml_merge_missing_source() {
            // source/dest are now Optional, validation happens at validate() time
            let op = YamlMergeOp {
                dest: Some("config.yaml".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("source and dest"));
        }

        #[test]
        fn test_parse_yaml_merge_missing_dest() {
            let op = YamlMergeOp {
                source: Some("fragment.yaml".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("source and dest"));
        }

        #[test]
        fn test_parse_json_merge_missing_source() {
            let op = JsonMergeOp {
                dest: Some("package.json".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("source and dest"));
        }

        #[test]
        fn test_parse_json_merge_missing_dest() {
            let op = JsonMergeOp {
                source: Some("fragment.json".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("source and dest"));
        }

        #[test]
        fn test_parse_toml_merge_without_path() {
            // TOML merge path is optional - merges at root if omitted
            let yaml = r#"
- toml:
    source: fragment.toml
    dest: Cargo.toml
"#;
            let result = parse(yaml);
            assert!(result.is_ok());
            let schema = result.unwrap();
            match &schema[0] {
                Operation::Toml { toml } => {
                    assert_eq!(toml.source.as_deref(), Some("fragment.toml"));
                    assert_eq!(toml.dest.as_deref(), Some("Cargo.toml"));
                    assert_eq!(toml.path, None); // path is optional, defaults to None
                }
                _ => panic!("Expected Toml operation"),
            }
        }

        #[test]
        fn test_parse_ini_merge_missing_source() {
            let op = IniMergeOp {
                dest: Some("config.ini".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("source and dest"));
        }

        #[test]
        fn test_parse_markdown_merge_section_optional() {
            // Markdown merge section is optional (defaults to empty string)
            let yaml = r#"
- markdown:
    source: fragment.md
    dest: README.md
"#;
            let result = parse(yaml);
            assert!(result.is_ok());
            let ops = result.unwrap();
            match &ops[0] {
                Operation::Markdown { markdown } => {
                    assert_eq!(markdown.section, "");
                }
                _ => panic!("Expected Markdown operation"),
            }
        }

        #[test]
        fn test_parse_yaml_merge_invalid_array_mode() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    array_mode: invalid_mode
"#;
            let result = parse(yaml);
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_markdown_invalid_level() {
            // Level should be a number, not a string
            let yaml = r#"
- markdown:
    source: fragment.md
    dest: README.md
    section: Features
    level: "two"
"#;
            let result = parse(yaml);
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_ini_invalid_append() {
            // append should be a boolean
            let yaml = r#"
- ini:
    source: fragment.ini
    dest: config.ini
    append: "yes"
"#;
            let result = parse(yaml);
            assert!(result.is_err());
        }
    }

    // ========================================================================
    // Defer and Auto-Merge Tests
    // ========================================================================

    mod defer_auto_merge_tests {
        use super::*;

        #[test]
        fn test_auto_merge_implies_deferred() {
            let op = YamlMergeOp {
                auto_merge: Some("config.yaml".to_string()),
                ..Default::default()
            };
            assert!(op.is_deferred());
        }

        #[test]
        fn test_defer_true_is_deferred() {
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                defer: Some(true),
                ..Default::default()
            };
            assert!(op.is_deferred());
        }

        #[test]
        fn test_defer_false_not_deferred() {
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                defer: Some(false),
                ..Default::default()
            };
            assert!(!op.is_deferred());
        }

        #[test]
        fn test_no_defer_not_deferred() {
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                ..Default::default()
            };
            assert!(!op.is_deferred());
        }

        #[test]
        fn test_auto_merge_sets_source_and_dest() {
            let op = YamlMergeOp {
                auto_merge: Some("config.yaml".to_string()),
                ..Default::default()
            };
            assert_eq!(op.get_source(), Some("config.yaml"));
            assert_eq!(op.get_dest(), Some("config.yaml"));
        }

        #[test]
        fn test_auto_merge_conflicts_with_source() {
            let op = YamlMergeOp {
                auto_merge: Some("config.yaml".to_string()),
                source: Some("other.yaml".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("auto-merge"));
        }

        #[test]
        fn test_auto_merge_conflicts_with_dest() {
            let op = YamlMergeOp {
                auto_merge: Some("config.yaml".to_string()),
                dest: Some("other.yaml".to_string()),
                ..Default::default()
            };
            let result = op.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("auto-merge"));
        }

        #[test]
        fn test_auto_merge_validates_successfully() {
            let op = YamlMergeOp {
                auto_merge: Some("config.yaml".to_string()),
                ..Default::default()
            };
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_explicit_source_dest_validates_successfully() {
            let op = YamlMergeOp {
                source: Some("s.yaml".to_string()),
                dest: Some("d.yaml".to_string()),
                ..Default::default()
            };
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_json_auto_merge() {
            let op = JsonMergeOp {
                auto_merge: Some("package.json".to_string()),
                ..Default::default()
            };
            assert!(op.is_deferred());
            assert_eq!(op.get_source(), Some("package.json"));
            assert_eq!(op.get_dest(), Some("package.json"));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_toml_auto_merge() {
            let op = TomlMergeOp {
                auto_merge: Some("Cargo.toml".to_string()),
                path: Some("dependencies".to_string()),
                ..Default::default()
            };
            assert!(op.is_deferred());
            assert_eq!(op.get_source(), Some("Cargo.toml"));
            assert_eq!(op.get_dest(), Some("Cargo.toml"));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_markdown_auto_merge() {
            let op = MarkdownMergeOp {
                auto_merge: Some("CLAUDE.md".to_string()),
                section: "Rules".to_string(),
                ..Default::default()
            };
            assert!(op.is_deferred());
            assert_eq!(op.get_source(), Some("CLAUDE.md"));
            assert_eq!(op.get_dest(), Some("CLAUDE.md"));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_ini_auto_merge() {
            let op = IniMergeOp {
                auto_merge: Some("config.ini".to_string()),
                ..Default::default()
            };
            assert!(op.is_deferred());
            assert_eq!(op.get_source(), Some("config.ini"));
            assert_eq!(op.get_dest(), Some("config.ini"));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_parse_yaml_with_auto_merge() {
            let yaml = r#"
- yaml:
    auto-merge: config.yaml
    path: metadata
"#;
            let ops = parse(yaml).unwrap();
            match &ops[0] {
                Operation::Yaml { yaml: op } => {
                    assert!(op.is_deferred());
                    assert_eq!(op.get_source(), Some("config.yaml"));
                    assert_eq!(op.get_dest(), Some("config.yaml"));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_yaml_with_defer() {
            let yaml = r#"
- yaml:
    source: fragment.yaml
    dest: config.yaml
    defer: true
"#;
            let ops = parse(yaml).unwrap();
            match &ops[0] {
                Operation::Yaml { yaml: op } => {
                    assert!(op.is_deferred());
                    assert_eq!(op.get_source(), Some("fragment.yaml"));
                    assert_eq!(op.get_dest(), Some("config.yaml"));
                }
                _ => panic!("Expected Yaml operation"),
            }
        }

        #[test]
        fn test_parse_markdown_with_auto_merge() {
            let yaml =
                "- markdown:\n    auto-merge: CLAUDE.md\n    section: Rules\n    append: true\n";
            let ops = parse(yaml).unwrap();
            match &ops[0] {
                Operation::Markdown { markdown: op } => {
                    assert!(op.is_deferred());
                    assert_eq!(op.get_source(), Some("CLAUDE.md"));
                }
                _ => panic!("Expected Markdown operation"),
            }
        }

        // Tests for Operation::is_deferred() method
        #[test]
        fn test_operation_is_deferred_yaml() {
            let op = Operation::Yaml {
                yaml: YamlMergeOp {
                    auto_merge: Some("config.yaml".to_string()),
                    ..Default::default()
                },
            };
            assert!(op.is_deferred());
        }

        #[test]
        fn test_operation_is_deferred_json() {
            let op = Operation::Json {
                json: JsonMergeOp {
                    defer: Some(true),
                    source: Some("s.json".to_string()),
                    dest: Some("d.json".to_string()),
                    ..Default::default()
                },
            };
            assert!(op.is_deferred());
        }

        #[test]
        fn test_operation_is_deferred_non_merge_op() {
            // Include operations cannot be deferred
            let op = Operation::Include {
                include: IncludeOp {
                    patterns: vec!["*.rs".to_string()],
                },
            };
            assert!(!op.is_deferred());
        }

        #[test]
        fn test_operation_is_deferred_exclude_op() {
            // Exclude operations cannot be deferred
            let op = Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            };
            assert!(!op.is_deferred());
        }

        #[test]
        fn test_operation_is_deferred_repo_op() {
            // Repo operations cannot be deferred
            let op = Operation::Repo {
                repo: RepoOp {
                    url: "https://example.com/repo".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            };
            assert!(!op.is_deferred());
        }

        #[test]
        fn test_operation_is_deferred_merge_without_defer() {
            // Merge op without defer flag should not be deferred
            let op = Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("s.yaml".to_string()),
                    dest: Some("d.yaml".to_string()),
                    ..Default::default()
                },
            };
            assert!(!op.is_deferred());
        }
    }

    // ========================================================================
    // Builder Pattern Tests
    // ========================================================================

    mod builder_tests {
        use super::*;

        #[test]
        fn test_yaml_merge_op_builder() {
            let op = YamlMergeOp::new()
                .source("fragment.yaml")
                .dest("config.yaml")
                .path("metadata.labels")
                .array_mode(ArrayMergeMode::AppendUnique)
                .defer(true);

            assert_eq!(op.source.as_deref(), Some("fragment.yaml"));
            assert_eq!(op.dest.as_deref(), Some("config.yaml"));
            assert_eq!(op.path.as_deref(), Some("metadata.labels"));
            assert_eq!(op.array_mode, Some(ArrayMergeMode::AppendUnique));
            assert_eq!(op.defer, Some(true));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_yaml_merge_op_builder_auto_merge() {
            let op = YamlMergeOp::new()
                .auto_merge("config.yaml")
                .path("metadata");

            assert!(op.is_deferred());
            assert_eq!(op.get_source(), Some("config.yaml"));
            assert_eq!(op.get_dest(), Some("config.yaml"));
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_json_merge_op_builder() {
            let op = JsonMergeOp::new()
                .source("fragment.json")
                .dest("package.json")
                .path("dependencies")
                .position("end")
                .append(true);

            assert_eq!(op.source.as_deref(), Some("fragment.json"));
            assert_eq!(op.dest.as_deref(), Some("package.json"));
            assert_eq!(op.path.as_deref(), Some("dependencies"));
            assert_eq!(op.position.as_deref(), Some("end"));
            assert!(op.append);
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_toml_merge_op_builder() {
            let op = TomlMergeOp::new()
                .source("fragment.toml")
                .dest("Cargo.toml")
                .path("dependencies")
                .array_mode(ArrayMergeMode::Append)
                .preserve_comments(true);

            assert_eq!(op.source.as_deref(), Some("fragment.toml"));
            assert_eq!(op.dest.as_deref(), Some("Cargo.toml"));
            assert_eq!(op.path.as_deref(), Some("dependencies"));
            assert_eq!(op.array_mode, Some(ArrayMergeMode::Append));
            assert!(op.preserve_comments);
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_ini_merge_op_builder() {
            let op = IniMergeOp::new()
                .source("fragment.ini")
                .dest("config.ini")
                .section("database")
                .append(true)
                .allow_duplicates(true);

            assert_eq!(op.source.as_deref(), Some("fragment.ini"));
            assert_eq!(op.dest.as_deref(), Some("config.ini"));
            assert_eq!(op.section.as_deref(), Some("database"));
            assert!(op.append);
            assert!(op.allow_duplicates);
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_markdown_merge_op_builder() {
            let op = MarkdownMergeOp::new()
                .source("fragment.md")
                .dest("README.md")
                .section("Installation")
                .level(3)
                .position("end")
                .append(true)
                .create_section(true);

            assert_eq!(op.source.as_deref(), Some("fragment.md"));
            assert_eq!(op.dest.as_deref(), Some("README.md"));
            assert_eq!(op.section, "Installation");
            assert_eq!(op.level, 3);
            assert_eq!(op.position, "end");
            assert!(op.append);
            assert!(op.create_section);
            assert!(op.validate().is_ok());
        }

        #[test]
        fn test_builder_chain_validates() {
            // Ensure builders create valid configurations
            let yaml = YamlMergeOp::new().source("s.yaml").dest("d.yaml");
            assert!(yaml.validate().is_ok());

            let json = JsonMergeOp::new().source("s.json").dest("d.json");
            assert!(json.validate().is_ok());

            let toml = TomlMergeOp::new().source("s.toml").dest("d.toml");
            assert!(toml.validate().is_ok());

            let ini = IniMergeOp::new().source("s.ini").dest("d.ini");
            assert!(ini.validate().is_ok());

            let md = MarkdownMergeOp::new()
                .source("s.md")
                .dest("d.md")
                .section("Test");
            assert!(md.validate().is_ok());
        }

        #[test]
        fn test_builder_new_returns_default() {
            // Ensure new() returns the same as Default::default()
            let yaml_new = YamlMergeOp::new();
            let yaml_default = YamlMergeOp::default();
            assert_eq!(yaml_new, yaml_default);

            let json_new = JsonMergeOp::new();
            let json_default = JsonMergeOp::default();
            assert_eq!(json_new, json_default);

            let toml_new = TomlMergeOp::new();
            let toml_default = TomlMergeOp::default();
            assert_eq!(toml_new, toml_default);

            let ini_new = IniMergeOp::new();
            let ini_default = IniMergeOp::default();
            assert_eq!(ini_new, ini_default);

            let md_new = MarkdownMergeOp::new();
            let md_default = MarkdownMergeOp::default();
            assert_eq!(md_new, md_default);
        }
    }
}
