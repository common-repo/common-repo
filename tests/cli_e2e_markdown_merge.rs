//! E2E tests for Markdown merge operations.
//!
//! These tests invoke the actual CLI binary and validate Markdown merge behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_append_to_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"### New Feature

- Added support for JSON
- Performance improvements
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Features

### Basic Features

- Configuration management
- API integration

## Installation

Run `npm install`.
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: Features
    level: 2
    append: true
    position: end
"###,
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

    // Original content preserved
    assert!(merged_content.contains("# My Project"));
    assert!(merged_content.contains("## Features"));
    assert!(merged_content.contains("### Basic Features"));
    assert!(merged_content.contains("- Configuration management"));
    // New content appended
    assert!(merged_content.contains("### New Feature"));
    assert!(merged_content.contains("- Added support for JSON"));
    // Installation section still exists
    assert!(merged_content.contains("## Installation"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_insert_at_start() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"### Prerequisites

- Node.js >= 18.0.0
- npm >= 9.0.0
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Installation

### Install via npm

```bash
npm install my-project
```

## Usage

See examples.
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: Installation
    level: 2
    append: true
    position: start
"###,
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

    // Check structure
    assert!(merged_content.contains("# My Project"));
    assert!(merged_content.contains("## Installation"));
    // Prerequisites should appear in the Installation section
    assert!(merged_content.contains("### Prerequisites"));
    assert!(merged_content.contains("- Node.js >= 18.0.0"));
    // Original install content still present
    assert!(merged_content.contains("### Install via npm"));
    assert!(merged_content.contains("npm install my-project"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_create_new_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"We welcome contributions!

1. Fork the repository
2. Create a feature branch
3. Submit a pull request
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Features

- Feature A
- Feature B

## License

MIT License
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: Contributing
    level: 2
    create-section: true
"###,
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

    // Original sections preserved
    assert!(merged_content.contains("# My Project"));
    assert!(merged_content.contains("## Features"));
    assert!(merged_content.contains("## License"));
    // New section created
    assert!(merged_content.contains("## Contributing"));
    assert!(merged_content.contains("We welcome contributions!"));
    assert!(merged_content.contains("1. Fork the repository"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_replace_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"Apache 2.0 License - see LICENSE file for details.

Copyright 2024 My Organization.
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Features

- Feature A

## License

MIT License - old content that should be replaced.
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: License
    level: 2
    append: false
"###,
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

    // Original sections preserved
    assert!(merged_content.contains("# My Project"));
    assert!(merged_content.contains("## Features"));
    assert!(merged_content.contains("## License"));
    // Old license content replaced
    assert!(!merged_content.contains("MIT License - old content"));
    // New license content present
    assert!(merged_content.contains("Apache 2.0 License"));
    assert!(merged_content.contains("Copyright 2024"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_different_header_levels() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"Content for level 3 section.

- Item 1
- Item 2
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Features

### Basic

- Feature A

### Advanced

- Feature B

## Other
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: Advanced
    level: 3
    append: true
"###,
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

    // Structure preserved
    assert!(merged_content.contains("### Basic"));
    assert!(merged_content.contains("### Advanced"));
    // New content added to Advanced section
    assert!(merged_content.contains("Content for level 3 section"));
    assert!(merged_content.contains("- Feature B"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cli_markdown_merge_preserves_code_blocks() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("fragment.md");
    let dest_file = temp.child("README.md");

    source_file
        .write_str(
            r###"### Python Example

```python
import mylib
mylib.run()
```
"###,
        )
        .unwrap();

    dest_file
        .write_str(
            r###"# My Project

## Examples

### JavaScript Example

```javascript
const mylib = require('mylib');
mylib.run();
```

## License

MIT
"###,
        )
        .unwrap();

    config_file
        .write_str(
            r###"
- markdown:
    source: fragment.md
    dest: README.md
    section: Examples
    level: 2
    append: true
    position: end
"###,
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

    // Both code blocks preserved
    assert!(merged_content.contains("```javascript"));
    assert!(merged_content.contains("const mylib = require('mylib');"));
    assert!(merged_content.contains("```python"));
    assert!(merged_content.contains("import mylib"));
}
