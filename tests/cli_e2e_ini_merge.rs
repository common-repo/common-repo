//!
//! These tests invoke the actual CLI binary and validate INI merge behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_ini_merge_with_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.ini");
    let dest_file = temp.child("config.ini");

    // Create source INI fragment
    source_file
        .write_str(
            r#"
[database]
driver = postgresql
port = 5432
"#,
        )
        .unwrap();

    // Create destination INI file
    dest_file
        .write_str(
            r#"
[server]
host = localhost
port = 8080
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- ini:
    source: fragment.ini
    dest: config.ini
    section: database
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
    assert!(merged_content.contains("[server]"));
    assert!(merged_content.contains("host=localhost"));
    assert!(merged_content.contains("port=8080"));
    assert!(merged_content.contains("[database]"));
    assert!(merged_content.contains("driver=postgresql"));
    assert!(merged_content.contains("port=5432"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_ini_merge_without_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("multi.ini");
    let dest_file = temp.child("config.ini");

    // Create source INI fragment with multiple sections
    source_file
        .write_str(
            r#"
[database]
driver = postgresql
port = 5432

[cache]
enabled = true
ttl = 3600
"#,
        )
        .unwrap();

    // Create destination INI file with existing section
    dest_file
        .write_str(
            r#"
[server]
host = localhost
port = 8080
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- ini:
    source: multi.ini
    dest: config.ini
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
    assert!(merged_content.contains("[server]"));
    assert!(merged_content.contains("host=localhost"));
    assert!(merged_content.contains("port=8080"));
    assert!(merged_content.contains("[database]"));
    assert!(merged_content.contains("driver=postgresql"));
    assert!(merged_content.contains("port=5432"));
    assert!(merged_content.contains("[cache]"));
    assert!(merged_content.contains("enabled=true"));
    assert!(merged_content.contains("ttl=3600"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_ini_merge_root_level_into_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("db.ini");
    let dest_file = temp.child("config.ini");

    // Create source INI fragment with root-level entries
    source_file
        .write_str(
            r#"
host = postgres.example.com
port = 5432
ssl_mode = require
"#,
        )
        .unwrap();

    // Create destination INI file
    dest_file
        .write_str(
            r#"
[database]
driver = postgresql
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- ini:
    source: db.ini
    dest: config.ini
    section: database
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
    assert!(merged_content.contains("[database]"));
    assert!(merged_content.contains("driver=postgresql"));
    assert!(merged_content.contains("host=postgres.example.com"));
    assert!(merged_content.contains("port=5432"));
    assert!(merged_content.contains("ssl_mode=require"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_ini_merge_append_mode() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("new.ini");
    let dest_file = temp.child("config.ini");

    // Create source INI fragment
    source_file
        .write_str(
            r#"
[settings]
timeout = 60
debug = true
"#,
        )
        .unwrap();

    // Create destination INI file with overlapping key
    dest_file
        .write_str(
            r#"
[settings]
timeout = 30
host = localhost
"#,
        )
        .unwrap();

    config_file
        .write_str(
            r#"
- ini:
    source: new.ini
    dest: config.ini
    section: settings
    append: true
    allow-duplicates: false
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
    assert!(merged_content.contains("[settings]"));
    assert!(merged_content.contains("host=localhost"));
    assert!(merged_content.contains("debug=true"));
    // In append mode, existing keys should not be overwritten
    assert!(merged_content.contains("timeout=30"));
}
