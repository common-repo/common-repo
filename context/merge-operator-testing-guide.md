# Merge Operator Integration Testing Guide

## Context

This document describes the comprehensive testdata fixtures created for Phase 5 merge operators (YAML, JSON, TOML, INI, Markdown). These fixtures are committed and will be tagged with releases, enabling integration tests to reference them via git refs.

## Created Testdata Fixtures

### Directory Structure

```
tests/testdata/
├── merge-yaml-repo/       (9 files)
├── merge-json-repo/       (10 files)
├── merge-toml-repo/       (9 files)
├── merge-ini-repo/        (9 files)
└── merge-markdown-repo/   (8 files)
```

Total: **45 test fixture files** across **5 merge operator types**

---

## 1. YAML Merge Fixtures (`tests/testdata/merge-yaml-repo/`)

### Files Created

**Configuration:**
- `.common-repo.yaml` - Defines 4 YAML merge test scenarios

**Test Scenario 1: Basic Root-Level Merge**
- `fragment-basic.yml` - Fragment with version "2.0", added_field, nested object
- `destination-basic.yml` - Destination with version "1.0", existing_field, nested object
- **Expected behavior:** Root-level merge, fragment values override destination values

**Test Scenario 2: Nested Path Merge**
- `fragment-nested.yml` - Labels to merge (app, environment, team)
- `destination-nested.yml` - Kubernetes-style service definition with existing labels
- **Expected behavior:** Merge fragment into `metadata.labels` path, preserving other metadata

**Test Scenario 3: List Append**
- `fragment-list.yml` - Array with 2 new items
- `destination-list.yml` - Existing items array plus config section
- **Expected behavior:** Append fragment items to `items` array, preserving existing items

**Test Scenario 4: Section Replace**
- `fragment-replace.yml` - New config values (enabled: false, timeout: 30, retries: 3)
- `destination-replace.yml` - Existing config section with different values
- **Expected behavior:** Replace entire `config` section with fragment content

### Integration Test Usage

```rust
// Example integration test structure
#[test]
fn test_yaml_basic_merge() {
    let temp = TempDir::new().unwrap();

    // Clone or reference testdata repo at tagged version
    let repo_url = "https://github.com/common-repo/common-repo.git";
    let tag = "v0.8.0"; // Use actual release tag

    // Create local .common-repo.yaml that inherits from merge-yaml-repo
    let config = format!(r#"
- repo:
    url: {}
    ref: {}
    path: tests/testdata/merge-yaml-repo
"#, repo_url, tag);

    // Run common-repo apply
    // Assert merged file matches expected output
    // Verify:
    // - destination-basic.yml has version "2.0"
    // - added_field is present
    // - nested.from_fragment is true
    // - nested.priority is "high" (overridden)
}
```

**Test Coverage Goals:**
- Basic root-level merge
- Nested path merge with dot notation
- List append operations
- Section replacement (non-append)
- Conflict resolution (last-write-wins)

---

## 2. JSON Merge Fixtures (`tests/testdata/merge-json-repo/`)

### Files Created

**Configuration:**
- `.common-repo.yaml` - Defines 5 JSON merge test scenarios

**Test Scenario 1: Basic Root-Level Merge**
- `fragment-basic.json` - Fragment with version "2.0.0", added_field, nested object
- `destination-basic.json` - Destination with version "1.0.0", existing_field, nested object
- **Expected behavior:** Root-level merge, fragment values override destination

**Test Scenario 2: Package.json Dependencies**
- `fragment-deps.json` - Additional npm dependencies (lodash, axios, express)
- `package.json` - Existing package.json with react dependencies
- **Expected behavior:** Merge fragment into `dependencies` object, preserving existing deps

**Test Scenario 3: Array Append at Start**
- `fragment-scripts-start.json` - Pre-task script object
- `destination-scripts.json` - Array with main-task script
- **Expected behavior:** Insert fragment at start of `scripts` array (position: start)

