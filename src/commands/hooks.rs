//! # Hooks Command Implementation
//!
//! This module implements the `hooks` subcommand, which manages Git hooks
//! for common-repo integration. It provides functionality to install and
//! uninstall pre-commit hooks that automatically check for dependency drift.
//!
//! ## Functionality
//!
//! - **Install**: Installs a pre-commit hook that runs `sync-check`
//! - **Uninstall**: Removes the common-repo pre-commit hook
//! - **Status**: Shows whether hooks are installed
//!
//! The hook script runs `common-repo sync-check --auto-fix` before each commit,
//! ensuring that inherited files stay in sync with upstream repositories.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Manage Git hooks for common-repo
#[derive(Args, Debug)]
pub struct HooksArgs {
    #[command(subcommand)]
    pub command: HooksCommand,
}

#[derive(Subcommand, Debug)]
pub enum HooksCommand {
    /// Install the common-repo pre-commit hook
    Install(InstallArgs),

    /// Uninstall the common-repo pre-commit hook
    Uninstall(UninstallArgs),

    /// Show hook installation status
    Status(StatusArgs),
}

/// Arguments for hook installation
#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Path to the Git repository (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub repo: Option<PathBuf>,

    /// Enable auto-fix in the hook (applies small changes automatically)
    #[arg(long, default_value = "true")]
    pub auto_fix: bool,

    /// Maximum files to auto-fix (default: 5)
    #[arg(long, default_value = "5")]
    pub max_files: usize,

    /// Maximum lines to auto-fix (default: 50)
    #[arg(long, default_value = "50")]
    pub max_lines: usize,

    /// Overwrite existing hook if present
    #[arg(long)]
    pub force: bool,
}

/// Arguments for hook uninstallation
#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Path to the Git repository (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub repo: Option<PathBuf>,
}

/// Arguments for status check
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Path to the Git repository (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub repo: Option<PathBuf>,
}

/// Execute the `hooks` command
pub fn execute(args: HooksArgs) -> Result<()> {
    match args.command {
        HooksCommand::Install(install_args) => execute_install(install_args),
        HooksCommand::Uninstall(uninstall_args) => execute_uninstall(uninstall_args),
        HooksCommand::Status(status_args) => execute_status(status_args),
    }
}

/// Hook marker comment to identify common-repo hooks
const HOOK_MARKER: &str = "# common-repo-hook";

/// Generate the pre-commit hook script
fn generate_hook_script(auto_fix: bool, max_files: usize, max_lines: usize) -> String {
    let auto_fix_flag = if auto_fix { "--auto-fix" } else { "" };

    format!(
        r#"#!/bin/sh
{HOOK_MARKER}
# This hook was installed by common-repo to check for dependency drift.
# It runs before each commit to ensure inherited files are in sync.
#
# To uninstall: common-repo hooks uninstall
# To reinstall: common-repo hooks install

# Check if common-repo is available
if ! command -v common-repo >/dev/null 2>&1; then
    echo "Warning: common-repo not found in PATH, skipping sync check"
    exit 0
fi

# Check if .common-repo.yaml exists
if [ ! -f ".common-repo.yaml" ]; then
    # No config file, nothing to check
    exit 0
fi

# Run sync-check
echo "Checking common-repo dependencies..."
common-repo sync-check {auto_fix_flag} --max-files {max_files} --max-lines {max_lines}
EXIT_CODE=$?

case $EXIT_CODE in
    0)
        # In sync or auto-fixed
        ;;
    1)
        # Minor drift detected (not auto-fixed)
        echo ""
        echo "Minor drift detected. Run 'common-repo apply' to sync files."
        echo "Or use 'git commit --no-verify' to skip this check."
        exit 1
        ;;
    2)
        # Major drift detected
        echo ""
        echo "Major drift detected. Please run 'common-repo apply' to sync files."
        echo "Or use 'git commit --no-verify' to skip this check."
        exit 1
        ;;
    *)
        echo "Error running sync-check (exit code $EXIT_CODE)"
        exit 1
        ;;
esac
"#
    )
}

/// Find the .git directory for a repository
fn find_git_dir(repo_path: &Path) -> Result<PathBuf> {
    let git_dir = repo_path.join(".git");

    if git_dir.is_dir() {
        Ok(git_dir)
    } else if git_dir.is_file() {
        // Worktree or submodule - .git is a file pointing to the actual git dir
        let content = fs::read_to_string(&git_dir)?;
        let gitdir = content
            .strip_prefix("gitdir: ")
            .ok_or_else(|| anyhow::anyhow!("Invalid .git file format"))?
            .trim();

        let path = if Path::new(gitdir).is_absolute() {
            PathBuf::from(gitdir)
        } else {
            repo_path.join(gitdir)
        };

        Ok(path)
    } else {
        Err(anyhow::anyhow!(
            "Not a Git repository: {}",
            repo_path.display()
        ))
    }
}

