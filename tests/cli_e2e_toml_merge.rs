//!
//! These tests invoke the actual CLI binary and validate TOML merge behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_simple() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
new_key = "new_value"
another_key = 123
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
existing_key = "existing_value"
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    assert!(merged_content.contains(r#"existing_key = "existing_value""#));
    assert!(merged_content.contains(r#"new_key = "new_value""#));
    assert!(merged_content.contains("another_key = 123"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_nested_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
enabled = true
timeout = 30
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
[server]
host = "localhost"

[database]
name = "mydb"
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: server
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();

    // Should have server section with new fields
    assert!(merged_content.contains("[server]"));
    assert!(merged_content.contains(r#"host = "localhost""#));
    assert!(merged_content.contains("enabled = true"));
    assert!(merged_content.contains("timeout = 30"));
    // Should still have database section
    assert!(merged_content.contains("[database]"));
    assert!(merged_content.contains(r#"name = "mydb""#));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_array_mode_replace() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
[package]
items = ["new1", "new2"]
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
[package]
items = ["old1", "old2"]
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
    append: false
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    let value: toml::Value = merged_content.parse().unwrap();
    let items = value["package"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_str(), Some("new1"));
    assert_eq!(items[1].as_str(), Some("new2"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_array_mode_append() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
[package]
items = ["new1", "new2"]
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
[package]
items = ["old1", "old2"]
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
    append: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    println!("Merged content:\n{}", merged_content);
    let value: toml::Value = merged_content.parse().unwrap();
    let items = value["package"]["items"].as_array().unwrap();
    println!("Array length: {}, items: {:?}", items.len(), items);
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].as_str(), Some("old1"));
    assert_eq!(items[1].as_str(), Some("old2"));
    assert_eq!(items[2].as_str(), Some("new1"));
    assert_eq!(items[3].as_str(), Some("new2"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_array_mode_append_unique() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
[package]
items = ["item1", "item2", "item3"]
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
[package]
items = ["item1", "item4"]
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
    append: false
    array_mode: append_unique
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    let value: toml::Value = merged_content.parse().unwrap();
    let items = value["package"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].as_str(), Some("item1"));
    assert_eq!(items[1].as_str(), Some("item4"));
    assert_eq!(items[2].as_str(), Some("item2"));
    assert_eq!(items[3].as_str(), Some("item3"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_backward_compatibility_append_bool() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
[package]
items = ["new1"]
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
[package]
items = ["old1"]
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
    append: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    println!("Backward compat merged content:\n{}", merged_content);
    let value: toml::Value = merged_content.parse().unwrap();
    let items = value["package"]["items"].as_array().unwrap();
    println!(
        "Backward compat array length: {}, items: {:?}",
        items.len(),
        items
    );
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_str(), Some("old1"));
    assert_eq!(items[1].as_str(), Some("new1"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_toml_merge_preserve_comments() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.toml");
    let dest_file = temp.child("dest.toml");

    source_file
        .write_str(
            r#"
# New package metadata
name = "new-package"
version = "1.0.0"
"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"
# Original package
name = "old-package"
version = "0.1.0"

# Dependencies section
[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- toml:
    source: source.toml
    dest: dest.toml
    path: ""
    preserve-comments: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();

    // Should contain merged values
    assert!(merged_content.contains(r#"name = "new-package""#));
    assert!(merged_content.contains(r#"version = "1.0.0""#));
    assert!(merged_content.contains("[dependencies]"));
    assert!(merged_content.contains(r#"serde = "1.0""#));

    // Should be formatted (taplo formatting applied)
    let parsed: toml::Value = merged_content.parse().unwrap();
    assert_eq!(parsed["name"].as_str(), Some("new-package"));
    assert_eq!(parsed["version"].as_str(), Some("1.0.0"));
}
