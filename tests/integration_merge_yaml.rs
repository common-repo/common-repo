//! Integration tests for YAML merge operators.
//!
//! These tests verify end-to-end YAML merge functionality using fixtures
//! from the `tests/testdata/merge-yaml-repo/` directory.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Run all merge integration tests
//! cargo test --features integration-tests --test integration_merge_yaml
//!
//! # Run a specific test
//! cargo test --features integration-tests --test integration_merge_yaml test_yaml_basic_root_merge
//! ```
//!
//! ## Test Scenarios
//!
//! 1. `basic_root_merge` - Root-level merge, fragment values override destination
//! 2. `nested_path_merge` - Merge into a specific path (metadata.labels)
//! 3. `list_append` - Append items to an existing array
//! 4. `section_replace` - Replace an entire config section

mod integration_merge_common;

use integration_merge_common::{
    assert_yaml_contains, assert_yaml_nested, fixtures, parse_yaml, run_apply_expect_success,
    setup_fixture_dir,
};

/// Test 1: Basic root-level merge
///
/// Verifies that when merging at the root level:
/// - Fragment values override destination values (version: "1.0" -> "2.0")
/// - New fields from fragment are added (added_field)
/// - Existing destination fields are preserved (existing_field)
/// - Nested objects are merged (nested.from_fragment added, nested.priority overridden)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_yaml_basic_root_merge() {
    let temp = setup_fixture_dir(fixtures::YAML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify fragment values override destination values
    assert_yaml_contains(&temp, "destination-basic.yml", "version", "2.0");

    // Verify new fields from fragment are added
    assert_yaml_contains(
        &temp,
        "destination-basic.yml",
        "added_field",
        "This field was added by merge",
    );

    // Verify existing destination fields are preserved
    assert_yaml_contains(
        &temp,
        "destination-basic.yml",
        "existing_field",
        "This field was already here",
    );

    // Verify nested fields are merged correctly
    assert_yaml_nested(&temp, "destination-basic.yml", "nested.priority", "high");

    // Verify nested boolean field (from_fragment is a boolean, not a string)
    let yaml = parse_yaml(&temp, "destination-basic.yml");
    let from_fragment = yaml
        .get("nested")
        .and_then(|n| n.get("from_fragment"))
        .and_then(|v| v.as_bool())
        .expect("nested.from_fragment should be a boolean");
    assert!(from_fragment, "nested.from_fragment should be true");
}

/// Test 2: Nested path merge
///
/// Verifies that when merging at a specific path (metadata.labels):
/// - Fragment values are merged into the target path
/// - Existing values at the target path are preserved
/// - Other parts of the document remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_yaml_nested_path_merge() {
    let temp = setup_fixture_dir(fixtures::YAML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify fragment values are merged into metadata.labels
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.labels.app",
        "my-application",
    );
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.labels.environment",
        "production",
    );
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.labels.team",
        "platform",
    );

    // Verify existing labels are preserved
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.labels.version",
        "1.0",
    );
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.labels.region",
        "us-west-2",
    );

    // Verify other parts of the document remain unchanged
    assert_yaml_contains(&temp, "destination-nested.yml", "apiVersion", "v1");
    assert_yaml_contains(&temp, "destination-nested.yml", "kind", "Service");
    assert_yaml_nested(
        &temp,
        "destination-nested.yml",
        "metadata.name",
        "my-service",
    );
}

/// Test 3: List append
///
/// Verifies that when appending to a list:
/// - New items from the fragment are appended to the existing array
/// - Original items remain in their original positions
/// - The order is preserved (existing items first, then appended items)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_yaml_list_append() {
    let temp = setup_fixture_dir(fixtures::YAML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify array contents
    let yaml = parse_yaml(&temp, "destination-list.yml");

    // Get the items array
    let items = yaml
        .get("items")
        .expect("items key should exist")
        .as_sequence()
        .expect("items should be an array");

    // Verify we have 3 items total (1 original + 2 appended)
    assert_eq!(items.len(), 3, "Should have 3 items after append");

    // Verify the original item is first
    let first_item = &items[0];
    assert_eq!(
        first_item.get("name").and_then(|v| v.as_str()),
        Some("existing-item"),
        "First item should be the existing item"
    );
    assert_eq!(
        first_item.get("value").and_then(|v| v.as_i64()),
        Some(50),
        "First item value should be 50"
    );

    // Verify appended items follow
    let second_item = &items[1];
    assert_eq!(
        second_item.get("name").and_then(|v| v.as_str()),
        Some("new-item-1"),
        "Second item should be new-item-1"
    );
    assert_eq!(
        second_item.get("value").and_then(|v| v.as_i64()),
        Some(100),
        "Second item value should be 100"
    );

    let third_item = &items[2];
    assert_eq!(
        third_item.get("name").and_then(|v| v.as_str()),
        Some("new-item-2"),
        "Third item should be new-item-2"
    );
    assert_eq!(
        third_item.get("value").and_then(|v| v.as_i64()),
        Some(200),
        "Third item value should be 200"
    );

    // Verify other keys are preserved
    let yaml = parse_yaml(&temp, "destination-list.yml");
    let config = yaml.get("config").expect("config key should exist");
    assert_eq!(
        config.get("enabled").and_then(|v| v.as_bool()),
        Some(true),
        "config.enabled should be preserved"
    );
}

/// Test 4: Section replace
///
/// Verifies that when replacing a section:
/// - The entire target section is replaced with fragment contents
/// - Other sections of the document remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_yaml_section_replace() {
    let temp = setup_fixture_dir(fixtures::YAML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify section contents
    let yaml = parse_yaml(&temp, "destination-replace.yml");

    // Verify the config section has been replaced with fragment values
    let config = yaml.get("config").expect("config key should exist");

    // Check that fragment values are present
    assert_eq!(
        config.get("enabled").and_then(|v| v.as_bool()),
        Some(false),
        "config.enabled should be false (from fragment)"
    );
    assert_eq!(
        config.get("timeout").and_then(|v| v.as_i64()),
        Some(30),
        "config.timeout should be 30 (from fragment)"
    );
    assert_eq!(
        config.get("retries").and_then(|v| v.as_i64()),
        Some(3),
        "config.retries should be 3 (from fragment)"
    );

    // Verify other sections are unchanged
    assert_yaml_nested(&temp, "destination-replace.yml", "other.field", "value");
}