/// Install the pre-commit hook
fn execute_install(args: InstallArgs) -> Result<()> {
    let repo_path = args
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let git_dir = find_git_dir(&repo_path)?;
    let hooks_dir = git_dir.join("hooks");
    let hook_path = hooks_dir.join("pre-commit");

    // Create hooks directory if it doesn't exist
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
    }

    // Check for existing hook
    if hook_path.exists() {
        let existing_content = fs::read_to_string(&hook_path)?;

        if existing_content.contains(HOOK_MARKER) {
            if args.force {
                println!("Overwriting existing common-repo hook...");
            } else {
                println!("common-repo hook already installed. Use --force to overwrite.");
                return Ok(());
            }
        } else if args.force {
            // Backup existing hook
            let backup_path = hooks_dir.join("pre-commit.backup");
            fs::rename(&hook_path, &backup_path)?;
            println!(
                "Backed up existing hook to: {}",
                backup_path.display()
            );
        } else {
            anyhow::bail!(
                "A pre-commit hook already exists (not from common-repo). \
                 Use --force to overwrite (existing hook will be backed up)."
            );
        }
    }

    // Write the hook script
    let hook_content = generate_hook_script(args.auto_fix, args.max_files, args.max_lines);
    fs::write(&hook_path, &hook_content)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    println!("Installed pre-commit hook: {}", hook_path.display());
    println!();
    println!("The hook will run 'common-repo sync-check' before each commit.");
    if args.auto_fix {
        println!(
            "Auto-fix is enabled (max {} files, {} lines).",
            args.max_files, args.max_lines
        );
    } else {
        println!("Auto-fix is disabled. Manual 'common-repo apply' required for drift.");
    }

    Ok(())
}

/// Uninstall the pre-commit hook
fn execute_uninstall(args: UninstallArgs) -> Result<()> {
    let repo_path = args
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let git_dir = find_git_dir(&repo_path)?;
    let hook_path = git_dir.join("hooks").join("pre-commit");

    if !hook_path.exists() {
        println!("No pre-commit hook found.");
        return Ok(());
    }

    let content = fs::read_to_string(&hook_path)?;

    if !content.contains(HOOK_MARKER) {
        println!("The pre-commit hook was not installed by common-repo. Not removing.");
        return Ok(());
    }

    fs::remove_file(&hook_path)?;
    println!("Uninstalled pre-commit hook: {}", hook_path.display());

    // Check for backup
    let backup_path = git_dir.join("hooks").join("pre-commit.backup");
    if backup_path.exists() {
        println!(
            "Note: A backup hook exists at: {}",
            backup_path.display()
        );
        println!("You may want to restore it with: mv {} {}",
            backup_path.display(), hook_path.display());
    }

    Ok(())
}

/// Show hook installation status
fn execute_status(args: StatusArgs) -> Result<()> {
    let repo_path = args
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let git_dir = find_git_dir(&repo_path)?;
    let hook_path = git_dir.join("hooks").join("pre-commit");

    if !hook_path.exists() {
        println!("Status: Not installed");
        println!();
        println!("Run 'common-repo hooks install' to install the pre-commit hook.");
        return Ok(());
    }

    let content = fs::read_to_string(&hook_path)?;

    if content.contains(HOOK_MARKER) {
        println!("Status: Installed");
        println!("Hook path: {}", hook_path.display());

        // Check for auto-fix
        if content.contains("--auto-fix") {
            println!("Auto-fix: Enabled");
        } else {
            println!("Auto-fix: Disabled");
        }
    } else {
        println!("Status: Other hook present (not common-repo)");
        println!("Hook path: {}", hook_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_git_repo() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".git/hooks")).unwrap();
        temp
    }

    #[test]
    fn test_generate_hook_script_with_auto_fix() {
        let script = generate_hook_script(true, 5, 50);
        assert!(script.contains(HOOK_MARKER));
        assert!(script.contains("--auto-fix"));
        assert!(script.contains("--max-files 5"));
        assert!(script.contains("--max-lines 50"));
    }

    #[test]
    fn test_generate_hook_script_without_auto_fix() {
        let script = generate_hook_script(false, 5, 50);
        assert!(script.contains(HOOK_MARKER));
        assert!(!script.contains("--auto-fix"));
    }

    #[test]
    fn test_find_git_dir() {
        let temp = setup_git_repo();
        let result = find_git_dir(temp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp.path().join(".git"));
    }

    #[test]
    fn test_find_git_dir_not_repo() {
        let temp = TempDir::new().unwrap();
        let result = find_git_dir(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_install_creates_hook() {
        let temp = setup_git_repo();
        let args = InstallArgs {
            repo: Some(temp.path().to_path_buf()),
            auto_fix: true,
            max_files: 5,
            max_lines: 50,
            force: false,
        };

        execute_install(args).unwrap();

        let hook_path = temp.path().join(".git/hooks/pre-commit");
        assert!(hook_path.exists());

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains(HOOK_MARKER));
    }

    #[test]
    fn test_uninstall_removes_hook() {
        let temp = setup_git_repo();

        // First install
        let install_args = InstallArgs {
            repo: Some(temp.path().to_path_buf()),
            auto_fix: true,
            max_files: 5,
            max_lines: 50,
            force: false,
        };
        execute_install(install_args).unwrap();

        // Then uninstall
        let uninstall_args = UninstallArgs {
            repo: Some(temp.path().to_path_buf()),
        };
        execute_uninstall(uninstall_args).unwrap();

        let hook_path = temp.path().join(".git/hooks/pre-commit");
        assert!(!hook_path.exists());
    }

    #[test]
    fn test_status_not_installed() {
        let temp = setup_git_repo();
        let args = StatusArgs {
            repo: Some(temp.path().to_path_buf()),
        };

        // Should not error
        execute_status(args).unwrap();
    }
}
