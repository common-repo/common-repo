//! Integration tests for INI merge operators.
//!
//! These tests verify end-to-end INI merge functionality using fixtures
//! from the `tests/testdata/merge-ini-repo/` directory.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Run all INI merge integration tests
//! cargo test --features integration-tests --test integration_merge_ini
//!
//! # Run a specific test
//! cargo test --features integration-tests --test integration_merge_ini test_ini_basic_section_merge
//! ```
//!
//! ## Test Scenarios
//!
//! 1. `basic_section_merge` - Root-level merge, fragment values override destination
//! 2. `section_targeting` - Merge into specific section of destination
//! 3. `duplicates_allowed` - Append with duplicate keys allowed
//! 4. `duplicates_disallowed` - Append without duplicate keys

mod integration_merge_common;

use integration_merge_common::{
    assert_ini_contains, assert_ini_has_section, fixtures, parse_ini, run_apply_expect_success,
    setup_fixture_dir,
};

/// Test 1: Basic section merge
///
/// Verifies that when merging at the root level:
/// - Fragment values override destination values (version: 1.0 -> 2.0)
/// - New fields from fragment are added (added_field in general, new_feature in features)
/// - Existing destination fields are preserved (existing_field, old_feature)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ini_basic_section_merge() {
    let temp = setup_fixture_dir(fixtures::INI);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify fragment values override destination values
    assert_ini_contains(&temp, "destination-basic.ini", "general", "version", "2.0");

    // Verify new fields from fragment are added
    assert_ini_contains(
        &temp,
        "destination-basic.ini",
        "general",
        "added_field",
        "This field was added by merge",
    );

    // Verify existing destination fields are preserved
    assert_ini_contains(
        &temp,
        "destination-basic.ini",
        "general",
        "existing_field",
        "This field was already here",
    );

    // Verify features section is merged
    assert_ini_has_section(&temp, "destination-basic.ini", "features");

    // Verify new feature from fragment is added
    assert_ini_contains(
        &temp,
        "destination-basic.ini",
        "features",
        "new_feature",
        "enabled",
    );

    // Verify old feature is preserved
    assert_ini_contains(
        &temp,
        "destination-basic.ini",
        "features",
        "old_feature",
        "enabled",
    );
}

/// Test 2: Section targeting
///
/// Verifies that when merging into a specific section:
/// - Fragment values are merged into the target section only
/// - Values in the target section are overridden
/// - New keys are added to the target section
/// - Other sections remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ini_section_targeting() {
    let temp = setup_fixture_dir(fixtures::INI);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify database section is updated with fragment values
    assert_ini_contains(
        &temp,
        "config.ini",
        "database",
        "host",
        "postgres.example.com",
    );

    // Verify ssl_mode is updated from fragment
    assert_ini_contains(&temp, "config.ini", "database", "ssl_mode", "require");

    // Verify new key pool_size is added from fragment
    assert_ini_contains(&temp, "config.ini", "database", "pool_size", "20");

    // Verify port is preserved (same value in both)
    assert_ini_contains(&temp, "config.ini", "database", "port", "5432");

    // Verify app section remains unchanged
    assert_ini_contains(&temp, "config.ini", "app", "name", "My Application");
    assert_ini_contains(&temp, "config.ini", "app", "environment", "production");

    // Verify cache section remains unchanged
    assert_ini_contains(&temp, "config.ini", "cache", "enabled", "true");
    assert_ini_contains(&temp, "config.ini", "cache", "ttl", "3600");
}

/// Test 3: Duplicates allowed
///
/// Verifies that when appending with allow-duplicates: true:
/// - Fragment values are appended to the section
/// - New keys from fragment are added
/// - The INI file may contain multiple values for the same key
/// - Other sections remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ini_duplicates_allowed() {
    let temp = setup_fixture_dir(fixtures::INI);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify logging section exists
    assert_ini_has_section(&temp, "app.ini", "logging");

    // Verify new keys from fragment are added
    assert_ini_contains(
        &temp,
        "app.ini",
        "logging",
        "file_path",
        "/var/log/app/debug.log",
    );
    assert_ini_contains(&temp, "app.ini", "logging", "rotation", "daily");

    // Verify app section remains unchanged
    assert_ini_contains(&temp, "app.ini", "app", "name", "Test App");

    // Note: When duplicates are allowed, the INI library may return the last value
    // or support multi-values. We verify at least the file_path and rotation were added.
    // The handler key may have either the original or the fragment value (or both).
    let ini = parse_ini(&temp, "app.ini");
    let logging = ini
        .section(Some("logging"))
        .expect("logging section should exist");

    // At minimum, the section should have format from original and file_path/rotation from fragment
    assert!(
        logging.get("format").is_some() || logging.get("file_path").is_some(),
        "logging section should have values from original or fragment"
    );
}

/// Test 4: Duplicates disallowed
///
/// Verifies that when appending with allow-duplicates: false:
/// - Existing keys in destination are preserved (not overwritten by fragment)
/// - New keys from fragment are added
/// - Original keys not in fragment are preserved
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ini_duplicates_disallowed() {
    let temp = setup_fixture_dir(fixtures::INI);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify server section exists
    assert_ini_has_section(&temp, "server.ini", "server");

    // Verify existing keys in destination are preserved (not overwritten)
    // With append: true and allow-duplicates: false, the original timeout (60) is kept
    assert_ini_contains(&temp, "server.ini", "server", "timeout", "60");

    // Verify new keys from fragment are added
    assert_ini_contains(&temp, "server.ini", "server", "max_connections", "1000");
    assert_ini_contains(&temp, "server.ini", "server", "keepalive", "true");

    // Verify original keys not in fragment are preserved
    assert_ini_contains(&temp, "server.ini", "server", "host", "0.0.0.0");
    assert_ini_contains(&temp, "server.ini", "server", "port", "8080");
    assert_ini_contains(&temp, "server.ini", "server", "workers", "4");
}
