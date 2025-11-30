//! Integration tests for YAML merge operations.

use common_repo::config::{ArrayMergeMode, YamlMergeOp};
use common_repo::filesystem::MemoryFS;
use common_repo::merge::yaml::apply_yaml_merge_operation;

#[test]
fn test_yaml_merge_simple_keys() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "new_key: new_value\nanother_key: 123")
        .unwrap();

    fs.add_file_string("dest.yaml", "existing_key: existing_value")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: None,
        append: false,
        array_mode: None,
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    assert!(content.contains("existing_key: existing_value"));
    assert!(content.contains("new_key: new_value"));
    assert!(content.contains("another_key: 123"));
}

#[test]
fn test_yaml_merge_nested_path() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "- item1\n- item2")
        .unwrap();

    fs.add_file_string(
        "dest.yaml",
        "root:\n  nested:\n    array:\n      - existing_item",
    )
    .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: Some("root.nested.array".to_string()),
        append: true,
        array_mode: None,
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    assert!(content.contains("existing_item"));
    assert!(content.contains("item1"));
    assert!(content.contains("item2"));
}

#[test]
fn test_yaml_merge_array_append() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "- new_item1\n- new_item2")
        .unwrap();

    fs.add_file_string("dest.yaml", "- existing_item1\n- existing_item2")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: None,
        append: true,
        array_mode: None,
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    assert!(content.contains("existing_item1"));
    assert!(content.contains("existing_item2"));
    assert!(content.contains("new_item1"));
    assert!(content.contains("new_item2"));
}

#[test]
fn test_yaml_merge_array_replace() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "- new_item1\n- new_item2")
        .unwrap();

    fs.add_file_string("dest.yaml", "- existing_item1\n- existing_item2")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: None,
        append: false,
        array_mode: None,
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    assert!(content.contains("new_item1"));
    assert!(content.contains("new_item2"));
    assert!(!content.contains("existing_item1"));
    assert!(!content.contains("existing_item2"));
}

#[test]
fn test_yaml_merge_no_duplicates() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "- item1\n- item2\n- item3")
        .unwrap();

    fs.add_file_string("dest.yaml", "- item1\n- existing_item")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: None,
        append: true,
        array_mode: Some(ArrayMergeMode::AppendUnique),
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    let item1_count = content.matches("item1").count();
    assert_eq!(item1_count, 1, "item1 should appear exactly once");

    assert!(content.contains("item2"));
    assert!(content.contains("item3"));
    assert!(content.contains("existing_item"));
}

#[test]
fn test_yaml_merge_allow_duplicates() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "- item1\n- item2")
        .unwrap();

    fs.add_file_string("dest.yaml", "- item1\n- existing_item")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "dest.yaml".to_string(),
        path: None,
        append: true,
        array_mode: Some(ArrayMergeMode::Append),
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    let result = fs.get_file("dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    let item1_count = content.matches("item1").count();
    assert_eq!(
        item1_count, 2,
        "item1 should appear twice when duplicates allowed"
    );

    assert!(content.contains("item2"));
    assert!(content.contains("existing_item"));
}

#[test]
fn test_yaml_merge_create_dest() {
    let mut fs = MemoryFS::new();

    fs.add_file_string("source.yaml", "key1: value1\nkey2: value2")
        .unwrap();

    let op = YamlMergeOp {
        source: "source.yaml".to_string(),
        dest: "new_dest.yaml".to_string(),
        path: None,
        append: false,
        array_mode: None,
    };

    apply_yaml_merge_operation(&mut fs, &op).unwrap();

    assert!(fs.exists("new_dest.yaml"));

    let result = fs.get_file("new_dest.yaml").unwrap();
    let content = String::from_utf8_lossy(&result.content);

    assert!(content.contains("key1: value1"));
    assert!(content.contains("key2: value2"));
}
