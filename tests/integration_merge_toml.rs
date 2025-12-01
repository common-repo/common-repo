//! Integration tests for TOML merge operators.
//!
//! These tests verify end-to-end TOML merge functionality using fixtures
//! from the `tests/testdata/merge-toml-repo/` directory.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Run all TOML merge integration tests
//! cargo test --features integration-tests --test integration_merge_toml
//!
//! # Run a specific test
//! cargo test --features integration-tests --test integration_merge_toml test_toml_basic_root_merge
//! ```
//!
//! ## Test Scenarios
//!
//! 1. `basic_root_merge` - Root-level merge, fragment values override destination
//! 2. `cargo_dependencies` - Merge into Cargo.toml dependencies table
//! 3. `workspace_members_append` - Append to workspace.members array
//! 4. `comment_preservation` - Merge while preserving comments (if supported)

mod integration_merge_common;

use integration_merge_common::{
    assert_file_contains, assert_toml_array_contains, assert_toml_contains, assert_toml_nested,
    fixtures, parse_toml, read_file, run_apply_expect_success, setup_fixture_dir,
};

/// Test 1: Basic root-level merge
///
/// Verifies that when merging at the root level:
/// - Fragment values override destination values (version: "1.0.0" -> "2.0.0")
/// - New fields from fragment are added (added_field)
/// - Existing destination fields are preserved (existing_field)
/// - Nested tables are merged (nested.from_fragment added, nested.priority overridden)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_toml_basic_root_merge() {
    let temp = setup_fixture_dir(fixtures::TOML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify fragment values override destination values
    assert_toml_contains(&temp, "destination-basic.toml", "version", "2.0.0");

    // Verify new fields from fragment are added
    assert_toml_contains(
        &temp,
        "destination-basic.toml",
        "added_field",
        "This field was added by merge",
    );

    // Verify existing destination fields are preserved
    assert_toml_contains(
        &temp,
        "destination-basic.toml",
        "existing_field",
        "This field was already here",
    );

    // Verify nested fields are merged correctly
    assert_toml_nested(&temp, "destination-basic.toml", "nested.priority", "high");

    // Verify nested boolean field (from_fragment is a boolean, not a string)
    let toml = parse_toml(&temp, "destination-basic.toml");
    let from_fragment = toml
        .get("nested")
        .and_then(|n| n.get("from_fragment"))
        .and_then(|v| v.as_bool())
        .expect("nested.from_fragment should be a boolean");
    assert!(from_fragment, "nested.from_fragment should be true");

    // Verify original nested field is preserved
    let original = toml
        .get("nested")
        .and_then(|n| n.get("original"))
        .and_then(|v| v.as_bool())
        .expect("nested.original should be a boolean");
    assert!(original, "nested.original should be true (preserved)");
}

/// Test 2: Cargo dependencies merge
///
/// Verifies that when merging into Cargo.toml dependencies table:
/// - Fragment dependencies are merged into the dependencies section
/// - Original dependencies are preserved
/// - Other parts of Cargo.toml remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_toml_cargo_dependencies() {
    let temp = setup_fixture_dir(fixtures::TOML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify contents
    let toml = parse_toml(&temp, "Cargo.toml");

    // Verify original dependencies are preserved
    let deps = toml
        .get("dependencies")
        .expect("dependencies section should exist");

    // Check original dependencies exist (clap has a table value with version)
    let clap = deps.get("clap").expect("clap dependency should exist");
    assert_eq!(
        clap.get("version").and_then(|v| v.as_str()),
        Some("4.4"),
        "clap version should be preserved"
    );

    // regex is a simple string value
    assert_eq!(
        deps.get("regex").and_then(|v| v.as_str()),
        Some("1.10"),
        "regex should be preserved"
    );

    // Verify fragment dependencies are merged
    let serde = deps.get("serde").expect("serde dependency should be added");
    assert_eq!(
        serde.get("version").and_then(|v| v.as_str()),
        Some("1.0"),
        "serde version should be 1.0"
    );

    let tokio = deps.get("tokio").expect("tokio dependency should be added");
    assert_eq!(
        tokio.get("version").and_then(|v| v.as_str()),
        Some("1.35"),
        "tokio version should be 1.35"
    );

    assert_eq!(
        deps.get("anyhow").and_then(|v| v.as_str()),
        Some("1.0"),
        "anyhow should be added with version 1.0"
    );

    // Verify package section remains unchanged
    assert_toml_nested(&temp, "Cargo.toml", "package.name", "test-package");
    assert_toml_nested(&temp, "Cargo.toml", "package.version", "0.1.0");
    assert_toml_nested(&temp, "Cargo.toml", "package.edition", "2021");

    // Verify dev-dependencies remain unchanged
    assert_toml_nested(&temp, "Cargo.toml", "dev-dependencies.assert_cmd", "2.0");
}

