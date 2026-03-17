# Coding Conventions

**Analysis Date:** 2026-03-16

## Naming Patterns

**Files:**
- Rust source files use snake_case: `config.rs`, `filesystem.rs`, `phase_orchestrator.rs`
- Command modules follow their command name: `apply.rs`, `check.rs`, `init.rs`, `ls.rs`
- Test files use descriptive names with E2E or integration prefixes:
  - `cli_e2e_apply.rs` - End-to-end CLI tests
  - `integration_test.rs` - Integration tests with real repositories
  - `cli_snapshot_tests.rs` - Snapshot tests for CLI output
  - `schema_parsing_test.rs` - Datatest schema validation tests
- Submodule files use consistent names: `mod.rs` for module declaration files, then specific modules like `yaml.rs`, `json.rs`, `ini.rs`

**Functions:**
- Public functions use snake_case: `fetch_repository()`, `list_files_glob()`, `apply()`, `execute()`
- Private helper functions are also snake_case with `(crate)` visibility: `execute_command()`, `normalize_output()`
- Async functions are marked with `async` keyword (limited async usage in this codebase - mostly `anyhow::Result` based)
- Test functions follow Rust convention: `test_init_with_uri_positional_arg()`, `test_apply_missing_config()`, `test_clone_cache_and_load_repository()`

**Variables:**
- Camel case for most variable names: `tempDir`, `configFile`, `fsSource`, `targetFs`
- Temporary variables and loop iterators: `i`, `idx`, `path`, `pattern`, `entry`
- Boolean flags use `is_`, `has_`, `should_` prefix: `is_template`, `is_cached`, `is_empty()`
- Result-bearing variables: `result`, `error`, `output`

**Types:**
- Struct names use PascalCase: `File`, `MemoryFS`, `RepoCache`, `RepositoryManager`, `Operation`
- Enum names use PascalCase: `Error`, `Commands`, `Operation`
- Type aliases use PascalCase: `Schema` (alias for `Vec<Operation>`), `Result<T>` (alias for `std::result::Result<T, Error>`)
- Trait names would use PascalCase (limited trait usage)

## Code Style

**Formatting:**
- Tool: `rustfmt` (configured in pre-commit hooks)
- Line length: Not explicitly enforced; follows Rust convention (~100 chars comfort)
- Indentation: 4 spaces (standard Rust)
- Trailing commas: Included in multi-line structures
- No semicolons in match arms that return values

**Linting:**
- Tool: `cargo clippy` with strict settings: `--all-targets --all-features -- -D warnings`
- Pre-commit hooks enforce: `fmt` check, `cargo check`, `clippy` with `-D warnings` flag
- Any clippy warnings become hard errors during CI

**Example formatting:**
```rust
pub fn apply(op: &IncludeOp, source: &MemoryFS, target: &mut MemoryFS) -> Result<()> {
    for pattern in &op.patterns {
        let matching_files = source.list_files_glob(pattern)?;

        for path in matching_files {
            if let Some(file) = source.get_file(&path) {
                // Clone the file to add to target
                target.add_file(&path, file.clone())?;
            }
        }
    }

    Ok(())
}
```

## Import Organization

**Order:**
1. Standard library (`use std::`)
2. External crates (e.g., `use serde::`, `use clap::`)
3. Internal crate modules (e.g., `use crate::filesystem::`)
4. Prelude imports for tests (`use common_repo::*;`)

**Path Aliases:**
- No custom path aliases observed
- Full paths used: `std::path::Path`, `std::collections::HashMap`
- Re-exports in test common module: `pub mod prelude { pub use assert_cmd::cargo::cargo_bin_cmd; ... }`

**Example from `src/operators.rs`:**
```rust
use crate::config::{ExcludeOp, IncludeOp, Operation, RenameOp, RepoOp};
use crate::error::Result;
use crate::filesystem::MemoryFS;
use crate::path::regex_rename;
use crate::repository::RepositoryManager;
use std::path::Path;
```

## Error Handling

**Patterns:**
- Centralized error type using `thiserror` crate: `pub enum Error { ... }` in `src/error.rs`
- Custom `Result<T>` alias: `pub type Result<T> = std::result::Result<T, Error>;`
- All error variants are explicit and documented with fields for context
- Errors include optional `hint` fields for user-friendly guidance

**Example error types in `src/error.rs`:**
```rust
#[error("Configuration parsing error: {message}...")]
ConfigParse {
    message: String,
    hint: Option<String>,
},

#[error("Git clone error for {url}@{r#ref}: {message}...")]
GitClone {
    url: String,
    r#ref: String,
    message: String,
    hint: Option<String>,
},
```

