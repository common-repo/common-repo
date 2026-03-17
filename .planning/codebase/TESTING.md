# Testing Patterns

**Analysis Date:** 2026-03-16

## Test Framework

**Runner:**
- `cargo-nextest` (primary) with fallback to `cargo test`
- Config: `.config/nextest.toml`
- Profiles: `default` (development), `ci` (continuous integration)

**Assertion Library:**
- `assert_cmd` - CLI command execution assertions
- `assert_fs` - Filesystem assertions (TempDir, file existence, content)
- `predicates` - Predicate-based assertions (str::contains, path::exists)
- `insta` - Snapshot testing for CLI output (with YAML serialization)
- Standard `assert!`, `assert_eq!` for basic assertions

**Run Commands:**
```bash
cargo nextest run                          # Run all tests (fastest, default)
cargo nextest run --profile ci             # Run with CI profile (stricter timeouts)
cargo test                                 # Fallback to standard cargo test
cargo nextest run test_name                # Run specific test
cargo test --features integration-tests    # Run integration tests
SKIP_NETWORK_TESTS=1 cargo test            # Skip network tests
cargo xtask coverage                       # Generate HTML coverage report
cargo xtask coverage --open                # Open coverage in browser
```

## Test File Organization

**Location:**
- Unit tests: Inline in source files using `#[cfg(test)]` modules
- Integration/E2E tests: Separate files in `tests/` directory
- Co-located unit tests are NOT used; all tests are in `tests/`
- Test data/fixtures: `tests/testdata/` and `tests/common/` modules

**Naming:**
- E2E CLI tests: `cli_e2e_*.rs` (e.g., `cli_e2e_apply.rs`, `cli_e2e_init.rs`)
- Integration tests: `integration_*.rs` (e.g., `integration_test.rs`, `integration_merge_yaml.rs`)
- Snapshot tests: `cli_snapshot_tests.rs`
- Schema validation: `schema_parsing_test.rs`
- Helper module: `tests/common/mod.rs`

**Structure:**
```
tests/
├── cli_e2e_*.rs              # End-to-end CLI command tests
├── integration_*.rs          # Integration tests with real repos/network
├── cli_snapshot_tests.rs     # Snapshot tests for help/output
├── schema_parsing_test.rs    # Datatest schema validation
├── common/mod.rs             # Shared test utilities & fixtures
└── testdata/                 # Test configuration files
    └── deep-repo-*/          # Test repository structures
```

## Test Structure

**Suite Organization:**
```rust
//! End-to-end tests for the `apply` command
//!
//! These tests invoke the actual CLI binary and validate its behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Apply the .common-repo.yaml configuration"));
}
```

**Patterns:**
- Setup: Create `TempDir`, `cargo_bin_cmd!("common-repo")`, prepare test fixtures
- Execution: `.arg()` to add arguments, `.assert()` to check results
- Assertions: Chain predicates using `assert_fs::prelude::*` and `predicates::prelude::*`
- Teardown: Automatic via `TempDir` drop (no explicit cleanup needed)

**Configuration Examples in `tests/common/mod.rs`:**
```rust
pub mod configs {
    pub const MINIMAL: &str = r#"
- include: ["**/*"]
"#;

    pub const README_ONLY: &str = r#"
- include: ["README.md"]
"#;

    pub const WITH_EXCLUDE: &str = r#"
- include: ["**/*"]
- exclude: ["target/**", ".git/**"]
"#;

    pub const WITH_REPO: &str = r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
"#;
}
```

## Mocking

**Framework:** No external mocking library; real implementations preferred

**Patterns:**
- **In-memory filesystem** (`MemoryFS`) used instead of mocking filesystem
- **Real temporary directories** (`TempDir`, `assert_fs::TempDir`) for integration tests
- **Real Git operations** for repository cloning (with network test guards)
- **Test fixtures and known data** instead of mock objects

**What to Mock:**
- Nothing explicitly mocked in this codebase
- Use real/in-memory implementations instead:
  - `MemoryFS` for filesystem operations (avoids disk I/O in unit tests)
  - `TempDir` for temporary file operations (real directories, cleaned up automatically)

