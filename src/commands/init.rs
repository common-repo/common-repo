//! # Init Command Implementation
//!
//! This module implements the `init` subcommand, which creates new `.common-repo.yaml`
//! configuration files with different initialization modes.
//!
//! ## Functionality
//!
//! - **Minimal Config**: Creates a basic configuration with example operations and comments
//! - **Empty Config**: Creates an empty configuration file with minimal structure
//! - **Template Config**: Uses predefined templates for common tech stacks
//! - **Interactive Config**: Guides users through configuration creation via CLI wizard
//! - **Force Mode**: Overwrites existing configuration files when specified

use anyhow::Result;
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Input};
use std::fs;
use std::path::Path;

use common_repo::git;
use common_repo::version;

/// Initialize a new .common-repo.yaml configuration file
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Interactive setup wizard for configuration creation
    #[arg(short, long)]
    pub interactive: bool,

    /// Start from a predefined template (e.g., rust-cli, python-django)
    #[arg(short, long, value_name = "TEMPLATE")]
    pub template: Option<String>,

    /// Create minimal configuration with examples (default)
    #[arg(long)]
    pub minimal: bool,

    /// Create empty configuration file
    #[arg(long)]
    pub empty: bool,

    /// Overwrite existing configuration file
    #[arg(short, long)]
    pub force: bool,
}

/// Execute the `init` command.
///
/// This function handles the logic for the `init` subcommand, creating
/// `.common-repo.yaml` files based on the specified initialization mode.
pub fn execute(args: InitArgs) -> Result<()> {
    let config_path = Path::new(".common-repo.yaml");

    // Check if config file already exists
    if config_path.exists() && !args.force {
        return Err(anyhow::anyhow!(
            "Configuration file '.common-repo.yaml' already exists. Use --force to overwrite."
        ));
    }

    println!("🎯 Initializing common-repo configuration...");

    let config_content = if args.empty {
        generate_empty_config()
    } else if let Some(template) = &args.template {
        generate_template_config(template)?
    } else if args.interactive {
        generate_interactive_config()?
    } else {
        // Default to minimal
        generate_minimal_config()
    };

    // Write the configuration file
    fs::write(config_path, config_content)?;
    println!("✅ Created .common-repo.yaml");
    println!("💡 Run `common-repo apply` to fetch and apply configurations");

    Ok(())
}

/// Generate an empty configuration file.
fn generate_empty_config() -> String {
    r#"# common-repo configuration
# This file defines which repository configurations to inherit

# Add your repository configurations here
# See https://github.com/common-repo/common-repo for documentation

"#
    .to_string()
}

/// Generate a minimal configuration with examples and comments.
fn generate_minimal_config() -> String {
    r#"# common-repo configuration
# This file defines which repository configurations to inherit

# Example: Inherit from a common Rust CLI setup
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v1.2.0

# Include specific files from inherited repos
- include:
    patterns:
      - "**/*"

# Exclude files you don't want
- exclude:
    patterns:
      - "**/*.md"

# Example: Template processing
- template:
    patterns:
      - "**/*.template"

- template-vars:
    project_name: ${PROJECT_NAME:-my-project}
    author: ${AUTHOR:-Your Name}

# Example: Merge configuration fragments
- yaml:
    source: config.yml
    dest: .github/workflows/ci.yml
    path: jobs.test.steps
    append: true

"#
    .to_string()
}

/// Generate configuration from a predefined template.
fn generate_template_config(template_name: &str) -> Result<String> {
    match template_name {
        "rust-cli" => Ok(generate_rust_cli_template()),
        "python-django" => Ok(generate_python_django_template()),
        "node-typescript" => Ok(generate_node_typescript_template()),
        "go-service" => Ok(generate_go_service_template()),
        _ => Err(anyhow::anyhow!(
            "Unknown template '{}'. Available templates: rust-cli, python-django, node-typescript, go-service",
            template_name
        )),
    }
}

/// A repository entry collected during the interactive wizard.
struct RepoEntry {
    url: String,
    version: String,
}

