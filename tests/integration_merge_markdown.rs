//! Integration tests for Markdown merge operators.
//!
//! These tests verify end-to-end Markdown merge functionality using fixtures
//! from the `tests/testdata/merge-markdown-repo/` directory.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Run all Markdown merge integration tests
//! cargo test --features integration-tests --test integration_merge_markdown
//!
//! # Run a specific test
//! cargo test --features integration-tests --test integration_merge_markdown test_markdown_section_append_end
//! ```
//!
//! ## Test Scenarios
//!
//! 1. `section_append_end` - Append content to the end of an existing section (Features)
//! 2. `section_append_installation` - Append content to an existing section (Installation)
//! 3. `section_creation` - Create a new section at end of document (Contributing)
//! 4. `section_create_at_start` - Create a new section at start of document (Quick Start)
//! 5. `section_replace` - Replace a section's content entirely (License)

mod integration_merge_common;

use integration_merge_common::{
    assert_markdown_contains, assert_markdown_has_section, assert_markdown_section_order, fixtures,
    read_file, run_apply_expect_success, setup_fixture_dir,
};

/// Test 1: Section append end
///
/// Verifies that when appending to the end of a section:
/// - Fragment content is appended after existing section content
/// - Original section content is preserved
/// - Subsections from fragment are added after existing subsections
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_markdown_section_append_end() {
    let temp = setup_fixture_dir(fixtures::MARKDOWN);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify the Features section still exists
    assert_markdown_has_section(&temp, "README.md", "Features", 2);

    // Verify original Basic Features subsection is preserved
    assert_markdown_contains(&temp, "README.md", "### Basic Features");
    assert_markdown_contains(&temp, "README.md", "- Configuration management");
    assert_markdown_contains(&temp, "README.md", "- API integration");

    // Verify fragment content is appended (Enhanced Security and Performance Improvements)
    assert_markdown_contains(&temp, "README.md", "### Enhanced Security");
    assert_markdown_contains(&temp, "README.md", "- End-to-end encryption");
    assert_markdown_contains(&temp, "README.md", "- Multi-factor authentication support");

    assert_markdown_contains(&temp, "README.md", "### Performance Improvements");
    assert_markdown_contains(&temp, "README.md", "- 50% faster processing");
    assert_markdown_contains(&temp, "README.md", "- Reduced memory footprint");

    // Verify order: Basic Features should come before Enhanced Security
    let content = read_file(&temp, "README.md");
    let basic_pos = content
        .find("### Basic Features")
        .expect("Basic Features should exist");
    let enhanced_pos = content
        .find("### Enhanced Security")
        .expect("Enhanced Security should exist");
    assert!(
        basic_pos < enhanced_pos,
        "Basic Features should come before Enhanced Security (append to end)"
    );
}

/// Test 2: Section append (Installation)
///
/// Verifies that when appending to an existing section:
/// - Fragment content is appended after existing section content
/// - Original section content is preserved
/// - The section heading is preserved
///
/// Note: The position field only affects new section creation, not existing sections.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_markdown_section_append_installation() {
    let temp = setup_fixture_dir(fixtures::MARKDOWN);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify the Installation section still exists
    assert_markdown_has_section(&temp, "README.md", "Installation", 2);

    // Verify fragment content (Prerequisites) is present
    assert_markdown_contains(&temp, "README.md", "### Prerequisites");
    assert_markdown_contains(&temp, "README.md", "- Node.js >= 18.0.0");
    assert_markdown_contains(&temp, "README.md", "- npm >= 9.0.0");
    assert_markdown_contains(&temp, "README.md", "- Git >= 2.30.0");

    // Verify original content (Install via npm) is preserved
    assert_markdown_contains(&temp, "README.md", "### Install via npm");
    assert_markdown_contains(&temp, "README.md", "npm install my-project");

    // Verify order: Install via npm should come before Prerequisites (append to end)
    let content = read_file(&temp, "README.md");
    let install_pos = content
        .find("### Install via npm")
        .expect("Install via npm should exist");
    let prereq_pos = content
        .find("### Prerequisites")
        .expect("Prerequisites should exist");
    assert!(
        install_pos < prereq_pos,
        "Install via npm should come before Prerequisites (append to end)"
    );
}

