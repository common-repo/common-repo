//!
//! These tests invoke the actual CLI binary and validate YAML merge behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_simple() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("dest.yaml");

    source_file
        .write_str("new_key: new_value\nanother_key: 123")
        .unwrap();

    dest_file.write_str("existing_key: existing_value").unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: dest.yaml
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
    assert!(merged_content.contains("existing_key: existing_value"));
    assert!(merged_content.contains("new_key: new_value"));
    assert!(merged_content.contains("another_key: 123"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_nested_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("dest.yaml");

    source_file.write_str("- item1\n- item2").unwrap();

    dest_file
        .write_str("root:\n  nested:\n    array:\n      - existing_item")
        .unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: dest.yaml
    path: root.nested.array
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
    assert!(merged_content.contains("existing_item"));
    assert!(merged_content.contains("item1"));
    assert!(merged_content.contains("item2"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_array_append() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("dest.yaml");

    source_file.write_str("- new_item1\n- new_item2").unwrap();

    dest_file
        .write_str("- existing_item1\n- existing_item2")
        .unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: dest.yaml
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
    assert!(merged_content.contains("existing_item1"));
    assert!(merged_content.contains("existing_item2"));
    assert!(merged_content.contains("new_item1"));
    assert!(merged_content.contains("new_item2"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_array_replace() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("dest.yaml");

    source_file.write_str("- new_item1\n- new_item2").unwrap();

    dest_file
        .write_str("- existing_item1\n- existing_item2")
        .unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: dest.yaml
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
    assert!(merged_content.contains("new_item1"));
    assert!(merged_content.contains("new_item2"));
    assert!(!merged_content.contains("existing_item1"));
    assert!(!merged_content.contains("existing_item2"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_no_duplicates() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("dest.yaml");

    source_file.write_str("- item1\n- item2\n- item3").unwrap();

    dest_file.write_str("- item1\n- existing_item").unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: dest.yaml
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

    let item1_count = merged_content.matches("item1").count();
    assert_eq!(item1_count, 1, "item1 should appear exactly once");

    assert!(merged_content.contains("item2"));
    assert!(merged_content.contains("item3"));
    assert!(merged_content.contains("existing_item"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_yaml_merge_create_dest() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.yaml");
    let dest_file = temp.child("new_dest.yaml");

    source_file.write_str("key1: value1\nkey2: value2").unwrap();

    config_file
        .write_str(
            r#"
- yaml:
    source: source.yaml
    dest: new_dest.yaml
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

    assert!(dest_file.path().exists());

    let merged_content = std::fs::read_to_string(dest_file.path()).unwrap();
    assert!(merged_content.contains("key1: value1"));
    assert!(merged_content.contains("key2: value2"));
}