/// Generate interactive configuration through CLI wizard.
fn generate_interactive_config() -> Result<String> {
    let theme = ColorfulTheme::default();

    println!();
    println!("Welcome to common-repo!");
    println!("Enter repository URLs to inherit from. Leave empty when done.");
    println!();

    let mut repos: Vec<RepoEntry> = Vec::new();

    loop {
        let prompt = if repos.is_empty() {
            "Repository URL (e.g., https://github.com/org/repo)"
        } else {
            "Add another repository (leave empty to finish)"
        };

        let input: String = Input::with_theme(&theme)
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()?;

        let input = input.trim();
        if input.is_empty() {
            if repos.is_empty() {
                println!("No repositories added. Creating empty configuration.");
            }
            break;
        }

        // Normalize URL
        let url = normalize_repo_url(input);

        // Fetch tags and find latest semver
        print!("  Fetching tags from {}... ", url);
        match git::list_tags(&url) {
            Ok(tags) => {
                let semver_tags = version::filter_semver_tags(&tags);
                if let Some((latest_tag, _)) = find_latest_version(&semver_tags) {
                    println!("found {}", latest_tag);
                    repos.push(RepoEntry {
                        url: url.clone(),
                        version: latest_tag,
                    });
                } else if !tags.is_empty() {
                    // No semver tags, warn and use first tag or main
                    println!("no semver tags found");
                    println!("  Warning: No semantic version tags found. Using 'main' branch.");
                    repos.push(RepoEntry {
                        url: url.clone(),
                        version: "main".to_string(),
                    });
                } else {
                    println!("no tags found");
                    println!("  Warning: No tags found. Using 'main' branch.");
                    repos.push(RepoEntry {
                        url: url.clone(),
                        version: "main".to_string(),
                    });
                }
            }
            Err(e) => {
                println!("failed");
                println!("  Error fetching tags: {}. Using 'main' branch.", e);
                repos.push(RepoEntry {
                    url: url.clone(),
                    version: "main".to_string(),
                });
            }
        }
    }

    Ok(build_config_from_repos(&repos))
}

/// Normalize a repository URL, expanding GitHub shorthand.
fn normalize_repo_url(input: &str) -> String {
    // If it already looks like a URL, use as-is
    if input.starts_with("https://") || input.starts_with("git@") || input.starts_with("http://") {
        return input.to_string();
    }

    // Expand GitHub shorthand: org/repo -> https://github.com/org/repo
    if input.contains('/') && !input.contains(':') {
        return format!("https://github.com/{}", input);
    }

    input.to_string()
}

/// Find the latest semantic version from a list of tags.
fn find_latest_version(tags: &[String]) -> Option<(String, semver::Version)> {
    let mut latest: Option<(String, semver::Version)> = None;

    for tag in tags {
        if let Some(version) = git::parse_semver_tag(tag) {
            if let Some((_, ref latest_ver)) = latest {
                if version > *latest_ver {
                    latest = Some((tag.clone(), version));
                }
            } else {
                latest = Some((tag.clone(), version));
            }
        }
    }

    latest
}

/// Build configuration YAML from collected repository entries.
fn build_config_from_repos(repos: &[RepoEntry]) -> String {
    let mut config = String::from("# common-repo configuration\n");
    config.push_str("# Generated by interactive wizard\n\n");

    if repos.is_empty() {
        config.push_str("# Add your repository configurations here:\n");
        config.push_str("# - repo:\n");
        config.push_str("#     url: https://github.com/your-org/your-repo\n");
        config.push_str("#     ref: v1.0.0\n");
    } else {
        for repo in repos {
            config.push_str("- repo:\n");
            config.push_str(&format!("    url: {}\n", repo.url));
            config.push_str(&format!("    ref: {}\n", repo.version));
            config.push('\n');
        }

        config.push_str("- include:\n");
        config.push_str("    patterns:\n");
        config.push_str("      - \"**/*\"\n");
    }

    config
}

/// Generate Rust CLI template configuration.
fn generate_rust_cli_template() -> String {
    r#"# Rust CLI Application Configuration
# Inherits common Rust CLI project setup

- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v1.2.0

- repo:
    url: https://github.com/common-repo/ci-rust
    ref: v2.1.0

- repo:
    url: https://github.com/common-repo/pre-commit-hooks
    ref: v1.5.0

- include:
    patterns:
      - "**/*"

- exclude:
    patterns:
      - "**/*.md"
      - "docs/**"

- template:
    patterns:
      - "**/*.template"

- template-vars:
    project_name: ${PROJECT_NAME:-my-rust-cli}
    author: ${AUTHOR:-Your Name}
    rust_version: ${RUST_VERSION:-1.70}

"#
    .to_string()
}