/// Test 3: Section creation
///
/// Verifies that when creating a new section:
/// - The section is created with the correct heading level
/// - The fragment content is placed in the new section
/// - The section is placed at an appropriate location in the document
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_markdown_section_creation() {
    let temp = setup_fixture_dir(fixtures::MARKDOWN);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify the new Contributing section is created
    assert_markdown_has_section(&temp, "README.md", "Contributing", 2);

    // Verify the fragment content is in the Contributing section
    assert_markdown_contains(&temp, "README.md", "We welcome contributions!");
    assert_markdown_contains(&temp, "README.md", "1. Fork the repository");
    assert_markdown_contains(&temp, "README.md", "2. Create a feature branch");
    assert_markdown_contains(&temp, "README.md", "3. Write tests for your changes");
    assert_markdown_contains(&temp, "README.md", "4. Submit a pull request");
    assert_markdown_contains(
        &temp,
        "README.md",
        "See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.",
    );
}

/// Test 4: Section create at start
///
/// Verifies that when creating a new section with position: start:
/// - The new section is created at the start of the document
/// - The new section appears before existing sections
/// - Original document content is preserved
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_markdown_section_create_at_start() {
    let temp = setup_fixture_dir(fixtures::MARKDOWN);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify Quick Start section exists
    assert_markdown_has_section(&temp, "GUIDE.md", "Quick Start", 2);

    // Verify Quick Start content
    assert_markdown_contains(&temp, "GUIDE.md", "Get up and running in 5 minutes:");
    assert_markdown_contains(&temp, "GUIDE.md", "npm install my-package");
    assert_markdown_contains(&temp, "GUIDE.md", "npm run init");
    assert_markdown_contains(&temp, "GUIDE.md", "npm start");
    assert_markdown_contains(
        &temp,
        "GUIDE.md",
        "Your application will be available at `http://localhost:3000`.",
    );

    // Verify original sections still exist
    assert_markdown_has_section(&temp, "GUIDE.md", "User Guide", 1);
    assert_markdown_has_section(&temp, "GUIDE.md", "Getting Started", 2);
    assert_markdown_has_section(&temp, "GUIDE.md", "Advanced Usage", 2);

    // Verify Quick Start comes before Getting Started (created at start)
    assert_markdown_section_order(&temp, "GUIDE.md", "## Quick Start", "## Getting Started");
}

/// Test 5: Section replace
///
/// Verifies that when replacing a section's content:
/// - The original content is completely replaced
/// - The section heading is preserved
/// - The fragment content becomes the new section content
/// - Other sections remain unchanged
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_markdown_section_replace() {
    let temp = setup_fixture_dir(fixtures::MARKDOWN);

    // Run common-repo apply
    run_apply_expect_success(&temp, None);

    // Verify the License section still exists
    assert_markdown_has_section(&temp, "README.md", "License", 2);

    // Verify the fragment content is present (dual-licensed)
    assert_markdown_contains(
        &temp,
        "README.md",
        "This project is dual-licensed under MIT OR Apache-2.0.",
    );
    assert_markdown_contains(
        &temp,
        "README.md",
        "See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.",
    );

    // Verify the original content is replaced (not present)
    let content = read_file(&temp, "README.md");
    assert!(
        !content.contains("MIT License - see LICENSE file for details."),
        "Original License content should be replaced"
    );

    // Verify other sections remain unchanged
    assert_markdown_has_section(&temp, "README.md", "My Project", 1);
    assert_markdown_contains(
        &temp,
        "README.md",
        "A sample project for testing markdown merge operations.",
    );
}
