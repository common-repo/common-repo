# Architecture Best Practices Research

Research compiled from OSS best practices (2024-2025) and analysis of exemplar Rust projects.

## Reference Projects Analyzed

1. **uv** (astral-sh/uv) - Python package manager written in Rust
2. **ripgrep** (BurntSushi/ripgrep) - Line-oriented search tool
3. **tokio** (tokio-rs/tokio) - Asynchronous runtime for Rust
4. **clap** (clap-rs/clap) - Command-line argument parser

## Module System Fundamentals

### Rust's Organizational Units

Rust's module system comprises four key components:

1. **Packages**: Cargo features for building, testing, and sharing crates
2. **Crates**: A tree of modules producing a library or executable
3. **Modules**: Controlling organization, scope, and privacy of paths
4. **Paths**: Naming items like structs, functions, or modules

### Visibility Rules

Items in modules default to private visibility. Two core access rules:

1. Private items can be accessed by the current module and its descendants
2. Public items are accessible from outside the module scope via `pub`

**Visibility Modifiers**:
- `pub` - Fully public
- `pub(crate)` - Visible within the current crate only (common pattern)
- `pub(super)` - Visible to parent module only
- `pub(in path)` - Visible within specified path

**Re-exporting Pattern**:
```rust
// lib.rs - Create clean public API
pub use internal::ImportantType;

mod internal {
    pub struct ImportantType;
}
```

## Workspace Organization

### Flat Layout (Recommended)

For projects between 10k-1M lines, use a flat structure:

```
project-root/
├── Cargo.toml          # Virtual manifest (no [package])
├── Cargo.lock
├── crates/
│   ├── project-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── project-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   └── project-utils/
│       ├── Cargo.toml
│       └── src/
├── tests/              # Integration tests
├── benches/            # Benchmarks
└── examples/           # Example programs
```

**Why flat is superior**:
1. Cargo's namespace is flat; matching structure prevents inconsistencies
2. `ls ./crates` shows all components at a glance
3. Flat structures don't deteriorate over time like hierarchies

### Workspace Configuration

Root `Cargo.toml` (virtual manifest):
```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
authors = ["Author <author@example.com>"]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
thiserror = "2.0"
```

Child crate `Cargo.toml`:
```toml
[package]
name = "project-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
```

### Best Practices

1. **Virtual manifest root**: Don't put a crate in the root; use only workspace definition
2. **Consistent naming**: Folder names should match crate names exactly
3. **Internal versioning**: Use `version = "0.0.0"` for unpublished internal crates
4. **Centralized dependencies**: Define common deps in workspace for consistency
5. **Standard structure**: Keep `src/` even for single-file crates
6. **Prefix conventions**: Related crates use common prefix (e.g., `hir_def`, `hir_ty`)

### Automation with cargo xtask

Instead of scattered shell scripts, use a dedicated Rust crate:

```
project-root/
├── crates/
│   └── xtask/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
```

`.cargo/config.toml`:
```toml
[alias]
xtask = "run --package xtask --"
```

Usage: `cargo xtask codegen`, `cargo xtask release`

## Error Handling Patterns

### Library Error Handling

