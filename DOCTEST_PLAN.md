# Doctest Implementation Plan

This document outlines a comprehensive plan for adding doctests to the common-repo codebase. Doctests serve dual purposes: they provide executable examples in the API documentation and act as additional tests.

## Overview

**Current Status:** 1 doctest implemented (`path::encode_url_path`)
**Target:** 20+ doctests across the codebase
**Priority:** High priority candidates first, then medium, then low

## Selection Criteria

Good doctest candidates are:
1. **Public functions** - doctests appear in public API documentation
2. **Pure functions** - no side effects, deterministic outputs
3. **Utility functions** - commonly used transformations
4. **Clear inputs/outputs** - easy to demonstrate with simple examples
5. **No complex setup** - avoid functions requiring filesystem, network, or mocking

---

## High Priority Candidates (Easy wins, high value)

### 1. `src/path.rs` - Path utilities

#### `glob_match` (Line 9)
**Why:** Pure function, simple string matching, very clear behavior
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::path::glob_match;
///
/// assert!(glob_match("*.rs", "main.rs").unwrap());
/// assert!(glob_match("src/*.rs", "src/main.rs").unwrap());
/// assert!(!glob_match("*.rs", "main.js").unwrap());
/// ```
```

#### `regex_rename` (Line 22)
**Why:** Pure function, demonstrates regex capture groups
**Complexity:** Medium
**Example ideas:**
```rust
/// ```
/// use common_repo::path::regex_rename;
///
/// // Simple replacement
/// assert_eq!(
///     regex_rename(r"(\w+)\.rs", "$1_backup.rs", "main.rs").unwrap(),
///     Some("main_backup.rs".to_string())
/// );
///
/// // Multiple capture groups
/// assert_eq!(
///     regex_rename(r"(\w+)/(\w+)\.rs", "$2_$1.rs", "src/main.rs").unwrap(),
///     Some("main_src.rs".to_string())
/// );
/// ```
```

### 2. `src/git.rs` - Git utilities

#### `parse_semver_tag` (Line 197)
**Why:** Pure function, shows semver parsing patterns
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::git::parse_semver_tag;
/// use semver::Version;
///
/// // With 'v' prefix
/// assert_eq!(
///     parse_semver_tag("v1.0.0"),
///     Some(Version::parse("1.0.0").unwrap())
/// );
///
/// // Without prefix
/// assert_eq!(
///     parse_semver_tag("2.1.3"),
///     Some(Version::parse("2.1.3").unwrap())
/// );
///
/// // Pre-release versions
/// assert_eq!(
///     parse_semver_tag("v1.0.0-alpha"),
///     Some(Version::parse("1.0.0-alpha").unwrap())
/// );
///
/// // Invalid versions
/// assert_eq!(parse_semver_tag("not-a-version"), None);
/// ```
```

### 3. `src/cache.rs` - Cache utilities

#### `CacheKey::new` (Line 20)
**Why:** Simple constructor, demonstrates usage pattern
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::cache::CacheKey;
///
/// let key = CacheKey::new("https://github.com/user/repo.git", "main");
/// assert_eq!(key.url, "https://github.com/user/repo.git");
/// assert_eq!(key.r#ref, "main");
///
/// // Keys with same values are equal
/// let key1 = CacheKey::new("https://example.com/repo", "v1.0.0");
/// let key2 = CacheKey::new("https://example.com/repo", "v1.0.0");
/// assert_eq!(key1, key2);
/// ```
```

### 4. `src/filesystem.rs` - Filesystem utilities

#### `File::new` (Line 24)
**Why:** Constructor, shows basic usage
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::File;
///
/// let content = vec![72, 101, 108, 108, 111]; // "Hello"
/// let file = File::new(content);
///
/// assert_eq!(file.size(), 5);
/// assert_eq!(file.permissions, 0o644);
/// assert_eq!(file.content, vec![72, 101, 108, 108, 111]);
/// ```
```

#### `File::from_string` (Line 33)
**Why:** Convenience constructor, very common use case
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::File;
///
/// let file = File::from_string("Hello, world!");
///
/// assert_eq!(file.content, b"Hello, world!");
/// assert_eq!(file.size(), 13);
/// ```
```

#### `File::size` (Line 38)
**Why:** Simple getter, shows file metadata
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::File;
///
/// let empty = File::new(vec![]);
/// assert_eq!(empty.size(), 0);
///
/// let file = File::from_string("content");
/// assert_eq!(file.size(), 7);
/// ```
```

---

## Medium Priority Candidates (Good value, slightly more complex)

### 5. `src/filesystem.rs` - MemoryFS operations

#### `MemoryFS::new` (Line 54)
**Why:** Primary API entry point
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::MemoryFS;
///
/// let fs = MemoryFS::new();
/// assert!(fs.is_empty());
/// assert_eq!(fs.len(), 0);
/// ```
```

#### `MemoryFS::add_file_string` (Line 71)
**Why:** Most common way to add files
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::MemoryFS;
///
/// let mut fs = MemoryFS::new();
/// fs.add_file_string("README.md", "# My Project").unwrap();
///
/// assert!(fs.exists("README.md"));
/// assert_eq!(fs.len(), 1);
///
/// let file = fs.get_file("README.md").unwrap();
/// assert_eq!(file.content, b"# My Project");
/// ```
```

#### `MemoryFS::exists` (Line 86)
**Why:** Common check operation
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::MemoryFS;
///
/// let mut fs = MemoryFS::new();
///
/// assert!(!fs.exists("file.txt"));
///
/// fs.add_file_string("file.txt", "content").unwrap();
/// assert!(fs.exists("file.txt"));
/// ```
```

#### `MemoryFS::rename_file` (Line 112)
**Why:** Shows file manipulation
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::MemoryFS;
///
/// let mut fs = MemoryFS::new();
/// fs.add_file_string("old_name.txt", "content").unwrap();
///
/// fs.rename_file("old_name.txt", "new_name.txt").unwrap();
///
/// assert!(!fs.exists("old_name.txt"));
/// assert!(fs.exists("new_name.txt"));
/// ```
```

#### `MemoryFS::merge` (Line 157)
**Why:** Shows filesystem composition
**Complexity:** Medium
**Example ideas:**
```rust
/// ```
/// use common_repo::filesystem::MemoryFS;
///
/// let mut fs1 = MemoryFS::new();
/// fs1.add_file_string("file1.txt", "content1").unwrap();
///
/// let mut fs2 = MemoryFS::new();
/// fs2.add_file_string("file2.txt", "content2").unwrap();
///
/// fs1.merge(&fs2);
///
/// assert_eq!(fs1.len(), 2);
/// assert!(fs1.exists("file1.txt"));
/// assert!(fs1.exists("file2.txt"));
/// ```
```

### 6. `src/config.rs` - Configuration parsing

#### `default_header_level` (Line 169)
**Why:** Simple function, shows default behavior
**Complexity:** Low
**Example ideas:**
```rust
/// ```
/// use common_repo::config::default_header_level;
///
/// assert_eq!(default_header_level(), 2);
/// ```
```

---

## Low Priority Candidates (Lower value or higher complexity)

### 7. `src/config.rs` - Schema parsing

#### `parse` (Line 209)
**Why:** Main API entry point but requires understanding YAML schema
**Complexity:** High
**Notes:** Would need substantial YAML examples. Consider after high/medium priorities.

#### `parse_original_format` (Line 222)
**Why:** Legacy format support
**Complexity:** High
**Notes:** Similar to `parse`, lower priority as it's for backward compatibility.

### 8. `src/phases.rs` - Phase implementations

#### `RepoNode::new` (Line 40)
**Why:** Constructor for tree node
**Complexity:** Medium
**Notes:** Requires understanding of the inheritance system. Consider for comprehensive docs.

### 9. `src/operators.rs` - Operator implementations

**Notes:** The `apply` functions in the operator modules are more complex and require MemoryFS setup. While they could benefit from doctests showing usage patterns, they're lower priority because:
- They require more setup code
- They're tested comprehensively in unit tests
- They're less likely to be called directly by users

---

## Implementation Strategy

### Phase 1: Quick Wins (1-2 hours)
Add doctests to high-priority candidates:
1. `path::glob_match`
2. `path::regex_rename`
3. `git::parse_semver_tag`
4. `cache::CacheKey::new`
5. `filesystem::File::new`
6. `filesystem::File::from_string`
7. `filesystem::File::size`

**Expected outcome:** 7 new doctests, all pure functions with clear behavior

### Phase 2: Core API (2-3 hours)
Add doctests to medium-priority MemoryFS functions:
1. `filesystem::MemoryFS::new`
2. `filesystem::MemoryFS::add_file_string`
3. `filesystem::MemoryFS::exists`
4. `filesystem::MemoryFS::rename_file`
5. `filesystem::MemoryFS::merge`
6. `config::default_header_level`

**Expected outcome:** 6 new doctests covering the main user-facing API

### Phase 3: Comprehensive Coverage (3-4 hours)
Add doctests to low-priority candidates as time permits

---

## Testing Doctests

Run doctests with:
```bash
# Run all doctests
cargo test --doc

