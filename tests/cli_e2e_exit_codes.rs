//! End-to-end tests for CLI exit codes.
//!
//! These tests verify that the CLI returns the correct exit codes according to
//! the standard conventions documented in [`common_repo::exit_codes`]:
//!
//! - Exit code 0: Success
//! - Exit code 1: General error (or changes detected for `diff` command)
//! - Exit code 2: Invalid command-line usage (handled by clap)

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Exit code 0 is returned for successful operations.
#[test]
fn test_exit_code_success() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    patterns:
      - "*.txt"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .code(0);
}

/// Exit code 0 is returned for --help.
#[test]
fn test_exit_code_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--help").assert().code(0);
}

/// Exit code 0 is returned for --version.
#[test]
fn test_exit_code_version() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--version").assert().code(0);
}

/// Exit code 1 is returned for configuration file not found.
#[test]
fn test_exit_code_error_config_not_found() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg("nonexistent.yaml")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No such file or directory"));
}

/// Exit code 1 is returned for invalid YAML syntax.
#[test]
fn test_exit_code_error_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo.git
    ref: [unclosed
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .code(1);
}

/// Exit code 2 is returned for unknown command-line flags (handled by clap).
#[test]
fn test_exit_code_usage_unknown_flag() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--unknown-flag-that-does-not-exist")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error:"));
}

/// Exit code 2 is returned for unknown subcommand.
#[test]
fn test_exit_code_usage_unknown_subcommand() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("unknown-subcommand-xyz")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error:"));
}

/// Exit code 2 is returned when required arguments are missing.
#[test]
fn test_exit_code_usage_missing_required_arg() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    // The 'completions' command requires a SHELL argument
    cmd.arg("completions")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("required"));
}

/// Exit code 2 is returned for invalid argument values.
#[test]
fn test_exit_code_usage_invalid_arg_value() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    // 'completions' requires a valid shell name
    cmd.arg("completions")
        .arg("invalid-shell-name")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid value"));
}

/// Exit code 0 is returned by diff when no changes are detected.
#[test]
fn test_exit_code_diff_no_changes() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Empty config that produces no files
    config_file
        .write_str(
            r#"
- include:
    patterns:
      - "*.nonexistent"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No changes"));
}

/// Subcommand help returns exit code 0.
#[test]
fn test_exit_code_subcommand_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply").arg("--help").assert().code(0);
}

/// Exit code 2 is returned when --verbose and --quiet are used together.
#[test]
fn test_exit_code_usage_verbose_quiet_conflict() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--verbose")
        .arg("--quiet")
        .arg("validate")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

/// --verbose flag appears in help output.
#[test]
fn test_verbose_flag_in_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--help")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("--verbose"));
}

/// --quiet flag appears in help output.
#[test]
fn test_quiet_flag_in_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--help")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("--quiet"));
}

/// Global --verbose flag works with subcommands.
#[test]
fn test_verbose_flag_works_with_subcommand() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    patterns:
      - "*.txt"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // --verbose should work with validate
    cmd.current_dir(temp.path())
        .arg("--verbose")
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .code(0);
}

/// Global --quiet flag works with subcommands.
#[test]
fn test_quiet_flag_works_with_subcommand() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    patterns:
      - "*.txt"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // --quiet should work with validate
    cmd.current_dir(temp.path())
        .arg("--quiet")
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .code(0);
}