Use **thiserror** for structured error types in libraries:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("configuration file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("invalid configuration format")]
    InvalidFormat(#[from] serde_json::Error),

    #[error("missing required field: {0}")]
    MissingField(String),
}
```

**When to use thiserror**:
- Callers need to match on error variants
- Providing a library API
- Error details are meaningful to consumers

### Application Error Handling

Use **anyhow** for error propagation in applications:

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;

    serde_json::from_str(&content)
        .context("failed to parse config")
}
```

**When to use anyhow**:
- Caller just needs to report/log the error
- Adding context to propagated errors
- Main application code, not library interfaces

### Large Project Error Handling

For complex workspaces with many crates, consider **snafu**:

```rust
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to read config from {path}"))]
    ReadConfig { source: std::io::Error, path: PathBuf },

    #[snafu(display("Failed to parse config"))]
    ParseConfig { source: serde_json::Error },
}

fn load_config(path: &Path) -> Result<Config, Error> {
    let content = std::fs::read_to_string(path)
        .context(ReadConfigSnafu { path })?;

    serde_json::from_str(&content)
        .context(ParseConfigSnafu)?;
}
```

**SNAFU advantages**:
- Combines thiserror-style types with anyhow-style context
- Context selectors for type-safe error construction
- Built-in backtrace and error chain support
- Better for workspace-wide error hierarchies

### Error Handling Decision Tree

```
Should caller handle error variants differently?
├── Yes → Use thiserror (enum with variants)
└── No → Is this library or application code?
    ├── Library → Use thiserror (opaque or simple)
    └── Application → Use anyhow
        └── Complex workspace? → Consider snafu
```

## API Design Guidelines

### Official Rust API Guidelines Summary

From the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

**Naming**:
- Follow RFC 430 casing conventions
- Getters: Use `field()` not `get_field()` (exception: `get(key)` for lookups)
- Conversions: `as_*` (cheap borrow), `to_*` (expensive), `into_*` (consuming)
- Iterators: `iter()`, `iter_mut()`, `into_iter()`

**Traits to Implement**:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Config { /* ... */ }
```

Common traits to consider:
- `Debug` - Almost always implement
- `Clone` - When copying makes sense
- `PartialEq`, `Eq` - For comparison
- `Hash` - When used as HashMap keys
- `Default` - When sensible default exists
- `Send`, `Sync` - For concurrent use
- `Serialize`, `Deserialize` - For persistence

**Conversions**:
```rust
impl From<RawConfig> for Config { /* ... */ }
impl AsRef<str> for ConfigPath { /* ... */ }
```

**Documentation**:
- Document all public items
- Include examples in rustdoc
- Document "Errors", "Panics", and "Safety" sections as relevant

### Builder Pattern

Use builders for types with multiple optional fields:

```rust
// Using typed-builder (compile-time checking)
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct ServerConfig {
    host: String,
    port: u16,
    #[builder(default = 30)]
    timeout_seconds: u32,
    #[builder(default, setter(strip_option))]
    tls_cert: Option<PathBuf>,
}

// Usage
let config = ServerConfig::builder()
    .host("localhost".to_string())
    .port(8080)
    .build();
```

**Builder crate options**:
- **typed-builder**: Compile-time verification, zero-cost
- **derive_builder**: Runtime checking, more flexible
- **bon**: Newer alternative with builder generation

**Manual builder pattern**:
```rust
pub struct ServerConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    timeout_seconds: u32,
}

