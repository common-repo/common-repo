//! Integration tests for JSON merge operators.
//!
//! These tests verify end-to-end JSON merge functionality using fixtures
//! from the `tests/testdata/merge-json-repo/` directory.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Run all JSON merge integration tests
//! cargo test --features integration-tests --test integration_merge_json
//!
//! # Run a specific test
//! cargo test --features integration-tests --test integration_merge_json test_json_basic_root_merge
//! ```
//!
//! ## Test Scenarios
//!
//! 1. `basic_root_merge` - Root-level merge, fragment values override destination
//! 2. `package_dependencies` - Merge into package.json dependencies section
//! 3. `array_insert_start` - Insert item at start of scripts array
//! 4. `array_insert_end` - Insert item at end of scripts array
//! 5. `nested_object_replace` - Replace config.database with fragment

mod integration_merge_common;

use integration_merge_common::{
    assert_json_contains, assert_json_nested, fixtures, parse_json, run_apply_expect_success,
    setup_fixture_dir,
};

/// Test 1: Basic root-level merge
///
/// Verifies that when merging at the root level:
/// - Fragment values override destination values (version: "1.0.0" -> "2.0.0")
/// - New fields from fragment are added (added_field)
/// - Existing destination fields are preserved (existing_field)
/// - Nested objects are merged (nested.from_fragment added, nested.priority overridden)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_json_basic_root_merge() {
    let temp = setup_fixture_dir(fixtures::JSON);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify fragment values override destination values
    assert_json_contains(&temp, "destination-basic.json", "version", "2.0.0");

    // Verify new fields from fragment are added
    assert_json_contains(
        &temp,
        "destination-basic.json",
        "added_field",
        "This field was added by merge",
    );

    // Verify existing destination fields are preserved
    assert_json_contains(
        &temp,
        "destination-basic.json",
        "existing_field",
        "This field was already here",
    );

    // Verify nested fields are merged correctly
    assert_json_nested(&temp, "destination-basic.json", "nested.priority", "high");

    // Verify nested boolean field
    let json = parse_json(&temp, "destination-basic.json");
    let from_fragment = json
        .get("nested")
        .and_then(|n| n.get("from_fragment"))
        .and_then(|v| v.as_bool())
        .expect("nested.from_fragment should be a boolean");
    assert!(from_fragment, "nested.from_fragment should be true");

    // Verify original nested field is preserved
    let original = json
        .get("nested")
        .and_then(|n| n.get("original"))
        .and_then(|v| v.as_bool())
        .expect("nested.original should be a boolean");
    assert!(original, "nested.original should be true (preserved)");
}

/// Test 2: Package dependencies merge
///
/// Verifies that when merging into package.json dependencies:
/// - Fragment dependencies are merged into the dependencies section
/// - Original dependencies are preserved
/// - Other parts of package.json remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_json_package_dependencies() {
    let temp = setup_fixture_dir(fixtures::JSON);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify original dependencies are preserved
    assert_json_nested(&temp, "package.json", "dependencies.react", "^18.2.0");
    assert_json_nested(&temp, "package.json", "dependencies.react-dom", "^18.2.0");

    // Verify fragment dependencies are merged
    assert_json_nested(&temp, "package.json", "dependencies.lodash", "^4.17.21");
    assert_json_nested(&temp, "package.json", "dependencies.axios", "^1.6.0");
    assert_json_nested(&temp, "package.json", "dependencies.express", "^4.18.2");

    // Verify other parts of package.json remain unchanged
    assert_json_contains(&temp, "package.json", "name", "test-package");
    assert_json_contains(&temp, "package.json", "version", "1.0.0");
    assert_json_nested(&temp, "package.json", "devDependencies.jest", "^29.0.0");
}

