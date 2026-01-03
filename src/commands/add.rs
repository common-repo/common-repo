//! # Add Command Implementation
//!
//! This module implements the `add` subcommand, which adds a repository to an existing
//! or new `.common-repo.yaml` configuration file.
//!
//! ## Functionality
//!
//! - **URI Argument**: Add a repository by URL (supports GitHub shorthand like `org/repo`)
//! - **Semver Detection**: Automatically detects and uses the latest semver tag
//! - **Config Detection**: Creates config if missing (or invokes init wizard unless --yes)
//! - **Non-interactive Mode**: Skip prompts with --yes for automation

use anyhow::Result;
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::fs;
use std::path::Path;

use common_repo::defaults::DEFAULT_CONFIG_FILENAME;
use common_repo::git;
use common_repo::version;

/// Add a repository to the configuration file
#[derive(Args, Debug)]
pub struct AddArgs {
    /// Repository URL to add (e.g., https://github.com/org/repo or org/repo)
    #[arg(value_name = "URI")]
    pub uri: String,

    /// Non-interactive mode: create minimal config without prompting if none exists
    #[arg(short, long)]
    pub yes: bool,
}

/// Execute the `add` command.
///
/// This function handles the logic for the `add` subcommand, adding a repository
/// to the `.common-repo.yaml` configuration file. If no configuration exists,
/// it either invokes the init wizard (default) or creates a minimal config (--yes).
pub fn execute(args: AddArgs) -> Result<()> {
    let config_path = Path::new(DEFAULT_CONFIG_FILENAME);

    // Normalize URL (expand GitHub shorthand)
    let url = normalize_repo_url(&args.uri);

    // Fetch version info
    let (version, warnings) = fetch_version_info(&url);

    // Print warnings
    for warning in &warnings {
        eprintln!("  ⚠️  {}", warning);
    }

    // Check if config exists
    if config_path.exists() {
        // Append to existing config
        append_repo_to_config(config_path, &url, &version)?;
        println!("✅ Added {} @ {} to .common-repo.yaml", url, version);
    } else if args.yes {
        // Create minimal config with just this repo
        create_minimal_config(config_path, &url, &version)?;
        println!("✅ Created .common-repo.yaml with {} @ {}", url, version);
    } else {
        // No config exists - ask user for confirmation to create one
        println!("No .common-repo.yaml configuration file found.");
        println!();

        let theme = ColorfulTheme::default();
        let create_config = Confirm::with_theme(&theme)
            .with_prompt(format!(
                "Create a new configuration with {} @ {}?",
                url, version
            ))
            .default(true)
            .interact()?;

        if create_config {
            create_minimal_config(config_path, &url, &version)?;
            println!("✅ Created .common-repo.yaml with {} @ {}", url, version);
            println!("💡 Run `common-repo apply` to fetch and apply configurations");
        } else {
            println!("Aborted. Run 'common-repo init' to create a configuration interactively.");
            return Ok(());
        }
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

/// Fetch version information from a repository.
///
/// Returns the best version to use (latest semver tag or fallback) and any warnings.
fn fetch_version_info(url: &str) -> (String, Vec<String>) {
    print!("Fetching tags from {}... ", url);

    match git::list_tags(url) {
        Ok(tags) => {
            let semver_tags = version::filter_semver_tags(&tags);
            if let Some((latest_tag, parsed_version)) = find_latest_version(&semver_tags) {
                println!("found {}", latest_tag);

                let mut warnings = Vec::new();
                if parsed_version.major == 0 {
                    warnings.push(format!(
                        "Warning: {} is a 0.x.x version, which may indicate unstable API",
                        latest_tag
                    ));
                }
                (latest_tag, warnings)
            } else if !tags.is_empty() {
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
    }
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

/// Create a minimal configuration file with a single repository.
fn create_minimal_config(config_path: &Path, url: &str, version: &str) -> Result<()> {
    let config = format!(
        r#"# common-repo configuration
# Generated by common-repo add

- repo:
    url: {}
    ref: {}

- include:
    patterns:
      - "**/*"
"#,
        url, version
    );

    fs::write(config_path, config)?;
    Ok(())
}

/// Append a repository entry to an existing configuration file.
fn append_repo_to_config(config_path: &Path, url: &str, version: &str) -> Result<()> {
    let mut content = fs::read_to_string(config_path)?;

    // Find where to insert the new repo entry (before include section or at end)
    let new_entry = format!(
        r#"
- repo:
    url: {}
    ref: {}
"#,
        url, version
    );

    // Try to insert before the include section if it exists
    if let Some(include_pos) = content.find("- include:") {
        content.insert_str(include_pos, &new_entry);
    } else {
        // Append at end
        content.push_str(&new_entry);
    }

    fs::write(config_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
    fn test_normalize_repo_url_http() {
        assert_eq!(
            normalize_repo_url("http://example.com/repo"),
            "http://example.com/repo"
        );
    }

    #[test]
    fn test_find_latest_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v2.0.0".to_string(),
            "v1.5.0".to_string(),
        ];
        let result = find_latest_version(&tags);
        assert!(result.is_some());
        let (tag, _) = result.unwrap();
        assert_eq!(tag, "v2.0.0");
    }

    #[test]
    fn test_find_latest_version_empty() {
        let tags: Vec<String> = vec![];
        let result = find_latest_version(&tags);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_latest_version_no_semver() {
        let tags = vec!["main".to_string(), "develop".to_string()];
        let result = find_latest_version(&tags);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_latest_version_zero_major() {
        let tags = vec![
            "v0.1.0".to_string(),
            "v0.5.0".to_string(),
            "v0.2.3".to_string(),
        ];
        let result = find_latest_version(&tags);
        assert!(result.is_some());
        let (tag, version) = result.unwrap();
        assert_eq!(tag, "v0.5.0");
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 5);
    }

    #[test]
    fn test_create_minimal_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");

        create_minimal_config(&config_path, "https://github.com/org/repo", "v1.0.0").unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("url: https://github.com/org/repo"));
        assert!(content.contains("ref: v1.0.0"));
        assert!(content.contains("- include:"));
    }

    #[test]
    fn test_append_repo_to_config_with_include() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");

        // Create initial config with include section
        let initial_config = r#"# common-repo configuration

- repo:
    url: https://github.com/org/repo1
    ref: v1.0.0

- include:
    patterns:
      - "**/*"
"#;
        fs::write(&config_path, initial_config).unwrap();

        append_repo_to_config(&config_path, "https://github.com/org/repo2", "v2.0.0").unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        // New repo should be inserted before include
        assert!(content.contains("url: https://github.com/org/repo1"));
        assert!(content.contains("url: https://github.com/org/repo2"));
        assert!(content.contains("ref: v2.0.0"));
        // Include should still be present
        assert!(content.contains("- include:"));
    }

    #[test]
    fn test_append_repo_to_config_without_include() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");

        // Create initial config without include section
        let initial_config = r#"# common-repo configuration

- repo:
    url: https://github.com/org/repo1
    ref: v1.0.0
"#;
        fs::write(&config_path, initial_config).unwrap();

        append_repo_to_config(&config_path, "https://github.com/org/repo2", "v2.0.0").unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("url: https://github.com/org/repo1"));
        assert!(content.contains("url: https://github.com/org/repo2"));
        assert!(content.contains("ref: v2.0.0"));
    }
}
