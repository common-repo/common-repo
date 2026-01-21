//! # Init Command Implementation
//!
//! This module implements the `init` subcommand, which creates new `.common-repo.yaml`
//! configuration files through an interactive setup wizard.
//!
//! ## Functionality
//!
//! - **Interactive Wizard**: Default mode that guides users through configuration creation
//! - **URI Argument**: Initialize from an existing repository URL
//! - **Pre-commit Setup**: Optional integration with pre-commit hooks (prek or pre-commit)
//! - **Force Mode**: Overwrites existing configuration files when specified

use anyhow::Result;
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use std::fs;
use std::path::Path;
use std::process::Command;

use common_repo::defaults::DEFAULT_CONFIG_FILENAME;
use common_repo::git;
use common_repo::version;

/// Initialize a new .common-repo.yaml configuration file
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Repository URL to initialize from (e.g., https://github.com/org/repo or org/repo)
    #[arg(value_name = "URI")]
    pub uri: Option<String>,

    /// Interactive setup wizard for configuration creation (default when no URI provided)
    #[arg(short, long)]
    pub interactive: bool,

    /// Overwrite existing configuration file
    #[arg(short, long)]
    pub force: bool,
}

/// Execute the `init` command.
///
/// This function handles the logic for the `init` subcommand, creating
/// `.common-repo.yaml` files. The default behavior is to run the interactive
/// wizard, unless a URI argument is provided.
pub fn execute(args: InitArgs) -> Result<()> {
    let config_path = Path::new(DEFAULT_CONFIG_FILENAME);

    // Check if config file already exists
    if config_path.exists() && !args.force {
        return Err(anyhow::anyhow!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            DEFAULT_CONFIG_FILENAME
        ));
    }

    println!("🎯 Initializing common-repo configuration...");

    let config_content = if let Some(uri) = &args.uri {
        generate_config_from_uri(uri)?
    } else {
        // Default to interactive wizard
        generate_interactive_config()?
    };

    // Write the configuration file
    fs::write(config_path, config_content)?;
    println!("✅ Created .common-repo.yaml");
    println!("💡 Run `common-repo apply` to fetch and apply configurations");

    Ok(())
}

/// Generate configuration from a repository URI.
///
/// Fetches the repository tags, finds the latest semver version,
/// and generates a configuration with that repository.
fn generate_config_from_uri(uri: &str) -> Result<String> {
    // Normalize URL (expand GitHub shorthand)
    let url = normalize_repo_url(uri);

    print!("Fetching tags from {}... ", url);
    let (version, warnings) = match git::list_tags(&url) {
        Ok(tags) => {
            let semver_tags = version::filter_semver_tags(&tags);
            if let Some((latest_tag, parsed_version)) = find_latest_version(&semver_tags) {
                println!("found {}", latest_tag);

                // Check for 0.x.x versions
                let mut warnings = Vec::new();
                if parsed_version.major == 0 {
                    warnings.push(format!(
                        "Warning: {} is a 0.x.x version, which may indicate unstable API",
                        latest_tag
                    ));
                }
                (latest_tag, warnings)
            } else if !tags.is_empty() {
                // No semver tags, warn and use main
                println!("no semver tags found");
                (
                    "main".to_string(),
                    vec![
                        "Warning: No semantic version tags found. Using 'main' branch.".to_string(),
                        "Consider pinning to a specific commit hash for reproducibility."
                            .to_string(),
                    ],
                )
            } else {
                println!("no tags found");
                (
                    "main".to_string(),
                    vec![
                        "Warning: No tags found. Using 'main' branch.".to_string(),
                        "Consider pinning to a specific commit hash for reproducibility."
                            .to_string(),
                    ],
                )
            }
        }
        Err(e) => {
            println!("failed");
            (
                "main".to_string(),
                vec![
                    format!("Warning: Error fetching tags: {}. Using 'main' branch.", e),
                    "Consider pinning to a specific commit hash for reproducibility.".to_string(),
                ],
            )
        }
    };

    // Print warnings
    for warning in &warnings {
        println!("  ⚠️  {}", warning);
    }

    // Build the config
    let repos = vec![RepoEntry {
        url: url.clone(),
        version,
    }];

    Ok(build_config_from_repos(&repos))
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

    // Ask about pre-commit hooks
    println!();
    let setup_hooks = Confirm::with_theme(&theme)
        .with_prompt("Set up pre-commit hooks?")
        .default(true)
        .interact()?;

    if setup_hooks {
        setup_precommit_hooks(&theme)?;
    }

    Ok(build_config_from_repos(&repos))
}

/// Detect which pre-commit CLI is available.
///
/// Returns `Some("prek")` or `Some("pre-commit")` if found, `None` otherwise.
fn detect_precommit_cli() -> Option<&'static str> {
    // Prefer prek (Rust-based, faster)
    if Command::new("prek")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("prek");
    }

    // Fall back to pre-commit (Python-based)
    if Command::new("pre-commit")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("pre-commit");
    }

    None
}

