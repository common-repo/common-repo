# CLI Testing Strategy

## Overview

The CLI testing strategy builds on the existing library tests (186 total tests) and adds CLI-specific testing layers to ensure the command-line interface is reliable, user-friendly, and correct.

## Current Testing Infrastructure

### Existing Test Coverage

✅ **Library Unit Tests** - 167 tests in `src/lib.rs`
- All modules well-tested: cache, config, error, filesystem, git, operators, path, phases, repository
- High coverage of core functionality

✅ **Integration Tests** - 5 tests in `tests/integration_test.rs`
- Feature-gated with `integration-tests` feature
- Test real repository cloning
- Respect `SKIP_NETWORK_TESTS` environment variable

✅ **Schema Parsing Tests** - 14 datatest tests in `tests/schema_parsing_test.rs`
- Automatically discover test cases from YAML files
- Test both current and original schema formats

### Test Infrastructure Already Available
- `tempfile` crate for temporary directories
- Feature-gated integration tests pattern
- Environment variable test control
- Mock implementations in repository.rs

---

## CLI Testing Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Manual/Exploratory Testing                        │
│ - Real-world usage scenarios                               │
│ - UX validation                                            │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: Full CLI Tests (assert_cmd)                │
│ - Full CLI binary invocation                              │
│ - Real file I/O, real processes                           │
│ - Exit codes, stdout/stderr validation                    │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: Integration Tests (existing pattern)             │
│ - Command module with real backend                        │
│ - Feature-gated network tests                             │
│ - Temporary directories                                    │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Unit Tests (in command modules)                  │
│ - Command logic with mocked backend                       │
│ - Argument parsing validation                             │
│ - Output formatting                                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Layer 1: Command Unit Tests

### Purpose
Test individual command logic in isolation with mocked dependencies.

### Location
- `src/commands/apply.rs` - tests module
- `src/commands/validate.rs` - tests module
- etc.

### Testing Approach

**Mock the library layer** - Use traits and mock implementations:

```rust
// Example: src/commands/apply.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phases::Phase1;

    // Mock that simulates successful execution
    struct MockPhaseExecutor {
        should_fail: bool,
    }

    impl MockPhaseExecutor {
        fn new() -> Self {
            Self { should_fail: false }
        }

        fn with_failure() -> Self {
            Self { should_fail: true }
        }
    }

    #[test]
    fn test_apply_command_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");

        // Write test config
        std::fs::write(&config_path, "- include: ['**/*']").unwrap();

        // Test apply logic (without actually running phases)
        let result = validate_apply_args(&ApplyArgs {
            config: Some(config_path),
            dry_run: false,
            verbose: false,
            force: false,
            no_cache: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_command_missing_config() {
        let result = validate_apply_args(&ApplyArgs {
            config: Some(PathBuf::from("/nonexistent/config.yaml")),
            dry_run: false,
            verbose: false,
            force: false,
            no_cache: false,
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
```

### What to Test

**For each command**:
- ✅ Valid argument combinations
- ✅ Invalid argument combinations
- ✅ Argument defaults
- ✅ Argument validation
- ✅ Error handling paths
- ✅ Output formatting
- ✅ Flag combinations

**Example test cases for `apply` command**:
```rust
#[test] fn test_apply_valid_config()
#[test] fn test_apply_missing_config()
#[test] fn test_apply_invalid_yaml()
#[test] fn test_apply_dry_run_mode()
#[test] fn test_apply_force_flag()
#[test] fn test_apply_no_cache_flag()
#[test] fn test_apply_verbose_output()
#[test] fn test_apply_quiet_mode()
#[test] fn test_apply_conflicting_flags()
```

---

## Layer 2: Command Integration Tests

### Purpose
Test commands with real backend but controlled environment.

### Location
- `tests/cli_integration_test.rs` (new file)

### Testing Approach

Use the same pattern as existing integration tests:

```rust
// tests/cli_integration_test.rs

use common_repo::commands::apply::run_apply;
use tempfile::TempDir;
use std::env;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_command_with_real_repo() {
    if env::var("SKIP_NETWORK_TESTS").is_ok() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let output_dir = temp_dir.path().join("output");

    // Create test config
    let config = r#"
- repo:
    url: https://github.com/rust-lang/mdBook
    ref: v0.4.35
- include:
    patterns: ["README.md"]
"#;

    let config_path = temp_dir.path().join(".common-repo.yaml");
    std::fs::write(&config_path, config).unwrap();

    // Run apply command (using library, not CLI binary)
    let result = run_apply(&ApplyArgs {
        config: Some(config_path),
        output: Some(output_dir.clone()),
        cache_root: Some(cache_dir),
        dry_run: false,
        verbose: true,
        force: true,
        no_cache: false,
    });

    assert!(result.is_ok());
    assert!(output_dir.join("README.md").exists());
}
```