/// Generate Python Django template configuration.
fn generate_python_django_template() -> String {
    r#"# Python Django Application Configuration
# Inherits common Python Django project setup

- repo:
    url: https://github.com/common-repo/python-django
    ref: v2.0.0

- repo:
    url: https://github.com/common-repo/ci-python
    ref: v1.8.0

- repo:
    url: https://github.com/common-repo/pre-commit-hooks
    ref: v1.5.0

- include:
    patterns:
      - "**/*"

- exclude:
    patterns:
      - "**/*.md"
      - "docs/**"

- template:
    patterns:
      - "**/*.template"

- template-vars:
    project_name: ${PROJECT_NAME:-my-django-app}
    author: ${AUTHOR:-Your Name}
    python_version: ${PYTHON_VERSION:-3.9}
    django_version: ${DJANGO_VERSION:-4.2}

"#
    .to_string()
}

/// Generate Node.js TypeScript template configuration.
fn generate_node_typescript_template() -> String {
    r#"# Node.js TypeScript Application Configuration
# Inherits common Node.js TypeScript project setup

- repo:
    url: https://github.com/common-repo/node-typescript
    ref: v1.9.0

- repo:
    url: https://github.com/common-repo/ci-node
    ref: v1.7.0

- repo:
    url: https://github.com/common-repo/pre-commit-hooks
    ref: v1.5.0

- include:
    patterns:
      - "**/*"

- exclude:
    patterns:
      - "**/*.md"
      - "docs/**"

- template:
    patterns:
      - "**/*.template"

- template-vars:
    project_name: ${PROJECT_NAME:-my-node-app}
    author: ${AUTHOR:-Your Name}
    node_version: ${NODE_VERSION:-18}
    typescript_version: ${TYPESCRIPT_VERSION:-5.0}

"#
    .to_string()
}

