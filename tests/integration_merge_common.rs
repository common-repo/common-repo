//! Shared test utilities for merge operator integration tests.
//!
//! This module provides common helpers for:
//! - Loading test fixtures from the testdata directories
//! - Running the common-repo CLI with captured output
//! - Asserting on merged file contents across different formats
//!
//! ## Usage
//!
//! ```rust,ignore
//! use integration_merge_common::*;
//!
//! #[test]
//! #[cfg_attr(not(feature = "integration-tests"), ignore)]
//! fn test_yaml_merge() {
//!     let temp = setup_fixture_dir("merge-yaml-repo");
//!     run_apply(&temp, None).expect("apply should succeed");
//!     assert_yaml_contains(&temp, "destination-basic.yml", "version", "2.0");
//! }
//! ```

// Allow dead code for helper functions that will be used by future integration tests
// (JSON, TOML, INI, Markdown merge tests are pending implementation)
#![allow(dead_code)]

use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};

/// Path to the testdata directory containing merge fixtures.
pub const TESTDATA_DIR: &str = "tests/testdata";

/// Names of available merge fixture repositories.
pub mod fixtures {
    pub const YAML: &str = "merge-yaml-repo";
    pub const JSON: &str = "merge-json-repo";
    pub const TOML: &str = "merge-toml-repo";
    pub const INI: &str = "merge-ini-repo";
    pub const MARKDOWN: &str = "merge-markdown-repo";
}

/// Returns the absolute path to a fixture repository.
///
/// # Arguments
///
/// * `fixture_name` - The name of the fixture directory (e.g., "merge-yaml-repo")
///
/// # Returns
///
/// The absolute path to the fixture directory.
pub fn fixture_path(fixture_name: &str) -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set during tests");
    PathBuf::from(manifest_dir)
        .join(TESTDATA_DIR)
        .join(fixture_name)
}

/// Copies a fixture repository to a temporary directory for testing.
///
/// This function copies all files from the specified fixture directory
/// to a new temporary directory, allowing tests to modify files without
/// affecting the original fixtures.
///
/// # Arguments
///
/// * `fixture_name` - The name of the fixture directory to copy
///
/// # Returns
///
/// A `TempDir` containing a copy of the fixture files.
///
/// # Panics
///
/// Panics if the fixture directory doesn't exist or copying fails.
pub fn setup_fixture_dir(fixture_name: &str) -> assert_fs::TempDir {
    let source = fixture_path(fixture_name);
    assert!(
        source.exists(),
        "Fixture directory does not exist: {}",
        source.display()
    );

    let temp = assert_fs::TempDir::new().expect("Failed to create temp directory");

    copy_dir_recursive(&source, temp.path()).expect("Failed to copy fixture directory");

    temp
}