### What to Test

- ✅ Real file operations (with tempfile)
- ✅ Real git operations (feature-gated)
- ✅ Cache behavior
- ✅ Multiple commands in sequence
- ✅ Complex configurations
- ✅ Error recovery

**Example test cases**:
```rust
#[test] fn test_apply_then_validate()
#[test] fn test_init_then_apply()
#[test] fn test_cache_list_after_apply()
#[test] fn test_apply_with_inheritance_chain()
#[test] fn test_apply_with_merge_operations()
```

---

## Layer 3: Full CLI Tests

### Purpose
Test the actual CLI binary as users would invoke it.

### Required Crates

Add to `Cargo.toml`:
```toml
[dev-dependencies]
assert_cmd = "2.0"    # CLI testing framework
predicates = "3.1"    # Assertions for output
assert_fs = "1.1"     # Filesystem assertions
tempfile = "3.0"      # Already present
```

### Testing Approach

Use `assert_cmd` to invoke the binary and validate output:

```rust
// tests/cli_e2e_test.rs

use assert_cmd::Command;
use predicates::prelude::*;
use assert_fs::prelude::*;

#[test]
fn test_apply_command_help() {
    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.arg("apply")
       .arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("Apply the .common-repo.yaml configuration"));
}

#[test]
fn test_apply_missing_config_file() {
    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.arg("apply")
       .arg("--config")
       .arg("/nonexistent/config.yaml")
       .assert()
       .failure()
       .code(2)  // Configuration error
       .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_apply_with_valid_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write minimal valid config
    config_file.write_str(r#"
- include:
    patterns: ["README.md"]
"#).unwrap();

    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.arg("apply")
       .arg("--config")
       .arg(config_file.path())
       .arg("--dry-run")
       .current_dir(temp.path())
       .assert()
       .success()
       .stdout(predicate::str::contains("✅"));
}

#[test]
fn test_validate_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write invalid YAML
    config_file.write_str("invalid: yaml: content:").unwrap();

    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.arg("validate")
       .arg(config_file.path())
       .assert()
       .failure()
       .code(2)
       .stderr(predicate::str::contains("YAML"));
}

#[test]
fn test_cache_list_empty() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.env("COMMON_REPO_CACHE", temp.path())
       .arg("cache")
       .arg("list")
       .assert()
       .success()
       .stdout(predicate::str::contains("No cached repositories"));
}

#[test]
fn test_init_creates_config_file() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.current_dir(temp.path())
       .arg("init")
       .assert()
       .success()
       .stdout(predicate::str::contains("Created .common-repo.yaml"));

    // Verify file was created
    temp.child(".common-repo.yaml").assert(predicate::path::exists());
}

#[test]
fn test_init_does_not_overwrite_without_force() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    config_file.write_str("existing content").unwrap();

    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.current_dir(temp.path())
       .arg("init")
       .assert()
       .failure()
       .stderr(predicate::str::contains("already exists"));

    // Verify original content preserved
    config_file.assert("existing content");
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("common-repo").unwrap();

    cmd.arg("--version")
       .assert()
       .success()
       .stdout(predicate::str::contains("common-repo"));
}
```

### What to Test

**For each command**:
- ✅ Help output (`--help`)
- ✅ Success cases
- ✅ Error cases with correct exit codes
- ✅ Stdout and stderr output
- ✅ File creation/modification
- ✅ Environment variable handling
- ✅ Interactive prompts (with input simulation)

**Cross-cutting concerns**:
- ✅ Global flags (`--help`, `--version`, `--color`)
- ✅ Environment variables
- ✅ Exit codes
- ✅ Error message quality
- ✅ Output formatting

### Exit Code Testing

Test that exit codes match the spec:

```rust
#[test]
fn test_exit_codes() {
    // Success
    Command::cargo_bin("common-repo").unwrap()
        .arg("validate")
        .arg("valid.yaml")
        .assert()
        .code(0);

    // General error
    Command::cargo_bin("common-repo").unwrap()
        .arg("invalid-command")
        .assert()
        .code(1);

    // Configuration error
    Command::cargo_bin("common-repo").unwrap()
        .arg("apply")
        .arg("--config")
        .arg("missing.yaml")
        .assert()
        .code(2);

    // Network error (mock with invalid URL)
    // Test would need network mocking...
}
```