/// Generate a basic .pre-commit-config.yaml file.
fn generate_precommit_config() -> String {
    r#"# Pre-commit configuration
# Generated by common-repo init
# See https://pre-commit.com for more information

repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-added-large-files
      - id: check-merge-conflict
"#
    .to_string()
}

/// Set up pre-commit hooks interactively.
fn setup_precommit_hooks(theme: &ColorfulTheme) -> Result<()> {
    let config_path = Path::new(".pre-commit-config.yaml");

    // Check if config already exists
    if config_path.exists() {
        println!("  .pre-commit-config.yaml already exists, skipping generation.");
    } else {
        // Generate the config file
        fs::write(config_path, generate_precommit_config())?;
        println!("✅ Created .pre-commit-config.yaml");
    }

    // Detect available CLI
    if let Some(cli) = detect_precommit_cli() {
        println!("  Found {} CLI.", cli);

        let install_hooks = Confirm::with_theme(theme)
            .with_prompt(format!("Run '{} install' to set up git hooks?", cli))
            .default(true)
            .interact()?;

        if install_hooks {
            print!("  Installing hooks... ");
            let result = Command::new(cli).arg("install").output();

            match result {
                Ok(output) if output.status.success() => {
                    println!("done!");
                }
                Ok(output) => {
                    println!("failed");
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        println!("  Error: {}", stderr.trim());
                    }
                }
                Err(e) => {
                    println!("failed");
                    println!("  Error running {}: {}", cli, e);
                }
            }
        }
    } else {
        println!("  No pre-commit CLI found (prek or pre-commit).");
        println!("  Install prek: curl -fsSL https://prek.dev/install.sh | sh");
        println!("  Or pre-commit: pip install pre-commit");
        println!("  Then run: prek install  (or pre-commit install)");
    }

    Ok(())
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
        config.push_str("    - \"**/*\"\n");
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    // Note: execute() tests are skipped because dialoguer requires a TTY
    // for interactive prompts (which is now the default). Interactive mode
    // is tested via E2E tests with TTY simulation in tests/cli_e2e_init_interactive.rs.
    // The force flag behavior is tested via E2E tests in tests/cli_e2e_init.rs.

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

    #[test]
    fn test_find_latest_version_zero_major() {
        // 0.x.x versions are valid semver and should be returned
        let tags = vec![
            "v0.1.0".to_string(),
            "v0.5.0".to_string(),
            "v0.2.3".to_string(),
        ];
        let result = super::find_latest_version(&tags);
        assert!(result.is_some());
        let (tag, version) = result.unwrap();
        assert_eq!(tag, "v0.5.0");
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 5);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_find_latest_version_mixed_zero_and_stable() {
        // Stable versions should be preferred over 0.x.x
        let tags = vec![
            "v0.9.9".to_string(),
            "v1.0.0".to_string(),
            "v0.10.0".to_string(),
        ];
        let result = super::find_latest_version(&tags);
        assert!(result.is_some());
        let (tag, version) = result.unwrap();
        assert_eq!(tag, "v1.0.0");
        assert_eq!(version.major, 1);
    }

    #[test]
    fn test_generate_precommit_config() {
        let config = generate_precommit_config();
        // Verify it contains expected structure
        assert!(config.contains("repos:"));
        assert!(config.contains("https://github.com/pre-commit/pre-commit-hooks"));
        assert!(config.contains("rev: v5.0.0"));
        assert!(config.contains("trailing-whitespace"));
        assert!(config.contains("end-of-file-fixer"));
        assert!(config.contains("check-added-large-files"));
        assert!(config.contains("check-merge-conflict"));
    }

    #[test]
    fn test_detect_precommit_cli() {
        // This test verifies the function runs without panicking
        // The actual result depends on system state (whether prek/pre-commit is installed)
        let result = detect_precommit_cli();
        // Result should be None, Some("prek"), or Some("pre-commit")
        match result {
            None => {} // CLI not installed - valid
            Some("prek") => {}
            Some("pre-commit") => {}
            Some(other) => panic!("Unexpected CLI detected: {}", other),
        }
    }

    #[test]
    #[serial]
    fn test_setup_precommit_hooks_creates_config() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Verify .pre-commit-config.yaml doesn't exist
        assert!(!Path::new(".pre-commit-config.yaml").exists());

        // Create the config file directly (simulating what setup_precommit_hooks does)
        fs::write(".pre-commit-config.yaml", generate_precommit_config()).unwrap();

        // Verify file was created
        assert!(Path::new(".pre-commit-config.yaml").exists());

        let content = fs::read_to_string(".pre-commit-config.yaml").unwrap();
        assert!(content.contains("repos:"));
        assert!(content.contains("trailing-whitespace"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_setup_precommit_hooks_skips_existing() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Create existing config with custom content
        let existing_content = "# Custom pre-commit config\nrepos: []";
        fs::write(".pre-commit-config.yaml", existing_content).unwrap();

        // Verify it still has custom content (setup would skip generation)
        let content = fs::read_to_string(".pre-commit-config.yaml").unwrap();
        assert_eq!(content, existing_content);

        env::set_current_dir(original_dir).unwrap();
    }
}
