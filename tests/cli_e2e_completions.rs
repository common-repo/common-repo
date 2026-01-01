//! End-to-end tests for the `common-repo completions` command.
//!
//! These tests verify the CLI behavior of the `completions` command by invoking
//! the binary directly and checking its output.

#[allow(dead_code)]
mod common;
#[allow(unused_imports)]
use common::prelude::*;

#[test]
fn test_completions_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate shell completion scripts",
        ))
        .stdout(predicate::str::contains("bash"))
        .stdout(predicate::str::contains("zsh"))
        .stdout(predicate::str::contains("fish"))
        .stdout(predicate::str::contains("powershell"))
        .stdout(predicate::str::contains("elvish"));
}

#[test]
fn test_completions_bash() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("bash")
        .assert()
        .success()
        // Bash completions should contain the completion function
        .stdout(predicate::str::contains("_common-repo()"))
        // And should reference our subcommands
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("zsh")
        .assert()
        .success()
        // Zsh completions should start with compdef
        .stdout(predicate::str::contains("#compdef common-repo"))
        // And should reference subcommands
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn test_completions_fish() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("fish")
        .assert()
        .success()
        // Fish completions use function syntax
        .stdout(predicate::str::contains("function __fish_common_repo"))
        // And should reference subcommands
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("validate"));
}

#[test]
fn test_completions_powershell() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("powershell")
        .assert()
        .success()
        // PowerShell uses Register-ArgumentCompleter
        .stdout(predicate::str::contains("Register-ArgumentCompleter"))
        .stdout(predicate::str::contains("common-repo"));
}

#[test]
fn test_completions_elvish() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("elvish")
        .assert()
        .success()
        // Elvish sets up completion in edit:completion
        .stdout(predicate::str::contains(
            "edit:completion:arg-completer[common-repo]",
        ))
        // And should contain command completions
        .stdout(predicate::str::contains("apply"));
}

#[test]
fn test_completions_invalid_shell() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .arg("invalid-shell")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_completions_missing_shell_argument() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("completions")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
