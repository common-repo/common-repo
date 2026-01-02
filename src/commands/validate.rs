//! # Validate Command Implementation
//!
//! This module implements the `validate` subcommand, which provides functionality
//! for validating the `.common-repo.yaml` configuration file without applying it.
//!
//! ## Functionality
//!
//! - **Configuration Validation**: Parses the configuration file and validates
//!   its structure and contents.
//! - **Cycle Detection**: Checks for circular dependencies in repository inheritance.
//! - **Pattern Validation**: Validates regex patterns in rename operations and
//!   glob patterns in include/exclude operations.
//! - **Repository Validation**: Verifies that referenced repositories can be
//!   accessed (optional, controlled by flag).
//!
//! This command is a safe, read-only operation that does not modify any files.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use common_repo::config;
use common_repo::output::{emoji, OutputConfig};
use common_repo::phases;
use common_repo::repository::RepositoryManager;

/// Validate a .common-repo.yaml configuration file
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to the .common-repo.yaml configuration file to validate.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// If not provided, it defaults to the system's cache directory
    /// (e.g., `~/.cache/common-repo` on Linux).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// If set, also validate that referenced repositories are accessible.
    #[arg(long)]
    pub check_repos: bool,

    /// Use strict validation (fail on warnings).
    #[arg(long)]
    pub strict: bool,
}

