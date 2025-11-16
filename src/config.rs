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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeOp {
    /// A list of glob patterns specifying the files to include.
    pub patterns: Vec<String>,
}

/// Exclude operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeOp {
    /// A list of glob patterns specifying the files to exclude.
    pub patterns: Vec<String>,
}

/// Template operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateOp {
    /// A list of glob patterns specifying the files to mark as templates.
    pub patterns: Vec<String>,
}

/// Rename operation mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameMapping {
    /// A regular expression used to match file paths.
    pub from: String,
    /// A replacement pattern that can include capture groups from the `from`
    /// regex (e.g., `$1`, `$2`).
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
    /// A list of required tools and their version constraints.
    pub tools: Vec<Tool>,
}

////// CONVERSION IMPLEMENTATIONS //////

/// YAML merge operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlMergeOp {
    /// Source fragment file
    pub source: String,
    /// Destination file to merge into
    pub dest: String,
    /// Path within the destination to merge at (optional - merges at root if omitted)
    #[serde(default)]
    pub path: Option<String>,
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
    #[serde(default, rename = "preserve-comments")]
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
    #[serde(default, rename = "allow-duplicates")]
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
    #[serde(default, rename = "create-section")]
    pub create_section: bool,
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

/// All possible operation types in the configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// The complete configuration schema, represented as a list of operations.
///
/// The operations are executed in the order they are defined in the file.
#[allow(dead_code)]
pub type Schema = Vec<Operation>;

/// Parses a YAML string into a `Schema`.
///
/// This function supports both the current, more structured format and the
/// original, more concise format for backward compatibility. It will first
/// attempt to parse as the current format, and if that fails, it will fall back
/// to the original format parser.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    })?;

    let op_type = key.as_str().ok_or_else(|| Error::ConfigParse {
        message: "Operation key must be string".to_string(),
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
                    });
                }
            };

            let url = repo_map
                .remove(serde_yaml::Value::String("url".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .ok_or_else(|| Error::ConfigParse {
                    message: "Repo operation missing url".to_string(),
                })?;

            let r#ref = repo_map
                .remove(serde_yaml::Value::String("ref".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .ok_or_else(|| Error::ConfigParse {
                    message: "Repo operation missing ref".to_string(),
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
                                });
                            }
                        }
                        with_ops
                    }
                    _ => {
                        return Err(Error::ConfigParse {
                            message: "With clause must be a sequence".to_string(),
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
        }),
    }
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
                assert_eq!(yaml.source, "fragment.yml");
                assert_eq!(yaml.dest, "config.yml");
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
                assert_eq!(json.source, "fragment.json");
                assert_eq!(json.dest, "package.json");
                assert_eq!(json.path, "dependencies");
                assert!(json.append);
                assert_eq!(json.position, "end".to_string());
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
                assert_eq!(toml.source, "fragment.toml");
                assert_eq!(toml.dest, "Cargo.toml");
                assert_eq!(toml.path, "dependencies");
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
                assert_eq!(ini.source, "fragment.ini");
                assert_eq!(ini.dest, "config.ini");
                assert_eq!(ini.section, "database");
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
                assert_eq!(markdown.source, "fragment.md");
                assert_eq!(markdown.dest, "README.md");
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
}
