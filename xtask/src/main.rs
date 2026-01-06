//! Development automation tasks for common-repo.
//!
//! This crate provides `cargo xtask` commands for common development tasks
//! that are too complex for shell scripts or require cross-platform support.
//!
//! # Usage
//!
//! ```bash
//! cargo xtask coverage      # Run test coverage with cargo-tarpaulin
//! cargo xtask release-prep  # Prepare a new release
//! cargo xtask check-prose   # Check for AI writing patterns in docs
//! ```

mod check_prose;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development automation tasks for common-repo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run test coverage with cargo-tarpaulin
    Coverage {
        /// Output format (html, json, xml, or lcov)
        #[arg(long, short, default_value = "html")]
        format: String,
        /// Minimum coverage threshold (0-100)
        #[arg(long)]
        fail_under: Option<u8>,
        /// Open HTML report in browser after completion
        #[arg(long)]
        open: bool,
    },
    /// Prepare a new release
    ReleasePrep {
        /// The version to release (e.g., 1.2.3)
        #[arg(long, short)]
        version: Option<String>,
        /// Perform a dry run without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Check for AI writing patterns in documentation and code comments
    CheckProse {
        /// Paths to check (files or directories). Defaults to current directory.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Output format (text or json)
        #[arg(long, short, default_value = "text")]
        format: String,
        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Find workspace root
    let workspace_root = workspace_root()?;
    env::set_current_dir(&workspace_root).with_context(|| {
        format!(
            "Failed to change to workspace root: {}",
            workspace_root.display()
        )
    })?;

    match cli.command {
        Commands::Coverage {
            format,
            fail_under,
            open,
        } => run_coverage(&format, fail_under, open),
        Commands::ReleasePrep { version, dry_run } => run_release_prep(version.as_deref(), dry_run),
        Commands::CheckProse {
            paths,
            format,
            verbose,
        } => run_check_prose(paths, &format, verbose),
    }
}

/// Find the workspace root directory.
fn workspace_root() -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .context("Failed to run 'cargo locate-project'")?;

    if !output.status.success() {
        bail!("Failed to locate workspace root");
    }

    let path = String::from_utf8(output.stdout).context("Invalid UTF-8 in cargo output")?;
    let path = PathBuf::from(path.trim());

    // The output is Cargo.toml path, we need the directory
    path.parent()
        .map(|p| p.to_path_buf())
        .context("Failed to get parent directory of Cargo.toml")
}

/// Run the prose linter to check for AI writing patterns.
fn run_check_prose(paths: Vec<PathBuf>, format: &str, verbose: bool) -> Result<()> {
    let output_format = format
        .parse::<check_prose::OutputFormat>()
        .map_err(|e| anyhow::anyhow!(e))?;

    let config = check_prose::CheckProseConfig {
        paths,
        format: output_format,
        verbose,
    };

    check_prose::run(config)
}

/// Run test coverage with cargo-tarpaulin.
fn run_coverage(format: &str, fail_under: Option<u8>, open: bool) -> Result<()> {
    // Check if tarpaulin is installed
    if !is_command_available("cargo-tarpaulin") {
        println!("cargo-tarpaulin is not installed.");
        println!("Install with: cargo install cargo-tarpaulin");
        println!();
        println!("Note: cargo-tarpaulin only works on Linux x86_64.");
        bail!("cargo-tarpaulin not found");
    }

    let mut args = vec!["tarpaulin"];

    // Set output format
    let out_format = match format.to_lowercase().as_str() {
        "html" => {
            args.push("--out");
            args.push("Html");
            Some("target/tarpaulin/tarpaulin-report.html")
        }
        "json" => {
            args.push("--out");
            args.push("Json");
            Some("target/tarpaulin/tarpaulin-report.json")
        }
        "xml" => {
            args.push("--out");
            args.push("Xml");
            Some("target/tarpaulin/cobertura.xml")
        }
        "lcov" => {
            args.push("--out");
            args.push("Lcov");
            Some("target/tarpaulin/lcov.info")
        }
        _ => {
            bail!("Unknown format '{}'. Use: html, json, xml, or lcov", format);
        }
    };

    // Add fail-under threshold
    if let Some(threshold) = fail_under {
        args.push("--fail-under");
        // We need to own this string
        let threshold_str = threshold.to_string();
        // Use leak to create a 'static str - acceptable for CLI args
        args.push(Box::leak(threshold_str.into_boxed_str()));
    }

    println!("Running coverage...");
    let status = run_cargo(&args)?;

    if !status.success() {
        if fail_under.is_some() {
            bail!("Coverage is below the required threshold");
        }
        bail!("Coverage failed");
    }

    if let Some(report_path) = out_format {
        println!();
        println!("Coverage report: {}", report_path);

        if open && format == "html" {
            open_in_browser(report_path)?;
        }
    }

    Ok(())
}

/// Prepare a new release.
fn run_release_prep(version: Option<&str>, dry_run: bool) -> Result<()> {
    // Get current version from Cargo.toml
    let cargo_toml = std::fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;

    let current_version = cargo_toml
        .lines()
        .find(|line| line.starts_with("version = "))
        .and_then(|line| line.split('"').nth(1))
        .context("Failed to find version in Cargo.toml")?;

    println!("Current version: {}", current_version);

    let new_version = match version {
        Some(v) => v.to_string(),
        None => {
            // Suggest next patch version
            let parts: Vec<&str> = current_version.split('.').collect();
            if parts.len() != 3 {
                bail!("Invalid version format in Cargo.toml");
            }
            let major = parts[0];
            let minor = parts[1];
            let patch: u32 = parts[2].parse().context("Invalid patch version")?;
            format!("{}.{}.{}", major, minor, patch + 1)
        }
    };

    println!("New version: {}", new_version);

    if dry_run {
        println!();
        println!("Dry run - the following changes would be made:");
        println!("  1. Update version in Cargo.toml to {}", new_version);
        println!("  2. Update version in Cargo.lock");
        println!("  3. Run cargo check to verify");
        println!();
        println!("Run without --dry-run to apply changes.");
        return Ok(());
    }

    // Update Cargo.toml
    println!("Updating Cargo.toml...");
    let updated_cargo_toml = cargo_toml.replacen(
        &format!("version = \"{}\"", current_version),
        &format!("version = \"{}\"", new_version),
        1,
    );
    std::fs::write("Cargo.toml", updated_cargo_toml).context("Failed to write Cargo.toml")?;

    // Update Cargo.lock by running cargo check
    println!("Updating Cargo.lock...");
    let status = run_cargo(&["check"])?;
    if !status.success() {
        bail!("cargo check failed after version update");
    }

    println!();
    println!("Release preparation complete!");
    println!();
    println!("Next steps:");
    println!("  1. Review changes: git diff");
    println!(
        "  2. Commit: git commit -am \"chore(main): release {}\"",
        new_version
    );
    println!("  3. Tag: git tag v{}", new_version);
    println!("  4. Push: git push && git push --tags");

    Ok(())
}

/// Check if a command is available in PATH.
fn is_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a cargo command.
fn run_cargo(args: &[&str]) -> Result<ExitStatus> {
    Command::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("Failed to run cargo {}", args.join(" ")))
}

/// Open a file in the default browser.
fn open_in_browser(path: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    Command::new(cmd)
        .arg(path)
        .spawn()
        .context("Failed to open browser")?;

    Ok(())
}