/// Recursively copies a directory and its contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Result of running the common-repo CLI.
#[derive(Debug)]
pub struct ApplyResult {
    /// Whether the command succeeded (exit code 0).
    pub success: bool,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

/// Runs `common-repo apply` in the specified directory.
///
/// # Arguments
///
/// * `dir` - The working directory to run the command in
/// * `config_file` - Optional path to config file (defaults to `.common-repo.yaml`)
///
/// # Returns
///
/// An `ApplyResult` containing the command outcome.
pub fn run_apply(dir: &assert_fs::TempDir, config_file: Option<&str>) -> ApplyResult {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(dir.path()).arg("apply");

    if let Some(config) = config_file {
        cmd.arg("--config").arg(config);
    }

    let output = cmd.output().expect("Failed to execute common-repo apply");

    ApplyResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// Runs `common-repo apply` and asserts it succeeds.
///
/// # Panics
///
/// Panics if the command fails, printing stdout and stderr.
pub fn run_apply_expect_success(dir: &assert_fs::TempDir, config_file: Option<&str>) {
    let result = run_apply(dir, config_file);
    if !result.success {
        panic!(
            "common-repo apply failed!\nstdout: {}\nstderr: {}",
            result.stdout, result.stderr
        );
    }
}

/// Reads a file from the test directory as a string.
///
/// # Arguments
///
/// * `dir` - The test directory
/// * `path` - Relative path to the file within the directory
///
/// # Returns
///
/// The file contents as a string.
///
/// # Panics
///
/// Panics if the file doesn't exist or can't be read.
pub fn read_file(dir: &assert_fs::TempDir, path: &str) -> String {
    let file_path = dir.path().join(path);
    std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read file {}: {}", file_path.display(), e))
}

/// Checks if a file exists in the test directory.
pub fn file_exists(dir: &assert_fs::TempDir, path: &str) -> bool {
    dir.path().join(path).exists()
}

// =============================================================================
// YAML Assertion Helpers
// =============================================================================

/// Parses a YAML file from the test directory.
///
/// # Returns
///
/// The parsed YAML value.
///
/// # Panics
///
/// Panics if the file doesn't exist or isn't valid YAML.
pub fn parse_yaml(dir: &assert_fs::TempDir, path: &str) -> serde_yaml::Value {
    let content = read_file(dir, path);
    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML from {}: {}", path, e))
}

/// Asserts that a YAML file contains a specific key-value pair at the root level.
pub fn assert_yaml_contains(dir: &assert_fs::TempDir, path: &str, key: &str, expected: &str) {
    let yaml = parse_yaml(dir, path);
    let value = yaml
        .get(key)
        .unwrap_or_else(|| panic!("YAML key '{}' not found in {}", key, path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "YAML value for '{}' is not a string in {}: {:?}",
            key, path, value
        )
    });

    assert_eq!(
        actual, expected,
        "YAML key '{}' in {} has unexpected value",
        key, path
    );
}

/// Asserts that a YAML file contains a nested value at the specified path.
///
/// Path notation uses dots: `metadata.labels.app`
pub fn assert_yaml_nested(
    dir: &assert_fs::TempDir,
    file_path: &str,
    yaml_path: &str,
    expected: &str,
) {
    let yaml = parse_yaml(dir, file_path);
    let value = get_nested_yaml(&yaml, yaml_path)
        .unwrap_or_else(|| panic!("YAML path '{}' not found in {}", yaml_path, file_path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "YAML value at '{}' is not a string in {}: {:?}",
            yaml_path, file_path, value
        )
    });

    assert_eq!(
        actual, expected,
        "YAML path '{}' in {} has unexpected value",
        yaml_path, file_path
    );
}

/// Gets a nested value from a YAML structure using dot notation.
fn get_nested_yaml<'a>(yaml: &'a serde_yaml::Value, path: &str) -> Option<&'a serde_yaml::Value> {
    let mut current = yaml;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

/// Asserts that a YAML array contains a specific string value.
pub fn assert_yaml_array_contains(dir: &assert_fs::TempDir, path: &str, expected: &str) {
    let yaml = parse_yaml(dir, path);
    let arr = yaml
        .as_sequence()
        .unwrap_or_else(|| panic!("YAML file {} is not an array", path));

    let found = arr.iter().any(|v| v.as_str() == Some(expected));
    assert!(
        found,
        "YAML array in {} does not contain '{}'",
        path, expected
    );
}

// =============================================================================
// JSON Assertion Helpers
// =============================================================================

/// Parses a JSON file from the test directory.
pub fn parse_json(dir: &assert_fs::TempDir, path: &str) -> serde_json::Value {
    let content = read_file(dir, path);
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse JSON from {}: {}", path, e))
}

/// Asserts that a JSON file contains a specific key-value pair at the root level.
pub fn assert_json_contains(dir: &assert_fs::TempDir, path: &str, key: &str, expected: &str) {
    let json = parse_json(dir, path);
    let value = json
        .get(key)
        .unwrap_or_else(|| panic!("JSON key '{}' not found in {}", key, path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "JSON value for '{}' is not a string in {}: {:?}",
            key, path, value
        )
    });

    assert_eq!(
        actual, expected,
        "JSON key '{}' in {} has unexpected value",
        key, path
    );
}

/// Asserts that a JSON file contains a nested value at the specified path.
///
/// Path notation uses dots: `config.database.host`
pub fn assert_json_nested(
    dir: &assert_fs::TempDir,
    file_path: &str,
    json_path: &str,
    expected: &str,
) {
    let json = parse_json(dir, file_path);
    let value = get_nested_json(&json, json_path)
        .unwrap_or_else(|| panic!("JSON path '{}' not found in {}", json_path, file_path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "JSON value at '{}' is not a string in {}: {:?}",
            json_path, file_path, value
        )
    });

    assert_eq!(
        actual, expected,
        "JSON path '{}' in {} has unexpected value",
        json_path, file_path
    );
}

