//! Configuration schema and parsing for .common-repo.yaml files

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a tool requirement with version constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g., "pre-commit", "rustc")
    pub name: String,
    /// Version constraint (e.g., "*", ">=1.70", "^3.9")
    pub version: String,
}

/// Template variables context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVars {
    /// Variable name to value mapping
    pub vars: HashMap<String, String>,
}

/// Repo operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoOp {
    /// Repository URL
    pub url: String,
    /// Git reference (branch, tag, commit)
    pub r#ref: String,
    /// Optional inline operations to apply to this repo
    #[serde(default)]
    pub with: Vec<Operation>,
}

/// Include operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeOp {
    /// Glob patterns to include
    pub patterns: Vec<String>,
}

/// Exclude operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeOp {
    /// Glob patterns to exclude
    pub patterns: Vec<String>,
}

/// Template operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateOp {
    /// Glob patterns for template files
    pub patterns: Vec<String>,
}

/// Rename operation mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameMapping {
    /// Regex pattern to match
    pub from: String,
    /// Replacement pattern with %[N] placeholders
    pub to: String,
}

/// Rename operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameOp {
    /// List of rename mappings
    pub mappings: Vec<RenameMapping>,
}

/// Tools operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsOp {
    /// List of required tools
    pub tools: Vec<Tool>,
}

/// YAML merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Path within the destination to merge at
    pub path: String,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
}

/// JSON merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Path within the destination to merge at
    pub path: String,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Position for appending ("end" or "start")
    #[serde(default)]
    pub position: String,
}

/// TOML merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Path within the destination to merge at
    pub path: String,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Whether to preserve comments
    #[serde(default)]
    pub preserve_comments: bool,
}

/// INI merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IniMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Section to merge into
    pub section: String,
    /// Whether to append (true) or replace (false)
    #[serde(default)]
    pub append: bool,
    /// Whether to allow duplicate keys
    #[serde(default)]
    pub allow_duplicates: bool,
}

/// Markdown merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Section header to merge under
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
    #[serde(default)]
    pub create_section: bool,
}

fn default_header_level() -> u8 {
    2
}

/// All possible operation types in the configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    /// Pull files from an inherited repository
    Repo { repo: RepoOp },
    /// Include files from current repository
    Include { include: IncludeOp },
    /// Exclude files from filesystem
    Exclude { exclude: ExcludeOp },
    /// Mark files as templates
    Template { template: TemplateOp },
    /// Rename files using regex patterns
    Rename { rename: RenameOp },
    /// Declare required tools
    Tools { tools: ToolsOp },
    /// Set template variables
    TemplateVars { template_vars: TemplateVars },
    /// Merge YAML fragments
    Yaml { yaml: YamlMergeOp },
    /// Merge JSON fragments
    Json { json: JsonMergeOp },
    /// Merge TOML fragments
    Toml { toml: TomlMergeOp },
    /// Merge INI fragments
    Ini { ini: IniMergeOp },
    /// Merge Markdown fragments
    Markdown { markdown: MarkdownMergeOp },
}

/// The complete configuration schema (list of operations)
#[allow(dead_code)]
pub type Schema = Vec<Operation>;

/// Parse a YAML string into a Schema
#[allow(dead_code)]
pub fn parse(yaml_content: &str) -> Result<Schema> {
    serde_yaml::from_str(yaml_content).map_err(Error::Yaml)
}

/// Parse a Schema from a YAML file path
#[allow(dead_code)]
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
    patterns: ["**/*", "*.md"]
- exclude:
    patterns: [".git/**", "*.tmp"]
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
          patterns: ["src/**/*"]
- include:
    patterns: ["**/*.md", "docs/**"]
- exclude:
    patterns: [".git/**", "*.tmp"]
- template:
    patterns: ["*.template"]
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
                patterns: ["src/**"]
      - exclude:
          patterns: ["tests/**"]
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
}