---

## Test Organization

### Directory Structure

```
tests/
├── cli_e2e_apply.rs              # Layer 3: E2E for apply command
├── cli_e2e_cache.rs              # Layer 3: E2E for cache commands
├── cli_e2e_check.rs              # Layer 3: E2E for check command
├── cli_e2e_diff.rs               # Layer 3: E2E for diff command
├── cli_e2e_info.rs               # Layer 3: E2E for info command
├── cli_e2e_init.rs               # Layer 3: E2E for init command
├── cli_e2e_ini_merge.rs          # Layer 3: E2E for INI file merging
├── cli_e2e_json_merge.rs         # Layer 3: E2E for JSON file merging
├── cli_e2e_ls.rs                 # Layer 3: E2E for ls command
├── cli_e2e_markdown_merge.rs     # Layer 3: E2E for Markdown file merging
├── cli_e2e_toml_merge.rs         # Layer 3: E2E for TOML file merging
├── cli_e2e_tree.rs               # Layer 3: E2E for tree command
├── cli_e2e_update.rs             # Layer 3: E2E for update command
├── cli_e2e_validate.rs           # Layer 3: E2E for validate command
├── cli_e2e_yaml_merge.rs         # Layer 3: E2E for YAML file merging
├── integration_test.rs           # Existing library integration tests
├── schema_parsing_test.rs        # Existing datatest tests
└── testdata/                     # Test fixtures
    ├── configs/
    │   ├── valid-minimal.yaml
    │   ├── valid-complex.yaml
    │   ├── invalid-syntax.yaml
    │   └── invalid-circular.yaml
    └── repos/
        └── [mock repo structures]
```

### Test Fixtures

Create reusable test configurations:

```yaml
# tests/testdata/configs/valid-minimal.yaml
- include:
    patterns: ["README.md"]
```

```yaml
# tests/testdata/configs/valid-inheritance.yaml
- repo:
    url: https://github.com/example/base
    ref: v1.0.0
- include:
    patterns: ["**/*"]
```

```yaml
# tests/testdata/configs/invalid-circular.yaml
- repo:
    url: https://github.com/example/circular-a
    ref: main
# This repo references circular-b which references circular-a
```

---

## Testing Each Command

### `common-repo apply`

**Unit Tests** (`src/commands/apply.rs`):
- Argument validation
- Config file discovery
- Dry-run logic
- Force flag behavior
- Progress output formatting

**Integration Tests**:
- Apply with real git repo (feature-gated)
- Cache creation and reuse
- File overwriting with --force
- Dry-run doesn't write files

**E2E Tests**:
- CLI invocation with valid config
- Missing config file error
- Invalid YAML error
- Correct exit codes
- Progress output validation
- `--dry-run` output

---

### `common-repo validate`

**Unit Tests**:
- Config parsing validation
- Regex pattern validation
- Glob pattern validation

**Integration Tests**:
- Circular dependency detection
- Invalid URL detection
- Invalid ref detection

**E2E Tests**:
- Valid config passes
- Invalid YAML fails with code 2
- Circular dependency detected
- `--json` output format
- `--strict` mode

---

### `common-repo init`

**Unit Tests**:
- Template generation
- File overwrite protection
- Force flag logic

**Integration Tests**:
- Creates valid config file
- Templates are parseable

**E2E Tests**:
- Creates `.common-repo.yaml`
- Refuses to overwrite without `--force`
- `--empty` creates empty file
- `--minimal` creates minimal config

---

### `common-repo cache list`

**Unit Tests**:
- Cache directory parsing
- Size calculation
- Timestamp formatting

**Integration Tests**:
- Lists cached repos after apply
- Empty cache message

**E2E Tests**:
- Lists repos with sizes
- Empty cache output
- `--json` format

---

### `common-repo cache clean`

**Unit Tests**:
- Age filtering logic
- Unused detection logic

**Integration Tests**:
- Deletes old caches
- Preserves recent caches
- Dry-run doesn't delete

**E2E Tests**:
- `--dry-run` shows what would be deleted
- `--all` requires confirmation
- `--yes` skips confirmation
- `--older-than` filters correctly

---

## Snapshot Testing for Output

For commands with complex output, use snapshot testing:

### Required Crate
```toml
[dev-dependencies]
insta = "1.34"
```

### Usage