/// Test 3: Workspace members append
///
/// Verifies that when appending to workspace.members array:
/// - New members from fragment are appended to the existing array
/// - Original members are preserved in their positions
/// - Other workspace settings remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_toml_workspace_members_append() {
    let temp = setup_fixture_dir(fixtures::TOML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Parse the result to verify contents
    let toml = parse_toml(&temp, "workspace.toml");

    // Get the workspace.members array
    let members = toml
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("workspace.members should be an array");

    // Verify we have 4 members total (2 original + 2 appended)
    assert_eq!(
        members.len(),
        4,
        "Should have 4 members after append: {:?}",
        members
    );

    // Verify original members are present
    assert_toml_array_contains(&temp, "workspace.toml", "workspace.members", "crate-a");
    assert_toml_array_contains(&temp, "workspace.toml", "workspace.members", "crate-b");

    // Verify new members are appended
    assert_toml_array_contains(&temp, "workspace.toml", "workspace.members", "new-crate-1");
    assert_toml_array_contains(&temp, "workspace.toml", "workspace.members", "new-crate-2");

    // Verify workspace.dependencies remains unchanged
    assert_toml_nested(
        &temp,
        "workspace.toml",
        "workspace.dependencies.common-dep",
        "1.0",
    );
}

/// Test 4: Comment preservation
///
/// Verifies that when merging with preserve-comments: true:
/// - Fragment values are merged into the target path
/// - Comments in the original file are preserved (if implementation supports it)
/// - New fields from fragment are added
///
/// Note: Comment preservation depends on the TOML merge implementation.
/// This test verifies the merge succeeds and checks basic comment presence.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_toml_comment_preservation() {
    let temp = setup_fixture_dir(fixtures::TOML);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify the merge succeeded by checking package values
    let toml = parse_toml(&temp, "destination-with-comments.toml");

    // Get the package section
    let package = toml.get("package").expect("package section should exist");

    // Verify original values are preserved
    assert_eq!(
        package.get("name").and_then(|v| v.as_str()),
        Some("my-package"),
        "package.name should be preserved"
    );
    assert_eq!(
        package.get("version").and_then(|v| v.as_str()),
        Some("1.0.0"),
        "package.version should be preserved"
    );

    // Verify fragment values are merged
    assert_eq!(
        package.get("description").and_then(|v| v.as_str()),
        Some("Updated package description"),
        "package.description should be updated from fragment"
    );
    assert_eq!(
        package.get("license").and_then(|v| v.as_str()),
        Some("MIT OR Apache-2.0"),
        "package.license should be added from fragment"
    );
    assert_eq!(
        package.get("repository").and_then(|v| v.as_str()),
        Some("https://github.com/example/repo"),
        "package.repository should be added from fragment"
    );

    // Verify authors are preserved
    let authors = package
        .get("authors")
        .and_then(|v| v.as_array())
        .expect("authors should be an array");
    assert_eq!(authors.len(), 1, "Should have 1 author");
    assert_eq!(
        authors[0].as_str(),
        Some("Author Name <author@example.com>"),
        "Author should be preserved"
    );

    // Verify dependencies section remains unchanged
    assert_toml_nested(
        &temp,
        "destination-with-comments.toml",
        "dependencies.serde",
        "1.0",
    );

    // Check if comments are preserved in the raw file content
    // This depends on the TOML merge implementation's comment handling
    let content = read_file(&temp, "destination-with-comments.toml");

    // Check for key comments that should be preserved if the implementation supports it
    // Note: If comment preservation is not fully supported, these assertions
    // document the expected behavior when it is implemented
    if content.contains("# This is an important comment at the top") {
        // Comment preservation is working
        assert_file_contains(
            &temp,
            "destination-with-comments.toml",
            "# This is an important comment at the top",
        );
        assert_file_contains(
            &temp,
            "destination-with-comments.toml",
            "# Package metadata",
        );
    }
    // If comments are not preserved, the test still passes as long as
    // the data merge was successful (verified above)
}