**Error propagation:**
- Uses `?` operator extensively to propagate errors up the call stack
- Top-level command handlers (in `src/commands/*.rs`) return `anyhow::Result<()>`
- Library code returns `common_repo::error::Result<T>`

## Logging

**Framework:** `log` crate with `env_logger`

**Patterns:**
- Debug output uses `println!()` in tests (e.g., `println!("Fetching repository for the first time (should clone)...");`)
- Production logging uses structured `log::` macros (debug, info, warn, error levels)
- Log level controlled via CLI `--log-level` flag or `--verbose`/`--quiet` overrides
- Example usage in `src/cli.rs`:
  - `--verbose` overrides `--log-level` to debug
  - `--verbose -v` (repeated) escalates to trace
  - `--quiet` suppresses everything except errors

## Comments

**When to Comment:**
- Module-level documentation using `//!` for every public module
- Function documentation using `///` for all public functions
- No comments on obvious code (e.g., "increment counter")
- Comments explain "why" not "what": `// Verify the schema is not empty` not `// Check if schema.is_empty()`

**JSDoc/TSDoc/Rustdoc:**
- All public modules have module-level docs: `//! # Module Name`
- All public functions have doc comments with examples where appropriate
- Extensive use of doc comments in `src/lib.rs`, `src/config.rs`, `src/filesystem.rs`, `src/operators.rs`

**Example from `src/filesystem.rs`:**
```rust
/// Creates a new `File` with the given content.
///
/// By default, the file is created with `0o644` permissions and the current
/// system time as the modification time.
///
/// # Examples
///
/// ```
/// use common_repo::filesystem::File;
///
/// let content = vec![72, 101, 108, 108, 111]; // "Hello"
/// let file = File::new(content);
///
/// assert_eq!(file.size(), 5);
/// assert_eq!(file.permissions, 0o644);
/// ```
pub fn new(content: Vec<u8>) -> Self { ... }
```

## Function Design

**Size:**
- Operators in `src/operators/*.rs` are typically 10-20 lines (focused, single responsibility)
- Command executors in `src/commands/*.rs` are 30-80 lines (handle argument parsing and delegation)
- No enforced maximum, but modularity preferred over long functions

**Parameters:**
- Use references for owned data that doesn't need mutation: `&MemoryFS`, `&IncludeOp`
- Use mutable references for filesystem modifications: `&mut MemoryFS`
- Pass borrowed data, not clones (except where cloning is necessary like in file operations)
- Configuration objects passed as single structs, not scattered parameters

**Example from `src/operators.rs`:**
```rust
pub(crate) fn apply(op: &IncludeOp, source: &MemoryFS, target: &mut MemoryFS) -> Result<()>
```

**Return Values:**
- All fallible operations return `Result<T>` not `Option<T>`
- Void operations return `Result<()>`
- Functions rarely return tuples; single return type is preferred
- Default trait used for initialization: `impl Default for MemoryFS { fn default() -> Self { ... } }`

## Module Design

**Exports:**
- All public items explicitly use `pub` keyword
- Private items use `pub(crate)` for internal module visibility (e.g., `pub(crate) fn apply()`)
- Modules export their public API cleanly: `pub struct File { ... }`, `pub struct MemoryFS { ... }`

**Barrel Files:**
- `src/lib.rs` declares all public modules: `pub mod cache;`, `pub mod config;`, etc.
- `src/commands/mod.rs` re-exports command modules and dispatches to them
- No wildcard re-exports; explicit imports encourage intentional public APIs

**Example from `src/lib.rs`:**
```rust
pub mod cache;
pub mod config;
pub mod defaults;
pub mod error;
pub mod filesystem;
pub mod git;
pub mod merge;
pub mod operators;
pub mod output;
pub mod path;
pub mod phases;
pub mod repository;
pub mod suggestions;
pub mod version;
```

## Code Patterns Observed

**Builder Pattern:**
- `DeferredMergeOp` uses a builder pattern with fluent API:
  ```rust
  pub fn new() -> Self { Self::default() }
  pub fn source(mut self, source: impl Into<String>) -> Self { ... }
  pub fn dest(mut self, dest: impl Into<String>) -> Self { ... }
  ```

**Type Aliases:**
- `Schema = Vec<Operation>` simplifies configuration representation
- `Result<T> = std::result::Result<T, Error>` throughout the library

**Trait Objects:**
- Limited trait usage; mostly concrete types
- `Operation` enum variants instead of trait objects for filesystem operations

**Match Expressions:**
- Exhaustive pattern matching on enums (no unhandled variants)
- Early returns in match arms when appropriate

---

*Convention analysis: 2026-03-16*
