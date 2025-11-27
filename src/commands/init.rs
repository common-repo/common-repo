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
use std::fs;
use std::path::Path;

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

/// Generate interactive configuration through CLI wizard.
fn generate_interactive_config() -> Result<String> {
    println!("🎉 Welcome to common-repo!");
    println!("Let's set up your repository configuration.");
    println!();

    // For now, return minimal config
    // TODO: Implement full interactive wizard with dialoguer
    println!("? What type of project is this?");
    println!("  ❯ Rust CLI application");
    println!("    Python web application");
    println!("    Node.js/TypeScript project");
    println!("    Go service");
    println!("    Custom/Other");
    println!();

    println!("? Which common configurations do you want?");
    println!("  ◉ Pre-commit hooks");
    println!("  ◉ CI/CD workflows (GitHub Actions)");
    println!("  ◯ Semantic versioning");
    println!("  ◉ Linters and formatters");
    println!();

    println!("? Pin to stable versions or track latest?");
    println!("  ❯ Stable (recommended)");
    println!("    Latest (auto-update to newest)");
    println!("    Custom");
    println!();

    // Return minimal config for now
    Ok(generate_minimal_config())
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

    #[test]
    #[serial]
    fn test_execute_interactive_config() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let args = InitArgs {
            interactive: true,
            template: None,
            minimal: false,
            empty: false,
            force: false,
        };

        let result = execute(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(".common-repo.yaml").unwrap();
        assert!(content.contains("# common-repo configuration"));

        env::set_current_dir(original_dir).unwrap();
    }
}