**Test Scenario 4: Array Append at End**
- `fragment-scripts-end.json` - Post-task script object
- `destination-scripts.json` - Same destination as Test 3
- **Expected behavior:** Insert fragment at end of `scripts` array (position: end)

**Test Scenario 5: Nested Object Replace**
- `fragment-config.json` - New database config (postgres.example.com, ssl: true, pool_size: 20)
- `destination-config.json` - Existing config with database and cache sections
- **Expected behavior:** Replace `config.database` with fragment, preserve `config.cache`

### Integration Test Usage

```rust
#[test]
fn test_json_dependency_merge() {
    // Similar structure to YAML test
    // Verify package.json has both original and merged dependencies
    // Assert dependencies object contains:
    // - react: "^18.2.0" (original)
    // - lodash: "^4.17.21" (merged)
    // - axios: "^1.6.0" (merged)
    // - express: "^4.18.2" (merged)
}

#[test]
fn test_json_array_positioning() {
    // Test both position: start and position: end
    // Verify array order matches expected positioning
    // Assert scripts[0] is pre-task (start)
    // Assert scripts[1] is main-task (original)
    // Assert scripts[2] is post-task (end)
}
```

**Test Coverage Goals:**
- Basic object merge
- Package.json dependency merging (real-world use case)
- Array positioning (start)
- Array positioning (end)
- Nested object replacement
- Preserving adjacent objects during path merge

---

## 3. TOML Merge Fixtures (`tests/testdata/merge-toml-repo/`)

### Files Created

**Configuration:**
- `.common-repo.yaml` - Defines 4 TOML merge test scenarios

**Test Scenario 1: Basic Root-Level Merge**
- `fragment-basic.toml` - Fragment with version "2.0.0", added_field, nested section
- `destination-basic.toml` - Destination with version "1.0.0", existing_field, nested section
- **Expected behavior:** Root-level merge, fragment values override destination

**Test Scenario 2: Cargo.toml Dependencies**
- `fragment-deps.toml` - Additional Rust dependencies (serde, tokio, anyhow)
- `Cargo.toml` - Existing Cargo.toml with clap and regex dependencies
- **Expected behavior:** Merge into `dependencies` table, preserve existing deps

**Test Scenario 3: Workspace Members Array**
- `fragment-members.toml` - Array with new workspace members
- `workspace.toml` - Existing workspace with 2 members
- **Expected behavior:** Append to `workspace.members` array

**Test Scenario 4: Comment Preservation**
- `fragment-with-comments.toml` - New package metadata (description, license, repository)
- `destination-with-comments.toml` - Existing package with comments throughout
- **Expected behavior:** Merge into `package` section, preserve comments (if preserve-comments: true)

### Integration Test Usage

```rust
#[test]
fn test_toml_cargo_deps_merge() {
    // Verify Cargo.toml dependency merging
    // Assert merged Cargo.toml contains:
    // - clap = { version = "4.4", features = ["derive"] } (original)
    // - serde = { version = "1.0", features = ["derive"] } (merged)
    // - tokio = { version = "1.35", features = ["full"] } (merged)
    // - anyhow = "1.0" (merged)
}

#[test]
fn test_toml_comment_preservation() {
    // If preserve-comments: true is implemented
    // Verify comments are preserved in merged output
    // Parse merged TOML and check for comment markers
}
```

**Test Coverage Goals:**
- Basic table merge
- Cargo.toml dependency merging (real-world Rust use case)
- Array append to workspace members
- Comment preservation (if feature implemented)
- Complex dependency syntax (version + features)

---

## 4. INI Merge Fixtures (`tests/testdata/merge-ini-repo/`)

### Files Created

**Configuration:**
- `.common-repo.yaml` - Defines 4 INI merge test scenarios

**Test Scenario 1: Basic Root-Level Merge**
- `fragment-basic.ini` - Fragment with [general] and [features] sections
- `destination-basic.ini` - Destination with same sections, different values
- **Expected behavior:** Merge sections, fragment values override destination

