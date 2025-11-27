//! E2E tests for JSON merge operations.
//!
//! These tests invoke the actual CLI binary and validate JSON merge behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_simple() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file
        .write_str(
            r#"{
  "new_key": "new_value",
  "another_key": 123
}"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"{
  "existing_key": "existing_value"
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    assert_eq!(value["existing_key"], "existing_value");
    assert_eq!(value["new_key"], "new_value");
    assert_eq!(value["another_key"], 123);
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_nested_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file
        .write_str(
            r#"{
  "lodash": "^4.17.21",
  "axios": "^1.6.0"
}"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"{
  "name": "test-package",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: dependencies
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    // Original fields preserved
    assert_eq!(value["name"], "test-package");
    assert_eq!(value["dependencies"]["react"], "^18.2.0");
    // New dependencies merged
    assert_eq!(value["dependencies"]["lodash"], "^4.17.21");
    assert_eq!(value["dependencies"]["axios"], "^1.6.0");
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_array_append() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file
        .write_str(r#"["new_item1", "new_item2"]"#)
        .unwrap();

    dest_file
        .write_str(
            r#"{
  "items": ["existing_item1", "existing_item2"]
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: items
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    let items = value["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "existing_item1");
    assert_eq!(items[1], "existing_item2");
    assert_eq!(items[2], "new_item1");
    assert_eq!(items[3], "new_item2");
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_array_with_position() {
    // Note: position field is accepted but array append always adds to end
    // This test verifies the basic append behavior works with position specified
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file.write_str(r#"["new_item"]"#).unwrap();

    dest_file
        .write_str(
            r#"{
  "items": ["existing_item"]
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: items
    append: true
    position: end
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    let items = value["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], "existing_item");
    assert_eq!(items[1], "new_item");
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_deep_nested_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file
        .write_str(
            r#"{
  "host": "db.example.com",
  "port": 5432
}"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"{
  "config": {
    "server": {
      "host": "localhost",
      "port": 8080
    },
    "database": {
      "name": "mydb"
    }
  }
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: config.database
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    // Server config unchanged
    assert_eq!(value["config"]["server"]["host"], "localhost");
    assert_eq!(value["config"]["server"]["port"], 8080);
    // Database config merged
    assert_eq!(value["config"]["database"]["name"], "mydb");
    assert_eq!(value["config"]["database"]["host"], "db.example.com");
    assert_eq!(value["config"]["database"]["port"], 5432);
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_replace_mode() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file.write_str(r#"["only_new"]"#).unwrap();

    dest_file
        .write_str(
            r#"{
  "items": ["old1", "old2", "old3"]
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: items
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    let items = value["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], "only_new");
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_json_merge_creates_missing_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("source.json");
    let dest_file = temp.child("dest.json");

    source_file
        .write_str(
            r#"{
  "key": "value"
}"#,
        )
        .unwrap();

    dest_file
        .write_str(
            r#"{
  "existing": true
}"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- json:
    source: source.json
    dest: dest.json
    path: new.nested.path
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
    let value: serde_json::Value = serde_json::from_str(&merged_content).unwrap();

    assert_eq!(value["existing"], true);
    assert_eq!(value["new"]["nested"]["path"]["key"], "value");
}