# Run doctests for a specific module
cargo test --doc module_name

# Run a specific doctest
cargo test --doc function_name
```

All doctests must pass for CI to succeed.

---

## Notes

- **Formatting:** All doctest code must pass `cargo fmt`
- **Linting:** Doctest code is checked by `cargo clippy`
- **Public API:** Only add doctests to public functions (users can't see private doctests)
- **Simplicity:** Keep examples simple and focused on one concept
- **Error Handling:** Use `.unwrap()` in doctests for brevity unless demonstrating error handling
- **Imports:** Always show the full import path in doctests

---

## Progress Tracking

- [x] `path::encode_url_path` (completed)
- [x] `path::glob_match` (completed)
- [x] `path::regex_rename` (completed)
- [x] `git::parse_semver_tag` (completed)
- [x] `cache::CacheKey::new` (completed)
- [x] `filesystem::File::new` (completed)
- [x] `filesystem::File::from_string` (completed)
- [x] `filesystem::File::size` (completed)
- [x] `filesystem::MemoryFS::new` (completed)
- [x] `filesystem::MemoryFS::add_file_string` (completed)
- [x] `filesystem::MemoryFS::exists` (completed)
- [x] `filesystem::MemoryFS::rename_file` (completed)
- [x] `filesystem::MemoryFS::merge` (completed)
- [x] `config::default_header_level` (completed)

**Total:** 14/14 high and medium priority doctests completed (100%)

---

## Future Considerations

1. **Integration examples:** Consider adding module-level doctests (`//!` comments) showing complete workflows
2. **Error handling examples:** Add doctests demonstrating error cases for critical functions
3. **Performance notes:** Document performance characteristics where relevant
4. **Operator examples:** Once core API is documented, add operator usage examples

---

## References

- Rust doctest documentation: https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
- Project CLAUDE.md: Guidelines for testing and code quality
- Existing tests: `src/*/tests` modules provide test patterns to follow