**Test Scenario 2: Specific Section Merge**
- `fragment-database.ini` - Database config without section header (key=value pairs)
- `config.ini` - Multi-section config with [app], [database], [cache] sections
- **Expected behavior:** Merge fragment into `database` section only

**Test Scenario 3: Section Append with Duplicates**
- `fragment-logging.ini` - Additional logging configuration
- `app.ini` - Existing app with [logging] section
- **Expected behavior:** Append to `logging` section with allow-duplicates: true

**Test Scenario 4: Section Append Without Duplicates**
- `fragment-server.ini` - Server config (timeout, max_connections, keepalive)
- `server.ini` - Existing [server] section with timeout already defined
- **Expected behavior:** Merge into `server` section, timeout value replaced (not duplicated)

### Integration Test Usage

```rust
#[test]
fn test_ini_section_targeting() {
    // Verify fragment merges only into specified section
    // Assert [database] section has merged values
    // Assert [app] and [cache] sections unchanged
    // Verify:
    // - database.host = "postgres.example.com"
    // - database.ssl_mode = "require"
    // - app.name still "My Application"
}

#[test]
fn test_ini_duplicate_handling() {
    // Test allow-duplicates: true
    // Verify both original and fragment values present when allowed
    // Test allow-duplicates: false (default)
    // Verify only latest value present when duplicates not allowed
}
```

**Test Coverage Goals:**
- Multi-section merge
- Section-specific targeting
- Duplicate key handling (allow vs disallow)
- Append mode for sections
- Configuration file use cases (database, server, logging)

---

## 5. Markdown Merge Fixtures (`tests/testdata/merge-markdown-repo/`)

### Files Created

**Configuration:**
- `.common-repo.yaml` - Defines 5 Markdown merge test scenarios

**Test Scenario 1: Section Append at End**
- `fragment-features.md` - New feature sections (Enhanced Security, Performance Improvements)
- `README.md` - Existing README with ## Features section
- **Expected behavior:** Append fragment content to end of Features section

**Test Scenario 2: Section Append at Start**
- `fragment-prerequisites.md` - Prerequisites subsection
- `README.md` - Same README with ## Installation section
- **Expected behavior:** Insert fragment at start of Installation section

**Test Scenario 3: Create New Section**
- `fragment-contributing.md` - Contributing guidelines
- `README.md` - README without Contributing section
- **Expected behavior:** Create new ## Contributing section (level: 2, create-section: true)

**Test Scenario 4: Position Before Another Section**
- `fragment-quickstart.md` - Quick Start guide
- `GUIDE.md` - User guide with ## Getting Started and ## Advanced Usage
- **Expected behavior:** Insert Quick Start section before Advanced Usage section

**Test Scenario 5: Section Replace**
- `fragment-license-updated.md` - Dual license text
- `README.md` - README with existing ## License section
- **Expected behavior:** Replace License section content (append: false)

### Integration Test Usage

```rust
#[test]
fn test_markdown_section_append() {
    // Verify content appended to Features section
    // Parse markdown to check section hierarchy
    // Assert Features section contains:
    // - ### Basic Features (original)
    // - ### Enhanced Security (appended)
    // - ### Performance Improvements (appended)
}

#[test]
fn test_markdown_section_creation() {
    // Verify new Contributing section created
    // Assert section exists at correct heading level
    // Verify create-section: true creates missing section
}

#[test]
fn test_markdown_positioning() {
    // Test position: before with reference-section
    // Parse GUIDE.md structure
    // Assert Quick Start appears before Advanced Usage
    // Verify section ordering is correct
}
```

**Test Coverage Goals:**
- Section append (start and end positioning)
- Section creation when missing
- Relative positioning (before/after sections)
- Section replacement (non-append mode)
- Heading level handling
- README.md and documentation use cases

---

## Implementation Roadmap

### Phase 1: Tag and Release

1. Merge this branch to main
2. CI creates release tag (e.g., `v0.8.0`)
3. Fixtures become available via git ref

### Phase 2: Implement Merge Operators (Future Branches)

For each merge operator, implement in this order:

#### 2.1 YAML Merge Operator
```rust
// src/operators/merge/yaml.rs
pub fn apply(
    fs: &mut MemoryFS,
    source: &Path,
    dest: &Path,
    path: Option<&str>,
    append: bool,
) -> Result<()>
```

**Integration test reference:**
```rust
// tests/integration_yaml_merge.rs
const FIXTURES_TAG: &str = "v0.8.0"; // Update to actual release tag
const FIXTURES_REPO: &str = "https://github.com/common-repo/common-repo.git";
const FIXTURES_PATH: &str = "tests/testdata/merge-yaml-repo";
```

#### 2.2 JSON Merge Operator
```rust
// src/operators/merge/json.rs
pub fn apply(
    fs: &mut MemoryFS,
    source: &Path,
    dest: &Path,
    path: Option<&str>,
    append: bool,
    position: Option<Position>,
) -> Result<()>
```

**Integration test reference:** Same pattern, use `tests/testdata/merge-json-repo`

#### 2.3 TOML Merge Operator
```rust
// src/operators/merge/toml.rs
pub fn apply(
    fs: &mut MemoryFS,
    source: &Path,
    dest: &Path,
    path: Option<&str>,
    append: bool,
    preserve_comments: bool,
) -> Result<()>
```

**Integration test reference:** Use `tests/testdata/merge-toml-repo`

#### 2.4 INI Merge Operator
```rust
// src/operators/merge/ini.rs
pub fn apply(
    fs: &mut MemoryFS,
    source: &Path,
    dest: &Path,
    section: Option<&str>,
    append: bool,
    allow_duplicates: bool,
) -> Result<()>
```

**Integration test reference:** Use `tests/testdata/merge-ini-repo`

#### 2.5 Markdown Merge Operator
```rust
// src/operators/merge/markdown.rs
pub fn apply(
    fs: &mut MemoryFS,
    source: &Path,
    dest: &Path,
    section: &str,
    append: bool,
    level: Option<u8>,
    position: Option<Position>,
    reference_section: Option<&str>,
    create_section: bool,
) -> Result<()>
```

**Integration test reference:** Use `tests/testdata/merge-markdown-repo`

### Phase 3: Integration Test Pattern

Each merge operator should have integration tests following this pattern:

```rust
// tests/integration_<format>_merge.rs
use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use common_repo::repository::RepositoryManager;

const FIXTURES_TAG: &str = "v0.8.0"; // Update to actual release
const FIXTURES_REPO: &str = "https://github.com/common-repo/common-repo.git";

#[test]
#[cfg(feature = "integration-tests")]
fn test_<format>_basic_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Create config that references fixture repo
    config_file.write_str(&format!(r#"
- repo:
    url: {}
    ref: {}
    path: tests/testdata/merge-<format>-repo
"#, FIXTURES_REPO, FIXTURES_TAG)).unwrap();

    // Run common-repo apply
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    // Verify merged output
    let merged_file = temp.child("<destination-file>");
    assert!(merged_file.exists());

    let content = std::fs::read_to_string(merged_file.path()).unwrap();
    // Add specific assertions for expected merged content
    assert!(content.contains("<expected-merged-value>"));
}
```

### Phase 4: Test Coverage Targets

For each merge operator implementation:

- **Unit tests**: Test merge logic in isolation (mock MemoryFS)
- **Integration tests**: Test with actual fixture repos via git refs
- **Error handling**: Invalid paths, missing sections, parse errors
- **Edge cases**: Empty files, conflicting keys, malformed input
- **Performance**: Large file handling, deep nesting

Target coverage: **90%+** for merge operator modules

---

## Quick Reference: Test Scenarios by Format

### YAML (4 scenarios)
1. Basic root merge
2. Nested path (`metadata.labels`)
3. List append
4. Section replace

### JSON (5 scenarios)
1. Basic root merge
2. Dependencies merge (`dependencies`)
3. Array insert start (`position: start`)
4. Array insert end (`position: end`)
5. Nested object replace (`config.database`)