/// Gets a nested value from a JSON structure using dot notation.
fn get_nested_json<'a>(json: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = json;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

/// Asserts that a JSON array at a path contains elements in the expected order.
pub fn assert_json_array_order(
    dir: &assert_fs::TempDir,
    file_path: &str,
    json_path: &str,
    key: &str,
    expected_order: &[&str],
) {
    let json = parse_json(dir, file_path);
    let arr = if json_path.is_empty() {
        json.as_array()
    } else {
        get_nested_json(&json, json_path).and_then(|v| v.as_array())
    };

    let arr =
        arr.unwrap_or_else(|| panic!("JSON path '{}' is not an array in {}", json_path, file_path));

    let actual_order: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get(key).and_then(|v| v.as_str()))
        .collect();

    assert_eq!(
        actual_order, expected_order,
        "JSON array order mismatch at '{}' in {}",
        json_path, file_path
    );
}

// =============================================================================
// TOML Assertion Helpers
// =============================================================================

/// Parses a TOML file from the test directory.
pub fn parse_toml(dir: &assert_fs::TempDir, path: &str) -> toml::Value {
    let content = read_file(dir, path);
    content
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse TOML from {}: {}", path, e))
}

/// Asserts that a TOML file contains a specific key-value pair at the root level.
pub fn assert_toml_contains(dir: &assert_fs::TempDir, path: &str, key: &str, expected: &str) {
    let toml = parse_toml(dir, path);
    let value = toml
        .get(key)
        .unwrap_or_else(|| panic!("TOML key '{}' not found in {}", key, path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "TOML value for '{}' is not a string in {}: {:?}",
            key, path, value
        )
    });

    assert_eq!(
        actual, expected,
        "TOML key '{}' in {} has unexpected value",
        key, path
    );
}

/// Asserts that a TOML file contains a nested value at the specified path.
pub fn assert_toml_nested(
    dir: &assert_fs::TempDir,
    file_path: &str,
    toml_path: &str,
    expected: &str,
) {
    let toml = parse_toml(dir, file_path);
    let value = get_nested_toml(&toml, toml_path)
        .unwrap_or_else(|| panic!("TOML path '{}' not found in {}", toml_path, file_path));

    let actual = value.as_str().unwrap_or_else(|| {
        panic!(
            "TOML value at '{}' is not a string in {}: {:?}",
            toml_path, file_path, value
        )
    });

    assert_eq!(
        actual, expected,
        "TOML path '{}' in {} has unexpected value",
        toml_path, file_path
    );
}

/// Gets a nested value from a TOML structure using dot notation.
fn get_nested_toml<'a>(toml: &'a toml::Value, path: &str) -> Option<&'a toml::Value> {
    let mut current = toml;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

/// Asserts that a TOML array at a path contains a specific string value.
pub fn assert_toml_array_contains(
    dir: &assert_fs::TempDir,
    file_path: &str,
    toml_path: &str,
    expected: &str,
) {
    let toml = parse_toml(dir, file_path);
    let value = get_nested_toml(&toml, toml_path)
        .unwrap_or_else(|| panic!("TOML path '{}' not found in {}", toml_path, file_path));

    let arr = value
        .as_array()
        .unwrap_or_else(|| panic!("TOML path '{}' is not an array in {}", toml_path, file_path));

    let found = arr.iter().any(|v| v.as_str() == Some(expected));
    assert!(
        found,
        "TOML array at '{}' in {} does not contain '{}'",
        toml_path, file_path, expected
    );
}

// =============================================================================
// INI Assertion Helpers
// =============================================================================

/// Parses an INI file from the test directory.
pub fn parse_ini(dir: &assert_fs::TempDir, path: &str) -> ini::Ini {
    let file_path = dir.path().join(path);
    ini::Ini::load_from_file(&file_path)
        .unwrap_or_else(|e| panic!("Failed to parse INI from {}: {}", path, e))
}

/// Asserts that an INI file contains a specific key-value pair in a section.
pub fn assert_ini_contains(
    dir: &assert_fs::TempDir,
    path: &str,
    section: &str,
    key: &str,
    expected: &str,
) {
    let ini = parse_ini(dir, path);
    let section_data = ini
        .section(Some(section))
        .unwrap_or_else(|| panic!("INI section '{}' not found in {}", section, path));

    let actual = section_data.get(key).unwrap_or_else(|| {
        panic!(
            "INI key '{}' not found in section '{}' of {}",
            key, section, path
        )
    });

    assert_eq!(
        actual, expected,
        "INI key '{}' in section '{}' of {} has unexpected value",
        key, section, path
    );
}