/// Execute the `validate` command.
///
/// This function handles the logic for the `validate` subcommand. It performs
/// comprehensive validation of the configuration file and reports any issues.
///
/// # Arguments
/// * `args` - The command arguments
/// * `color_flag` - The value of the global --color flag ("always", "never", or "auto")
pub fn execute(args: ValidateArgs, color_flag: &str) -> Result<()> {
    let out = OutputConfig::from_env_and_flag(color_flag);
    let config_path = &args.config;
    println!(
        "{} Validating configuration: {}",
        emoji(&out, "🔍", "[SCAN]"),
        config_path.display()
    );

    // Load and parse configuration
    let schema = match config::from_file(config_path) {
        Ok(schema) => {
            println!(
                "{} Configuration file parsed successfully",
                emoji(&out, "✅", "[OK]")
            );
            schema
        }
        Err(e) => {
            println!(
                "{} Configuration parsing failed: {}",
                emoji(&out, "❌", "[ERR]"),
                e
            );
            return Err(anyhow::anyhow!("Configuration parsing failed: {}", e));
        }
    };

    let mut has_warnings = false;
    let mut has_errors = false;

    // Determine cache root (used for both cycle detection and repository checks)
    let cache_root = args.cache_root.unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".common-repo-cache"))
            .join("common-repo")
    });

    // Basic configuration statistics
    println!("\n{} Configuration Summary:", emoji(&out, "📊", "[INFO]"));
    println!("   Total operations: {}", schema.len());

    let repo_count = schema
        .iter()
        .filter(|op| matches!(op, config::Operation::Repo { .. }))
        .count();
    println!("   Repository operations: {}", repo_count);

    let other_ops = schema.len() - repo_count;
    println!("   Other operations: {}", other_ops);

    // Cycle detection
    println!(
        "\n{} Checking for circular dependencies...",
        emoji(&out, "🔄", "[CHECK]")
    );
    match phases::discover_repos(&schema, &RepositoryManager::new(cache_root.clone())) {
        Ok(repo_tree) => {
            println!(
                "{} No circular dependencies detected",
                emoji(&out, "✅", "[OK]")
            );
            println!(
                "   Discovered {} repositories in dependency tree",
                repo_tree.all_repos.len()
            );
        }
        Err(e) => {
            if e.to_string().contains("cycle detected") {
                println!(
                    "{} Circular dependency detected: {}",
                    emoji(&out, "❌", "[ERR]"),
                    e
                );
                has_errors = true;
            } else {
                println!(
                    "{} Warning during dependency discovery: {}",
                    emoji(&out, "⚠️", "[WARN]"),
                    e
                );
                has_warnings = true;
            }
        }
    }

    // Validate operation-specific patterns
    println!(
        "\n{} Validating operation patterns...",
        emoji(&out, "🔍", "[SCAN]")
    );

    for (idx, operation) in schema.iter().enumerate() {
        match operation {
            config::Operation::Rename { rename } => {
                // Validate regex patterns
                for mapping in &rename.mappings {
                    if let Err(e) = regex::Regex::new(&mapping.from) {
                        println!(
                            "{} Invalid regex pattern in rename operation {}: {}",
                            emoji(&out, "❌", "[ERR]"),
                            idx,
                            e
                        );
                        has_errors = true;
                    }
                }
            }
            config::Operation::Include { include } => {
                // Validate glob patterns
                for pattern in &include.patterns {
                    if let Err(e) = glob::Pattern::new(pattern) {
                        println!(
                            "{} Invalid glob pattern in include operation {}: {}",
                            emoji(&out, "❌", "[ERR]"),
                            idx,
                            e
                        );
                        has_errors = true;
                    }
                }
            }
            config::Operation::Exclude { exclude } => {
                // Validate glob patterns
                for pattern in &exclude.patterns {
                    if let Err(e) = glob::Pattern::new(pattern) {
                        println!(
                            "{} Invalid glob pattern in exclude operation {}: {}",
                            emoji(&out, "❌", "[ERR]"),
                            idx,
                            e
                        );
                        has_errors = true;
                    }
                }
            }
            config::Operation::Tools { tools } => {
                // Basic validation - tools array should not be empty
                if tools.tools.is_empty() {
                    println!(
                        "{} Tools operation {} has no tools defined",
                        emoji(&out, "⚠️", "[WARN]"),
                        idx
                    );
                    has_warnings = true;
                }
            }
            // Validate merge operations (source/dest requirements, auto-merge conflicts)
            config::Operation::Yaml { yaml } => {
                if let Err(e) = yaml.validate() {
                    println!(
                        "{} Invalid yaml merge operation {}: {}",
                        emoji(&out, "❌", "[ERR]"),
                        idx,
                        e
                    );
                    has_errors = true;
                }
            }
            config::Operation::Json { json } => {
                if let Err(e) = json.validate() {
                    println!(
                        "{} Invalid json merge operation {}: {}",
                        emoji(&out, "❌", "[ERR]"),
                        idx,
                        e
                    );
                    has_errors = true;
                }
            }
            config::Operation::Toml { toml } => {
                if let Err(e) = toml.validate() {
                    println!(
                        "{} Invalid toml merge operation {}: {}",
                        emoji(&out, "❌", "[ERR]"),
                        idx,
                        e
                    );
                    has_errors = true;
                }
            }
            config::Operation::Ini { ini } => {
                if let Err(e) = ini.validate() {
                    println!(
                        "{} Invalid ini merge operation {}: {}",
                        emoji(&out, "❌", "[ERR]"),
                        idx,
                        e
                    );
                    has_errors = true;
                }
            }
            config::Operation::Markdown { markdown } => {
                if let Err(e) = markdown.validate() {
                    println!(
                        "{} Invalid markdown merge operation {}: {}",
                        emoji(&out, "❌", "[ERR]"),
                        idx,
                        e
                    );
                    has_errors = true;
                }
            }
            _ => {
                // Other operations don't have additional validation
            }
        }
    }

    if !has_errors {
        println!(
            "{} All operation patterns are valid",
            emoji(&out, "✅", "[OK]")
        );
    }

    // Optional repository accessibility check
    if args.check_repos {
        println!(
            "\n{} Checking repository accessibility...",
            emoji(&out, "🌐", "[NET]")
        );

        let repo_manager = RepositoryManager::new(cache_root);

        for operation in &schema {
            if let config::Operation::Repo { repo } = operation {
                print!("   Checking {}@{}... ", repo.url, repo.r#ref);

                // Try to list tags to verify accessibility
                match repo_manager.list_repository_tags(&repo.url) {
                    Ok(tags) => {
                        if tags.is_empty() {
                            println!(
                                "{} accessible but no tags found",
                                emoji(&out, "⚠️", "[WARN]")
                            );
                            has_warnings = true;
                        } else {
                            println!(
                                "{} accessible ({} tags)",
                                emoji(&out, "✅", "[OK]"),
                                tags.len()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{} not accessible: {}", emoji(&out, "❌", "[ERR]"), e);
                        has_errors = true;
                    }
                }
            }
        }
    }

    // Final result
    println!("\n{} Validation Result:", emoji(&out, "🎯", "[RESULT]"));

    if has_errors {
        println!(
            "{} Configuration has errors that must be fixed",
            emoji(&out, "❌", "[ERR]")
        );
        return Err(anyhow::anyhow!("Configuration validation failed"));
    }

    if has_warnings && args.strict {
        println!(
            "{} Configuration has warnings (strict mode enabled)",
            emoji(&out, "❌", "[ERR]")
        );
        return Err(anyhow::anyhow!(
            "Configuration validation failed in strict mode"
        ));
    }

    if has_warnings {
        println!(
            "{} Configuration is valid but has warnings",
            emoji(&out, "⚠️", "[WARN]")
        );
    } else {
        println!("{} Configuration is valid", emoji(&out, "✅", "[OK]"));
    }

    if !args.check_repos {
        println!(
            "\n{} Tip: Use --check-repos to also validate repository accessibility",
            emoji(&out, "💡", "[TIP]")
        );
    }

    Ok(())
}