impl ServerConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            timeout_seconds: 30,
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn build(self) -> Result<ServerConfig, ConfigError> {
        Ok(ServerConfig {
            host: self.host.ok_or(ConfigError::MissingField("host"))?,
            port: self.port.ok_or(ConfigError::MissingField("port"))?,
            timeout_seconds: self.timeout_seconds,
            tls_cert: None,
        })
    }
}
```

### Newtype Pattern

Use newtypes for type safety:

```rust
// Instead of raw strings
pub struct UserId(String);
pub struct Email(String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

## Exemplar Project Patterns

### uv (astral-sh/uv)

**Structure**: Monorepo with `crates/` directory
- 98% Rust codebase
- Flat workspace layout
- Cargo-style workspace management

**Key patterns**:
- Separate crates for distinct functionality
- Standard Rust tooling (clippy.toml, rustfmt.toml)
- Python bindings in dedicated directory

### ripgrep

**Structure**: Workspace with modular crates
- `crates/` for core components
- `tests/` for integration testing
- `benchsuite/` for performance testing

**Key patterns**:
- Separate crates for reusable components (ignore, grep-regex)
- CI scripts in `ci/` directory
- Platform packaging in `pkg/`

### tokio

**Structure**: Multi-crate workspace
- Core runtime in `tokio/`
- Utility crates: `tokio-util`, `tokio-stream`, `tokio-test`, `tokio-macros`
- Independent changelogs per crate

**Architecture layers**:
1. Task scheduler (work-stealing)
2. Reactor (OS event integration)
3. Networking APIs (async TCP/UDP)

**Key patterns**:
- Separate macro crate for procedural macros
- Feature flags for optional functionality
- Zero-cost abstraction philosophy

### clap

**Structure**: Multi-crate workspace with specialized components
- `clap_builder` - Core parsing
- `clap_derive` - Procedural macros
- `clap_complete` - Shell completions
- `clap_lex` - Lexical analysis
- `clap_mangen` - Man page generation

**Key patterns**:
- Dual API styles (derive and builder)
- Separate crates for optional features
- Examples demonstrating both approaches

## Module Organization Patterns

### Feature-Based Organization

```
src/
├── lib.rs
├── config/
│   ├── mod.rs
│   ├── parser.rs
│   └── validation.rs
├── network/
│   ├── mod.rs
│   ├── client.rs
│   └── server.rs
└── storage/
    ├── mod.rs
    ├── database.rs
    └── cache.rs
```

### Layer-Based Organization

```
src/
├── lib.rs
├── domain/          # Core business logic
│   ├── mod.rs
│   └── models.rs
├── application/     # Use cases
│   ├── mod.rs
│   └── services.rs
├── infrastructure/  # External concerns
│   ├── mod.rs
│   ├── database.rs
│   └── http.rs
└── presentation/    # CLI/API layer
    ├── mod.rs
    └── handlers.rs
```

### Prelude Pattern

Expose commonly-used items through a prelude:

```rust
// src/prelude.rs
pub use crate::config::Config;
pub use crate::error::{Error, Result};
pub use crate::traits::{Parser, Validator};

// In other modules
use crate::prelude::*;
```

## Key Takeaways

### Project Structure

1. Use flat `crates/` layout for workspaces
2. Make root a virtual manifest only
3. Match folder names to crate names exactly
4. Use cargo xtask for automation

### Error Handling

1. thiserror for libraries with meaningful error types
2. anyhow for applications focused on error reporting
3. Consider snafu for complex multi-crate workspaces
4. Always add context when propagating errors

### API Design

1. Follow official Rust API Guidelines
2. Implement standard traits (Debug, Clone, PartialEq, etc.)
3. Use builders for complex construction
4. Use newtypes for type safety
5. Prefer `pub(crate)` over `pub` for internal items

### Module Organization

1. Keep modules focused on single responsibility
2. Use re-exports to create clean public APIs
3. Consider prelude pattern for common imports
4. Feature-gate optional functionality

## Sources

### Project Structure
- [The Rust Book - Modules](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [Rust Project Structure Best Practices](https://www.djamware.com/post/68b2c7c451ce620c6f5efc56/rust-project-structure-and-best-practices-for-clean-scalable-code)
- [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)
- [Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Rust Project Primer - Workspace](https://rustprojectprimer.com/organization/workspace.html)

### Visibility and Privacy
- [Rust By Example - Visibility](https://doc.rust-lang.org/rust-by-example/mod/visibility.html)
- [The Rust Reference - Visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
- [Understanding Rust Privacy Model](https://iximiuz.com/en/posts/rust-privacy-and-visibility/)

### Error Handling
- [thiserror, anyhow, or How I Handle Errors](https://www.shakacode.com/blog/thiserror-anyhow-or-how-i-handle-errors-in-rust-apps/)
- [Rust Error Handling Guide](https://momori.dev/posts/rust-error-handling-thiserror-anyhow/)
- [Error Handling for Large Rust Projects](https://greptime.com/blogs/2024-05-07-error-rust)
- [Error Handling In Rust - A Deep Dive](https://lpalmieri.com/posts/error-handling-rust/)
- [SNAFU Documentation](https://docs.rs/snafu/latest/snafu/)
- [anyhow GitHub](https://github.com/dtolnay/anyhow)
- [thiserror crates.io](https://crates.io/crates/thiserror)

### API Design
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust API Guidelines Checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Nine Rules for Elegant Rust Library APIs](https://towardsdatascience.com/nine-rules-for-elegant-rust-library-apis-9b986a465247/)
- [Elegant Library APIs in Rust](https://deterministic.space/elegant-apis-in-rust.html)
- [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/)

### Builder Pattern
- [Builder - Rust Design Patterns](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)
- [Effective Rust - Builders](https://effective-rust.com/builders.html)
- [typed-builder GitHub](https://github.com/idanarye/rust-typed-builder)
- [derive_builder Documentation](https://docs.rs/derive_builder/latest/derive_builder/)

### Reference Projects
- [uv GitHub](https://github.com/astral-sh/uv)
- [ripgrep GitHub](https://github.com/BurntSushi/ripgrep)
- [tokio GitHub](https://github.com/tokio-rs/tokio)
- [clap GitHub](https://github.com/clap-rs/clap)