### TOML (4 scenarios)
1. Basic root merge
2. Cargo dependencies (`dependencies`)
3. Workspace members append (`workspace.members`)
4. Comment preservation (`preserve-comments: true`)

### INI (4 scenarios)
1. Basic section merge
2. Section targeting (`section: database`)
3. Duplicates allowed (`allow-duplicates: true`)
4. Duplicates disallowed (`allow-duplicates: false`)

### Markdown (5 scenarios)
1. Section append end (`position: end`)
2. Section append start (`position: start`)
3. Section creation (`create-section: true`)
4. Position before section (`position: before`)
5. Section replace (`append: false`)

---

## Notes for Future Implementation

### Dependencies Required

Add these to `Cargo.toml` when implementing merge operators:

```toml
[dependencies]
# Already present for config parsing
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1.0"

# Add for merge operators
toml = "0.8"  # TOML parsing and merging
ini = "1.3"   # INI file handling
pulldown-cmark = "0.9"  # Markdown parsing
pulldown-cmark-to-cmark = "10.0"  # Markdown rendering
```

### Design Considerations

1. **Path parsing**: Implement dot-notation path parser for YAML/JSON/TOML
   - Handle nested objects: `metadata.labels`
   - Handle array indices: `items[0]`
   - Handle complex paths: `config.database.ssl.enabled`

2. **Conflict resolution**: Currently last-write-wins, consider:
   - Warning on overwrites
   - Optional strict mode (fail on conflicts)
   - Merge strategies (deep merge vs shallow merge)

3. **Array handling**: Implement positioning logic
   - `start`: Insert at beginning
   - `end`: Append at end
   - `before`/`after`: Relative positioning
   - Index-based: `position: 2`

4. **Comment preservation**: TOML-specific feature
   - Use TOML library with comment support
   - Preserve comments during merge
   - Optional: strip comments for clean output

5. **Section creation**: Markdown-specific feature
   - Parse markdown AST
   - Find or create sections
   - Maintain heading hierarchy
   - Handle section references

### Testing Strategy

1. **Start with YAML**: Simplest format, good for initial implementation
2. **Then JSON**: Similar to YAML, adds positioning complexity
3. **Then TOML**: Adds comment preservation, Cargo.toml use case
4. **Then INI**: Different structure (sections), duplicate handling
5. **Then Markdown**: Most complex, section-based, positioning

Each operator builds on lessons from previous implementations.

---

## Success Criteria

Before marking merge operators complete:

- [ ] All 5 merge operators implemented
- [ ] All fixture scenarios have passing integration tests
- [ ] Unit test coverage ≥90% for merge modules
- [ ] Integration tests use tagged fixture refs
- [ ] Error handling covers malformed input
- [ ] Performance acceptable for large files (>1MB)
- [ ] Documentation updated with merge operator usage
- [ ] Examples added to README or docs/

---

## Example: Using Fixtures in Tests

After release tag `v0.8.0` is created:

```rust
// tests/integration_yaml_merge.rs

#[test]
#[cfg(feature = "integration-tests")]
fn test_yaml_nested_path_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config = temp.child(".common-repo.yaml");

    config.write_str(r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.8.0
    path: tests/testdata/merge-yaml-repo
"#).unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("apply")
        .assert()
        .success();

    // Verify destination-nested.yml was merged correctly
    let merged = temp.child("destination-nested.yml");
    let content = std::fs::read_to_string(merged.path()).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    // Check merged labels
    assert_eq!(
        parsed["metadata"]["labels"]["app"].as_str().unwrap(),
        "my-application"
    );
    assert_eq!(
        parsed["metadata"]["labels"]["environment"].as_str().unwrap(),
        "production"
    );
    assert_eq!(
        parsed["metadata"]["labels"]["team"].as_str().unwrap(),
        "platform"
    );

    // Check original labels preserved
    assert_eq!(
        parsed["metadata"]["labels"]["version"].as_str().unwrap(),
        "1.0"
    );
}
```

This guide provides complete context for continuing merge operator implementation work in future branches.