**What NOT to Mock:**
- Git operations (tests use real `git clone`)
- Repository loading (tests load real test repositories)
- File system operations (use `MemoryFS` or `TempDir` instead)

**Network Test Handling:**
- Integration tests guarded with `#[cfg_attr(not(feature = "integration-tests"), ignore)]`
- Environment variable check: `if env::var("SKIP_NETWORK_TESTS").is_ok() { return; }`
- Example from `tests/integration_test.rs`:
  ```rust
  #[test]
  #[cfg_attr(not(feature = "integration-tests"), ignore)]
  fn test_clone_cache_and_load_repository() {
      if env::var("SKIP_NETWORK_TESTS").is_ok() {
          println!("Skipping network integration test");
          return;
      }
      // Real cloning code
  }
  ```

## Fixtures and Factories

**Test Data:**
- Shared configurations in `tests/common/mod.rs::configs`:
  - `MINIMAL` - Single include pattern
  - `README_ONLY` - Include only README
  - `WITH_EXCLUDE` - Include and exclude
  - `INVALID_YAML` - For error testing
  - `WITH_REPO` - Repository reference
  - `EMPTY` - Empty configuration

**Test Fixture Builder:**
```rust
pub struct TestFixture {
    // ... fields
}

impl TestFixture {
    pub fn new() -> Self { ... }
    pub fn with_minimal_config(self) -> Self { ... }
}
```

**Location:**
- `tests/common/mod.rs` - Shared fixtures and configuration constants
- `tests/testdata/` - Static test data files (YAML configs, sample repos)
- Inline in test files for one-off test data

## Coverage

**Requirements:**
- No enforced minimum coverage target in CI
- Coverage reports generated but not blocking
- Tool: `cargo-tarpaulin` (alternative: `llvm-cov`)

**View Coverage:**
```bash
cargo xtask coverage                 # Generate HTML report (default)
cargo xtask coverage --format json   # JSON output
cargo xtask coverage --fail-under 80 # Fail if < 80% coverage
cargo xtask coverage --open          # Open in browser
```

**Configuration:**
- `.tarpaulin.toml` present (564B) - Tarpaulin-specific settings
- Coverage reports written to `target/` (gitignored)

## Test Types

**Unit Tests:**
- Scope: Individual functions and modules
- Approach: Real/in-memory implementations, no mocks
- Location: In `tests/` directory, not inline
- Example: `test_schema_parsing` validates schema parsing against YAML test files

**Integration Tests:**
- Scope: Multiple modules working together, real repositories
- Approach: Use real Git operations, network calls (feature-gated)
- Location: `tests/integration_*.rs` files
- Feature flag: `#[cfg_attr(not(feature = "integration-tests"), ignore)]`
- Network guard: `if env::var("SKIP_NETWORK_TESTS").is_ok() { return; }`
- Example from `tests/integration_test.rs`:
  ```rust
  #[test]
  #[cfg_attr(not(feature = "integration-tests"), ignore)]
  fn test_clone_cache_and_load_repository() {
      // Real cloning from GitHub
      let manager = RepositoryManager::new(cache_dir.clone());
      let fs1 = manager.fetch_repository(repo_url, ref_name)?;
      assert!(fs1.exists("Cargo.toml"));
  }
  ```

**E2E Tests:**
- Scope: Full CLI command execution
- Approach: Use `cargo_bin_cmd!("common-repo")` to invoke the binary
- Location: `tests/cli_e2e_*.rs` files
- Feature flag: `#[cfg_attr(not(feature = "integration-tests"), ignore)]` for network tests
- Example from `tests/cli_e2e_apply.rs`:
  ```rust
  #[test]
  #[cfg_attr(not(feature = "integration-tests"), ignore)]
  fn test_apply_valid_config() {
      let temp = assert_fs::TempDir::new().unwrap();
      let config_file = temp.child(".common-repo.yaml");
      config_file.write_str(r#"- include: ["README.md"]"#).unwrap();

      let mut cmd = cargo_bin_cmd!("common-repo");
      cmd.arg("apply")
          .arg("--config").arg(config_file.path())
          .arg("--dry-run")
          .arg("--quiet")
          .assert()
          .success();
  }
  ```

