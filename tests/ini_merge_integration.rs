//! Integration tests for INI merge operations using test fixtures.

use common_repo::config::IniMergeOp;
use common_repo::filesystem::MemoryFS;
use common_repo::phases::phase5;

#[test]
fn test_ini_merge_basic_root_level() {
    // Test 1: Basic merge at root level (no section specified)
    // fragment-basic.ini → destination-basic.ini

    let mut fs = MemoryFS::new();

    // Load test fixtures
    let fragment_content = include_str!("testdata/merge-ini-repo/fragment-basic.ini");
    let destination_content = include_str!("testdata/merge-ini-repo/destination-basic.ini");

    fs.add_file_string("fragment-basic.ini", fragment_content)
        .unwrap();
    fs.add_file_string("destination-basic.ini", destination_content)
        .unwrap();

    let op = IniMergeOp {
        source: "fragment-basic.ini".to_string(),
        dest: "destination-basic.ini".to_string(),
        section: None, // No specific section
        append: false,
        allow_duplicates: false,
    };

    phase5::apply_ini_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("destination-basic.ini").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    // [general] section: version should be 2.0 (overwritten), added_field added, existing_field preserved
    assert!(content.contains("[general]"));
    assert!(content.contains("version=2.0"));
    assert!(content.contains("added_field=This field was added by merge"));
    assert!(content.contains("existing_field=This field was already here"));

    // [features] section: old_feature preserved, new_feature added
    assert!(content.contains("[features]"));
    assert!(content.contains("old_feature=enabled"));
    assert!(content.contains("new_feature=enabled"));
}

#[test]
fn test_ini_merge_specific_section() {
    // Test 2: Specific section merge
    // fragment-database.ini → config.ini section database

    let mut fs = MemoryFS::new();

    // Load test fixtures
    let fragment_content = include_str!("testdata/merge-ini-repo/fragment-database.ini");
    let config_content = include_str!("testdata/merge-ini-repo/config.ini");

    fs.add_file_string("fragment-database.ini", fragment_content)
        .unwrap();
    fs.add_file_string("config.ini", config_content).unwrap();

    let op = IniMergeOp {
        source: "fragment-database.ini".to_string(),
        dest: "config.ini".to_string(),
        section: Some("database".to_string()),
        append: false,
        allow_duplicates: false,
    };

    phase5::apply_ini_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("config.ini").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    // [database] section should have merged values
    assert!(content.contains("[database]"));
    assert!(content.contains("host=postgres.example.com"));
    assert!(content.contains("port=5432"));
    assert!(content.contains("ssl_mode=require"));
    assert!(content.contains("pool_size=20"));

    // Other sections should be unchanged
    assert!(content.contains("[app]"));
    assert!(content.contains("name=My Application"));
    assert!(content.contains("[cache]"));
    assert!(content.contains("enabled=true"));
}

#[test]
fn test_ini_merge_append_with_duplicates() {
    // Test 3: Append to section with duplicates allowed
    // fragment-logging.ini → app.ini section logging

    let mut fs = MemoryFS::new();

    // Load test fixtures
    let fragment_content = include_str!("testdata/merge-ini-repo/fragment-logging.ini");
    let app_content = include_str!("testdata/merge-ini-repo/app.ini");

    fs.add_file_string("fragment-logging.ini", fragment_content)
        .unwrap();
    fs.add_file_string("app.ini", app_content).unwrap();

    let op = IniMergeOp {
        source: "fragment-logging.ini".to_string(),
        dest: "app.ini".to_string(),
        section: Some("logging".to_string()),
        append: true,
        allow_duplicates: true,
    };

    phase5::apply_ini_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("app.ini").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    // [logging] section should have both original and fragment values
    assert!(content.contains("[logging]"));
    // Original values
    assert!(content.contains("level=info"));
    assert!(content.contains("handler=console"));
    assert!(content.contains("format=json"));
    // Fragment values (root-level entries merged into logging section)
    assert!(content.contains("handler=file"));
    assert!(content.contains("file_path=/var/log/app/debug.log"));
    assert!(content.contains("rotation=daily"));
}

#[test]
fn test_ini_merge_append_without_duplicates() {
    // Test 4: Append without allowing duplicates (default)
    // fragment-server.ini → server.ini section server

    let mut fs = MemoryFS::new();

    // Load test fixtures
    let fragment_content = include_str!("testdata/merge-ini-repo/fragment-server.ini");
    let server_content = include_str!("testdata/merge-ini-repo/server.ini");

    fs.add_file_string("fragment-server.ini", fragment_content)
        .unwrap();
    fs.add_file_string("server.ini", server_content).unwrap();

    let op = IniMergeOp {
        source: "fragment-server.ini".to_string(),
        dest: "server.ini".to_string(),
        section: Some("server".to_string()),
        append: true,
        allow_duplicates: false,
    };

    phase5::apply_ini_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("server.ini").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    // [server] section should have merged values without duplicate timeout
    assert!(content.contains("[server]"));
    // Original timeout value should be preserved (not overwritten in append mode)
    assert!(content.contains("timeout=60"));
    // New values should be added
    assert!(content.contains("max_connections=1000"));
    assert!(content.contains("keepalive=true"));
}