```rust
use insta::assert_snapshot;

#[test]
fn test_tree_output_format() {
    let output = generate_tree_output(test_repo_tree());

    // First run creates snapshot in snapshots/ directory
    // Subsequent runs compare against snapshot
    assert_snapshot!(output);
}

#[test]
fn test_info_output_format() {
    let output = generate_info_output(test_repo_info());
    assert_snapshot!(output);
}
```

**What to snapshot**:
- `common-repo tree` - Tree visualization
- `common-repo info` - Repository information display
- `common-repo ls` - File listing output
- `common-repo diff --stat` - Diff summary
- `common-repo check` - Update check output

---

## Mocking and Test Doubles

### Network Mocking (for E2E tests)

For tests that need to simulate network without actual network calls:

```rust
// Use wiremock for HTTP mocking
[dev-dependencies]
wiremock = "0.6"

#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_check_command_with_mock_server() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/example/repo/tags"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([
                    {"name": "v1.0.0"},
                    {"name": "v2.0.0"}
                ])))
            .mount(&mock_server)
            .await;

        // Test check command with mock server URL
        // ...
    }
}
```

### Git Mocking

For unit tests that need git operations:

```rust
// Mock git operations in tests
struct MockGit {
    should_fail: bool,
    tags: Vec<String>,
}

impl GitOperations for MockGit {
    fn clone_shallow(&self, _url: &str, _ref: &str, _dir: &Path) -> Result<()> {
        if self.should_fail {
            Err(Error::GitClone { /* ... */ })
        } else {
            Ok(())
        }
    }

    fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
        Ok(self.tags.clone())
    }
}
```

---

## Performance Testing

### Benchmark Command Execution Time

```rust
// benches/cli_benchmarks.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::process::Command;

fn bench_apply_cached(c: &mut Criterion) {
    // Setup: Pre-populate cache
    setup_cached_repo();

    c.bench_function("apply with cache", |b| {
        b.iter(|| {
            Command::new("target/release/common-repo")
                .arg("apply")
                .arg("--config")
                .arg(black_box("test-config.yaml"))
                .output()
                .expect("failed to execute");
        });
    });
}

criterion_group!(benches, bench_apply_cached);
criterion_main!(benches);
```

**Add to Cargo.toml**:
```toml
[[bench]]
name = "cli_benchmarks"
harness = false

[dev-dependencies]
criterion = "0.5"
```

### Performance Targets

Document expected performance:

```rust
#[test]
fn test_apply_performance_with_cache() {
    let start = Instant::now();

    // Run apply with cached repo
    let result = run_apply_with_cache();

    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration < Duration::from_millis(100),
           "Cached apply should complete in <100ms, took {:?}", duration);
}
```

---

## Continuous Integration Testing

### GitHub Actions Workflow

```yaml
# .github/workflows/cli-tests.yml

name: CLI Tests

on: [push, pull_request]

jobs:
  cli-unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run CLI unit tests
        run: cargo test --lib --bins

  cli-integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run CLI integration tests
        run: cargo test --features integration-tests
        env:
          SKIP_NETWORK_TESTS: "0"  # Allow network in CI

  cli-e2e-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build binary
        run: cargo build --release
      - name: Run E2E tests
        run: cargo test --test cli_e2e_test

  cli-e2e-network-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run E2E tests with network
        run: cargo test --test cli_e2e_test --features integration-tests
        env:
          SKIP_NETWORK_TESTS: "0"
```

---

## Test Coverage Goals

### Coverage Targets

- **All modules**: 90%+ coverage
- **Error handling**: Full coverage of error paths
- **Integration paths**: All major workflows covered

### Running Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage including CLI tests
cargo tarpaulin --all-features --out Html --output-dir coverage

# View report
open coverage/index.html
```

---

## Test Maintenance

### Test Naming Convention

```rust
// Unit tests
#[test] fn test_<command>_<scenario>()
#[test] fn test_apply_valid_config()
#[test] fn test_apply_missing_config()

// Integration tests
#[test] fn test_<command>_<workflow>()
#[test] fn test_apply_with_real_repo()
#[test] fn test_cache_after_apply()