**Snapshot Tests:**
- Framework: `insta` (with YAML feature enabled)
- Approach: Capture and compare CLI output (help text, error messages)
- Normalization: Version numbers and paths normalized to make snapshots stable
- Location: `tests/cli_snapshot_tests.rs`
- Update command: `cargo insta test --accept`
- Example:
  ```rust
  fn normalize_output(output: &str) -> String {
      let re = regex::Regex::new(r"common-repo \d+\.\d+\.\d+").unwrap();
      let versioned = re.replace_all(output, "common-repo [VERSION]");
      versioned.lines()
          .map(|line| line.trim_end())
          .collect::<Vec<_>>()
          .join("\n")
  }

  #[test]
  fn test_main_help_snapshot() {
      let mut cmd = cargo_bin_cmd!("common-repo");
      let output = cmd.arg("--help").output().expect("Failed to execute");
      let stdout = String::from_utf8_lossy(&output.stdout);
      let normalized = normalize_output(&stdout);
      insta::assert_snapshot!("main_help", normalized);
  }
  ```

**Datatest Tests:**
- Framework: `datatest-stable`
- Approach: Auto-discover test data files and run test function on each
- Location: `tests/schema_parsing_test.rs`
- Data location: `tests/testdata/` - YAML files auto-discovered
- Example:
  ```rust
  fn test_schema_parsing(path: &Path) -> datatest_stable::Result<()> {
      let content = std::fs::read_to_string(path)?;
      let schema: Schema = parse(&content)?;
      assert!(!schema.is_empty());
      // Verify each operation in schema
      for (idx, operation) in schema.iter().enumerate() {
          match operation {
              Operation::Repo { repo } => {
                  assert!(!repo.url.is_empty());
              }
              // ... other variants
          }
      }
      Ok(())
  }
  ```

**Property-Based Tests:**
- Framework: `proptest` for property-based testing
- Used for: Path operations and other properties
- Location: `src/path_proptest.rs`
- Example: Testing that certain properties hold across random inputs
- Slow test handling: Special nextest override with extended timeouts

## Nextest Configuration Details

**From `.config/nextest.toml`:**

**Default Profile:**
- Slow test timeout: 5 seconds (terminate after 6 instances, 30s grace period)
- Fail fast: disabled (run all tests)
- Retries: 0

**CI Profile:**
- Slow test timeout: 3 seconds stricter (terminate after 10 instances)
- Immediate output for all tests (success and failure)
- Retries: 0 (no flaky test retries in CI)

**Integration Test Overrides:**
- Filter: `test(integration_test::) | test(::test_clone) | test(::test_fetch)`
- Default profile: 2 retries
- CI profile: 3 retries
- Rationale: Network-dependent tests may timeout due to GitHub rate limits or DNS

**Property-Based Test Overrides:**
- Filter: `test(proptest)`
- Slow timeout: 10s (default), 15s (CI)
- Grace period: 60s (longer for shrinking)

**JUnit Output:**
- Default: `target/nextest/default/junit.xml`
- CI: `target/nextest/ci/junit.xml`

## Testing Guidelines

**Before Committing:**
1. Run `./script/ci` - validates formatting, linting, and passes all checks
2. Run `./script/test` - executes full test suite
3. For integration tests: `cargo test --features integration-tests`

**Integration Test Requirements:**
- Feature-gated: `#[cfg_attr(not(feature = "integration-tests"), ignore)]`
- Network tests guarded: `if env::var("SKIP_NETWORK_TESTS").is_ok() { return; }`
- Explicit network call documentation
- Rationale: avoid flaky CI from external dependencies

**Test Data Stability:**
- No hardcoded version numbers or timestamps
- Snapshot tests normalize version strings: `common-repo \d+\.\d+\.\d+` → `[VERSION]`
- Use `datatest-stable` for schema validation across changing test data
- Real test repositories stored in `tests/testdata/`

**Failure Handling:**
- Errors include optional `hint` field for user guidance
- Tests verify both error occurrence AND error message content
- Snapshot tests allow easy review of expected output changes

---

*Testing analysis: 2026-03-16*