/// Generate Go service template configuration.
fn generate_go_service_template() -> String {
    r#"# Go Service Configuration
# Inherits common Go service project setup

- repo:
    url: https://github.com/common-repo/go-service
    ref: v1.6.0

- repo:
    url: https://github.com/common-repo/ci-go
    ref: v1.4.0

- repo:
    url: https://github.com/common-repo/pre-commit-hooks
    ref: v1.5.0

- include:
    patterns:
      - "**/*"

- exclude:
    patterns:
      - "**/*.md"
      - "docs/**"

- template:
    patterns:
      - "**/*.template"

- template-vars:
    project_name: ${PROJECT_NAME:-my-go-service}
    author: ${AUTHOR:-Your Name}
    go_version: ${GO_VERSION:-1.20}

"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_generate_empty_config() {
        let config = generate_empty_config();
        assert!(config.contains("# common-repo configuration"));
        assert!(config.contains("# Add your repository configurations here"));
    }

    #[test]
    fn test_generate_minimal_config() {
        let config = generate_minimal_config();
        assert!(config.contains("# common-repo configuration"));
        assert!(config.contains("repo:"));
        assert!(config.contains("include:"));
        assert!(config.contains("exclude:"));
        assert!(config.contains("template:"));
        assert!(config.contains("template-vars:"));
    }

    #[test]
    fn test_generate_template_config_rust_cli() {
        let config = generate_template_config("rust-cli").unwrap();
        assert!(config.contains("rust-cli"));
        assert!(config.contains("ci-rust"));
        assert!(config.contains("pre-commit-hooks"));
    }

    #[test]
    fn test_generate_template_config_python_django() {
        let config = generate_template_config("python-django").unwrap();
        assert!(config.contains("python-django"));
        assert!(config.contains("ci-python"));
    }

    #[test]
    fn test_generate_template_config_unknown() {
        let result = generate_template_config("unknown-template");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown template"));
    }

    #[test]
    #[serial]
    fn test_execute_force_flag() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Create existing config file
        fs::write(".common-repo.yaml", "existing content").unwrap();

        // Try to init without force - should fail
        let args = InitArgs {
            interactive: false,
            template: None,
            minimal: true,
            empty: false,
            force: false,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // Try with force - should succeed
        let args = InitArgs {
            interactive: false,
            template: None,
            minimal: true,
            empty: false,
            force: true,
        };

        let result = execute(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(".common-repo.yaml").unwrap();
        assert!(content.contains("# common-repo configuration"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_execute_empty_config() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let args = InitArgs {
            interactive: false,
            template: None,
            minimal: false,
            empty: true,
            force: false,
        };

        let result = execute(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(".common-repo.yaml").unwrap();
        assert!(content.contains("# common-repo configuration"));
        assert!(content.contains("# Add your repository configurations here"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_execute_minimal_config() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let args = InitArgs {
            interactive: false,
            template: None,
            minimal: true,
            empty: false,
            force: false,
        };

        let result = execute(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(".common-repo.yaml").unwrap();
        assert!(content.contains("repo:"));
        assert!(content.contains("include:"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_execute_template_config() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let args = InitArgs {
            interactive: false,
            template: Some("rust-cli".to_string()),
            minimal: false,
            empty: false,
            force: false,
        };

        let result = execute(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(".common-repo.yaml").unwrap();
        assert!(content.contains("rust-cli"));
        assert!(content.contains("ci-rust"));

        env::set_current_dir(original_dir).unwrap();
    }

    // Note: test_execute_interactive_config is skipped because dialoguer
    // requires a TTY for interactive prompts. Interactive mode is tested
    // manually or via E2E tests with TTY simulation.

    #[test]
    fn test_normalize_repo_url_full_url() {
        assert_eq!(
            normalize_repo_url("https://github.com/org/repo"),
            "https://github.com/org/repo"
        );
        assert_eq!(
            normalize_repo_url("git@github.com:org/repo.git"),
            "git@github.com:org/repo.git"
        );
    }

    #[test]
    fn test_normalize_repo_url_shorthand() {
        assert_eq!(
            normalize_repo_url("org/repo"),
            "https://github.com/org/repo"
        );
        assert_eq!(
            normalize_repo_url("common-repo/rust-cli"),
            "https://github.com/common-repo/rust-cli"
        );
    }

    #[test]
    fn test_build_config_from_repos_empty() {
        let repos: Vec<RepoEntry> = vec![];
        let config = build_config_from_repos(&repos);
        assert!(config.contains("# common-repo configuration"));
        assert!(config.contains("# Add your repository configurations here"));
    }

    #[test]
    fn test_build_config_from_repos_single() {
        let repos = vec![RepoEntry {
            url: "https://github.com/org/repo".to_string(),
            version: "v1.0.0".to_string(),
        }];
        let config = build_config_from_repos(&repos);
        assert!(config.contains("url: https://github.com/org/repo"));
        assert!(config.contains("ref: v1.0.0"));
        assert!(config.contains("- include:"));
    }

    #[test]
    fn test_build_config_from_repos_multiple() {
        let repos = vec![
            RepoEntry {
                url: "https://github.com/org/repo1".to_string(),
                version: "v1.0.0".to_string(),
            },
            RepoEntry {
                url: "https://github.com/org/repo2".to_string(),
                version: "v2.0.0".to_string(),
            },
        ];
        let config = build_config_from_repos(&repos);
        assert!(config.contains("url: https://github.com/org/repo1"));
        assert!(config.contains("ref: v1.0.0"));
        assert!(config.contains("url: https://github.com/org/repo2"));
        assert!(config.contains("ref: v2.0.0"));
    }

    #[test]
    fn test_find_latest_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v2.0.0".to_string(),
            "v1.5.0".to_string(),
        ];
        let result = super::find_latest_version(&tags);
        assert!(result.is_some());
        let (tag, _) = result.unwrap();
        assert_eq!(tag, "v2.0.0");
    }

    #[test]
    fn test_find_latest_version_empty() {
        let tags: Vec<String> = vec![];
        let result = super::find_latest_version(&tags);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_latest_version_no_semver() {
        let tags = vec!["main".to_string(), "develop".to_string()];
        let result = super::find_latest_version(&tags);
        assert!(result.is_none());
    }
}