/// Asserts that an INI file has a section.
pub fn assert_ini_has_section(dir: &assert_fs::TempDir, path: &str, section: &str) {
    let ini = parse_ini(dir, path);
    assert!(
        ini.section(Some(section)).is_some(),
        "INI file {} does not have section '{}'",
        path,
        section
    );
}

// =============================================================================
// Markdown Assertion Helpers
// =============================================================================

/// Asserts that a Markdown file contains specific text.
pub fn assert_markdown_contains(dir: &assert_fs::TempDir, path: &str, expected: &str) {
    let content = read_file(dir, path);
    assert!(
        content.contains(expected),
        "Markdown file {} does not contain expected text: {}",
        path,
        expected
    );
}

/// Asserts that a Markdown file contains a specific section heading.
pub fn assert_markdown_has_section(dir: &assert_fs::TempDir, path: &str, heading: &str, level: u8) {
    let content = read_file(dir, path);
    let prefix = "#".repeat(level as usize);
    let expected = format!("{} {}", prefix, heading);

    assert!(
        content.contains(&expected),
        "Markdown file {} does not contain heading: {}",
        path,
        expected
    );
}

/// Asserts that one section appears before another in a Markdown file.
pub fn assert_markdown_section_order(
    dir: &assert_fs::TempDir,
    path: &str,
    first_heading: &str,
    second_heading: &str,
) {
    let content = read_file(dir, path);
    let first_pos = content.find(first_heading).unwrap_or_else(|| {
        panic!(
            "Markdown file {} does not contain heading: {}",
            path, first_heading
        )
    });
    let second_pos = content.find(second_heading).unwrap_or_else(|| {
        panic!(
            "Markdown file {} does not contain heading: {}",
            path, second_heading
        )
    });

    assert!(
        first_pos < second_pos,
        "In {}, '{}' should appear before '{}' but it doesn't",
        path,
        first_heading,
        second_heading
    );
}

// =============================================================================
// Generic Content Assertions
// =============================================================================

/// Asserts that file content contains a specific string.
pub fn assert_file_contains(dir: &assert_fs::TempDir, path: &str, expected: &str) {
    let content = read_file(dir, path);
    assert!(
        content.contains(expected),
        "File {} does not contain: {}",
        path,
        expected
    );
}

/// Asserts that file content does not contain a specific string.
pub fn assert_file_not_contains(dir: &assert_fs::TempDir, path: &str, not_expected: &str) {
    let content = read_file(dir, path);
    assert!(
        !content.contains(not_expected),
        "File {} unexpectedly contains: {}",
        path,
        not_expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_path_exists() {
        let yaml_path = fixture_path(fixtures::YAML);
        assert!(yaml_path.exists(), "YAML fixtures should exist");
    }

    #[test]
    fn test_setup_fixture_dir() {
        let temp = setup_fixture_dir(fixtures::YAML);

        // Verify key files were copied
        assert!(temp.path().join(".common-repo.yaml").exists());
        assert!(temp.path().join("fragment-basic.yml").exists());
        assert!(temp.path().join("destination-basic.yml").exists());
    }

    #[test]
    fn test_read_file() {
        let temp = setup_fixture_dir(fixtures::YAML);
        let content = read_file(&temp, "destination-basic.yml");

        assert!(content.contains("version:"));
    }

    #[test]
    fn test_parse_yaml() {
        let temp = setup_fixture_dir(fixtures::YAML);
        let yaml = parse_yaml(&temp, "destination-basic.yml");

        assert!(yaml.get("version").is_some());
    }

    #[test]
    fn test_parse_json() {
        let temp = setup_fixture_dir(fixtures::JSON);
        let json = parse_json(&temp, "destination-basic.json");

        assert!(json.get("version").is_some());
    }

    #[test]
    fn test_parse_toml() {
        let temp = setup_fixture_dir(fixtures::TOML);
        let toml = parse_toml(&temp, "destination-basic.toml");

        assert!(toml.get("version").is_some());
    }

    #[test]
    fn test_parse_ini() {
        let temp = setup_fixture_dir(fixtures::INI);
        let ini = parse_ini(&temp, "destination-basic.ini");

        assert!(ini.section(Some("general")).is_some());
    }
}
