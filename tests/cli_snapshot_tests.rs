//! Snapshot tests for CLI output using insta.
//!
//! These tests capture CLI help text and error messages as snapshots,
//! making it easy to review changes to user-facing output.
//!
//! To update snapshots after intentional changes:
//! ```bash
//! cargo insta test --accept
//! ```

use assert_cmd::cargo::cargo_bin_cmd;

/// Normalize version and path-dependent parts of CLI output for stable snapshots
fn normalize_output(output: &str) -> String {
    // Replace version numbers to make snapshots stable across releases
    let re = regex::Regex::new(r"common-repo \d+\.\d+\.\d+").unwrap();
    let versioned = re.replace_all(output, "common-repo [VERSION]");
    // Strip trailing whitespace from each line to match pre-commit formatting
    versioned
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_main_help_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = normalize_output(&stdout);

    insta::assert_snapshot!("main_help", normalized);
}

#[test]
fn test_apply_help_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .args(["apply", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = normalize_output(&stdout);

    insta::assert_snapshot!("apply_help", normalized);
}

#[test]
fn test_check_help_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .args(["check", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = normalize_output(&stdout);

    insta::assert_snapshot!("check_help", normalized);
}

#[test]
fn test_ls_help_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .args(["ls", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = normalize_output(&stdout);

    insta::assert_snapshot!("ls_help", normalized);
}

#[test]
fn test_init_help_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .args(["init", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = normalize_output(&stdout);

    insta::assert_snapshot!("init_help", normalized);
}

#[test]
fn test_missing_config_error_snapshot() {
    let temp = tempfile::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .current_dir(temp.path())
        .arg("ls")
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Extract error message and hints, ignoring stack backtrace which varies by environment
    let error_lines: Vec<&str> = stderr
        .lines()
        .take_while(|line| !line.contains("Stack backtrace"))
        .filter(|line| !line.is_empty())
        .collect();
    let error_message = error_lines.join("\n");

    insta::assert_snapshot!("missing_config_error", error_message);
}

#[test]
fn test_invalid_subcommand_error_snapshot() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .arg("nonexistent-command")
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let normalized = normalize_output(&stderr);

    insta::assert_snapshot!("invalid_subcommand_error", normalized);
}
