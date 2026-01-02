//! End-to-end tests for TTY detection and color handling.
//!
//! These tests verify that the CLI properly handles:
//! - `--color=never` flag to disable colors and emojis
//! - `NO_COLOR` environment variable (https://no-color.org/)
//! - Non-TTY output (piped commands)

#[allow(dead_code)]
mod common;
#[allow(unused_imports)]
use common::prelude::*;

/// Helper predicate that matches any of our status emojis
fn contains_emoji() -> impl predicates::Predicate<str> {
    predicates::str::contains("🔍")
        .or(predicates::str::contains("✅"))
        .or(predicates::str::contains("❌"))
        .or(predicates::str::contains("📊"))
        .or(predicates::str::contains("🌳"))
}

// =============================================================================
// --color=never flag tests
// =============================================================================

#[test]
fn test_color_never_disables_emojis_in_validate() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .arg("--color=never")
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not())
        .stdout(predicate::str::contains("Validating configuration"));
}

#[test]
fn test_color_never_validate_still_shows_content() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .arg("--color=never")
        .arg("validate")
        .assert()
        .success()
        // Should still have meaningful output, just without emojis
        .stdout(predicate::str::contains("Configuration"))
        .stdout(predicate::str::contains("valid").or(predicate::str::contains("Valid")));
}

// =============================================================================
// NO_COLOR environment variable tests (https://no-color.org/)
// =============================================================================

#[test]
fn test_no_color_env_disables_emojis() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .env("NO_COLOR", "1")
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not());
}

#[test]
fn test_no_color_env_empty_value_disables_emojis() {
    // NO_COLOR spec: presence of variable (even empty) should disable colors
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .env("NO_COLOR", "")
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not());
}

#[test]
fn test_color_always_overrides_no_color() {
    // --color=always should force colors even when NO_COLOR is set
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .env("NO_COLOR", "1")
        .arg("--color=always")
        .arg("validate")
        .assert()
        .success()
        .stdout(contains_emoji());
}

// =============================================================================
// TERM=dumb tests
// =============================================================================

#[test]
fn test_term_dumb_disables_emojis() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .env("TERM", "dumb")
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not());
}

// =============================================================================
// --color=always tests
// =============================================================================

#[test]
fn test_color_always_shows_emojis() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .arg("--color=always")
        .arg("validate")
        .assert()
        .success()
        .stdout(contains_emoji());
}

// =============================================================================
// Tree command tests
// =============================================================================

#[test]
fn test_tree_color_never_disables_emoji() {
    let fixture = TestFixture::new().with_config("[]");

    fixture
        .command()
        .arg("--color=never")
        .arg("tree")
        .assert()
        // Tree with empty config should work but may fail - just check no emoji in output
        .stdout(predicate::str::contains("🌳").not());
}

// =============================================================================
// Help text should not have emojis
// =============================================================================

#[test]
fn test_help_has_no_emojis() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not())
        .stdout(predicate::str::contains("✅").not());
}

#[test]
fn test_validate_help_has_no_emojis() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("validate")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔍").not());
}