/// Test 3: Array insert at start
///
/// Verifies that when inserting at the start of an array:
/// - Fragment item is inserted at position 0
/// - Original items are shifted to later positions
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_json_array_insert_start() {
    let temp = setup_fixture_dir(fixtures::JSON);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify array contents
    let json = parse_json(&temp, "destination-scripts.json");

    // Get the scripts array
    let scripts = json
        .get("scripts")
        .expect("scripts key should exist")
        .as_array()
        .expect("scripts should be an array");

    // After both operations (insert at start, then insert at end), we expect 3 items:
    // [0] pre-task (inserted at start)
    // [1] main-task (original)
    // [2] post-task (inserted at end)
    assert_eq!(scripts.len(), 3, "Should have 3 scripts after both inserts");

    // Verify the first item is the pre-task (inserted at start)
    let first_item = &scripts[0];
    assert_eq!(
        first_item.get("name").and_then(|v| v.as_str()),
        Some("pre-task"),
        "First item should be pre-task (inserted at start)"
    );
    assert_eq!(
        first_item.get("command").and_then(|v| v.as_str()),
        Some("echo 'Starting task'"),
        "First item command should match"
    );
}

/// Test 4: Array insert at end
///
/// Verifies that when inserting at the end of an array:
/// - Fragment item is appended after existing items
/// - Original items remain in their original positions
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_json_array_insert_end() {
    let temp = setup_fixture_dir(fixtures::JSON);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify array contents
    let json = parse_json(&temp, "destination-scripts.json");

    // Get the scripts array
    let scripts = json
        .get("scripts")
        .expect("scripts key should exist")
        .as_array()
        .expect("scripts should be an array");

    // After both operations, we expect the original item in the middle
    // and post-task at the end
    assert_eq!(scripts.len(), 3, "Should have 3 scripts after both inserts");

    // Verify the middle item is the original main-task
    let middle_item = &scripts[1];
    assert_eq!(
        middle_item.get("name").and_then(|v| v.as_str()),
        Some("main-task"),
        "Middle item should be main-task (original)"
    );
    assert_eq!(
        middle_item.get("command").and_then(|v| v.as_str()),
        Some("npm run build"),
        "Middle item command should match"
    );

    // Verify the last item is the post-task (inserted at end)
    let last_item = &scripts[2];
    assert_eq!(
        last_item.get("name").and_then(|v| v.as_str()),
        Some("post-task"),
        "Last item should be post-task (inserted at end)"
    );
    assert_eq!(
        last_item.get("command").and_then(|v| v.as_str()),
        Some("echo 'Task completed'"),
        "Last item command should match"
    );

    // Verify other properties of the file remain unchanged
    assert_json_contains(&temp, "destination-scripts.json", "name", "my-app");
}

/// Test 5: Nested object replace
///
/// Verifies that when replacing a nested object path:
/// - The entire target path is replaced with fragment contents
/// - Other parts of the document remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_json_nested_object_replace() {
    let temp = setup_fixture_dir(fixtures::JSON);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify contents
    let json = parse_json(&temp, "destination-config.json");

    // Verify config.database has been replaced with fragment values
    let database = json
        .get("config")
        .expect("config key should exist")
        .get("database")
        .expect("config.database should exist");

    // Check that fragment values are present
    assert_eq!(
        database.get("host").and_then(|v| v.as_str()),
        Some("postgres.example.com"),
        "config.database.host should be from fragment"
    );
    assert_eq!(
        database.get("port").and_then(|v| v.as_i64()),
        Some(5432),
        "config.database.port should be 5432"
    );
    assert_eq!(
        database.get("ssl").and_then(|v| v.as_bool()),
        Some(true),
        "config.database.ssl should be true (from fragment)"
    );
    assert_eq!(
        database.get("pool_size").and_then(|v| v.as_i64()),
        Some(20),
        "config.database.pool_size should be 20 (from fragment)"
    );

    // Verify other parts of config remain unchanged
    let cache = json
        .get("config")
        .expect("config key should exist")
        .get("cache")
        .expect("config.cache should exist");

    assert_eq!(
        cache.get("enabled").and_then(|v| v.as_bool()),
        Some(true),
        "config.cache.enabled should be preserved"
    );
    assert_eq!(
        cache.get("ttl").and_then(|v| v.as_i64()),
        Some(3600),
        "config.cache.ttl should be preserved"
    );

    // Verify app_name remains unchanged
    assert_json_contains(
        &temp,
        "destination-config.json",
        "app_name",
        "my-application",
    );
}