// E2E tests
#[test] fn test_cli_<command>_<scenario>()
#[test] fn test_cli_apply_success()
#[test] fn test_cli_apply_missing_config()
```

### Documentation

Each test should have a doc comment explaining:
- What it tests
- Why it's important
- Any setup requirements

```rust
/// Test that apply command correctly handles missing config files.
/// This is critical for UX - users should get clear error messages.
#[test]
fn test_apply_missing_config() {
    // ...
}
```

---

## Implementation Checklist

### Phase 1: Basic CLI Tests (with MVP commands)

- [x] Set up test infrastructure
  - [x] Add `assert_cmd`, `predicates`, `assert_fs` to dev-dependencies
  - [x] Create E2E test files in `tests/`
  - [x] Create `tests/testdata/` directory
- [x] Test `apply` command
  - [x] Unit tests in `src/commands/apply.rs`
  - [x] E2E tests in `tests/cli_e2e_apply.rs`
- [x] Test `validate` command
  - [x] E2E tests in `tests/cli_e2e_validate.rs`
- [x] Test `init` command
  - [x] E2E tests (via apply tests)
- [x] Test `cache` commands
  - [x] E2E tests in `tests/cli_e2e_cache.rs`

### Phase 2: Enhanced Testing (with Phase 2 commands)

- [x] Test `tree` command - `tests/cli_e2e_tree.rs`
- [x] Test `info` command - `tests/cli_e2e_info.rs`
- [x] Test `ls` command - `tests/cli_e2e_ls.rs`
- [x] Test `check` command - `tests/cli_e2e_check.rs`

### Phase 3: Full Testing (with Phase 3 commands)

- [x] Test `diff` command - `tests/cli_e2e_diff.rs`
- [x] Test `update` command - `tests/cli_e2e_update.rs`
- [x] Test file merge operations - `tests/cli_e2e_*_merge.rs` (JSON, YAML, TOML, INI, Markdown)
- [ ] Test `init --interactive` (input simulation)
- [ ] Add performance benchmarks
- [ ] Add network mocking for isolated E2E tests
- [ ] Cross-platform testing in CI

---

## Example: Complete Test Suite for One Command

Here's what a complete test suite looks like for `apply`:

```rust
// src/commands/apply.rs - Unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apply_args() { /* ... */ }

    #[test]
    fn test_validate_config_path() { /* ... */ }

    #[test]
    fn test_dry_run_logic() { /* ... */ }

    #[test]
    fn test_force_flag() { /* ... */ }

    #[test]
    fn test_progress_output() { /* ... */ }
}

// tests/cli_integration_test.rs - Integration test

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_with_real_repo() { /* ... */ }

#[test]
fn test_apply_with_cache() { /* ... */ }

// tests/cli_e2e_apply.rs - E2E tests

#[test]
fn test_cli_apply_help() { /* ... */ }

#[test]
fn test_cli_apply_success() { /* ... */ }

#[test]
fn test_cli_apply_missing_config() { /* ... */ }

#[test]
fn test_cli_apply_invalid_yaml() { /* ... */ }

#[test]
fn test_cli_apply_dry_run() { /* ... */ }

#[test]
fn test_cli_apply_force() { /* ... */ }

#[test]
fn test_cli_apply_verbose() { /* ... */ }

#[test]
fn test_cli_apply_no_cache() { /* ... */ }

#[test]
fn test_cli_apply_exit_codes() { /* ... */ }
```

**Total for one command**: ~15-20 tests across all layers

---

## Summary

### Test Counts

| Layer | Type | Estimated Tests |
|-------|------|-----------------|
| Layer 1 | Command unit tests | ~5-10 per command |
| Layer 2 | Integration tests | ~2-5 per command |
| Layer 3 | E2E tests | ~8-15 per command |
| **Total per command** | | **15-30 tests** |

For 12 commands in full implementation: **180-360 CLI tests**

Plus existing library tests = full test coverage

### Key Testing Principles

1. **Test at multiple layers** - Unit, integration, E2E
2. **Use existing patterns** - Feature gates, tempfile, SKIP_NETWORK_TESTS
3. **Mock external dependencies** - Git, network, filesystem
4. **Validate exit codes** - Critical for shell scripts
5. **Test error messages** - User experience matters
6. **Snapshot complex output** - Tree, info, diff commands
7. **Cross-platform testing** - CI matrix for Linux/Mac/Windows
8. **Performance testing** - Benchmark critical paths

### Priority Order

1. **Phase 1 commands** - Basic E2E tests for MVP
2. **Exit code coverage** - All error cases
3. **Integration tests** - Real workflows
4. **Snapshot tests** - Complex output
5. **Performance tests** - Benchmark suite
6. **Cross-platform** - Windows-specific tests

---

**Last updated**: 2025-12-03
**Status**: Phase 1, Phase 2, and Phase 3 core testing complete; remaining items are enhancements (benchmarks, interactive testing, network mocking, cross-platform CI)
