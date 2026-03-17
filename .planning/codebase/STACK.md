# Technology Stack

**Analysis Date:** 2026-03-16

## Languages

**Primary:**
- Rust 1.85.0+ - Core application, CLI tool, and library crate

**Secondary:**
- Python 3.x - Used for pre-commit hooks and CI/CD tooling
- YAML - Configuration files (.common-repo.yaml, GitHub Actions)
- TOML - Cargo manifests and project configuration
- JSON - Configuration and data serialization
- Markdown - Documentation and changelog

## Runtime

**Environment:**
- Rust stable toolchain with rustfmt and clippy components
- Cargo (Rust package manager) - Version managed via rust-toolchain.toml
- Linux/Unix/macOS compatible (no Windows-specific tooling detected)

**Package Manager:**
- Cargo - Primary dependency manager
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Core:**
- `clap` 4.5 - Command-line argument parsing with derive macros
- `clap_complete` 4.5 - Shell completion generation
- `serde` 1.0 with derive - Serialization/deserialization framework
- `serde_yaml` 0.9 - YAML parsing for `.common-repo.yaml` files
- `serde_json` 1.0 - JSON parsing and manipulation
- `toml` 0.8 - TOML parsing
- `taplo` 0.9 - TOML analysis and editing

**CLI/UX:**
- `indicatif` 0.17 - Progress bars and spinners
- `console` 0.15 - Terminal output and formatting
- `dialoguer` 0.11 - Interactive user prompts
- `ptree` 0.4 - Tree-structured output display
- `log` 0.4 - Logging facade
- `env_logger` 0.11 - Environment-based logging configuration

**Testing:**
- `proptest` 1.5 - Property-based testing
- `datatest-stable` 0.1 - Data-driven test discovery (YAML schema tests)
- `serial_test` 3.0 - Sequential test execution for concurrency-sensitive tests
- `insta` 1.41 with YAML feature - Snapshot testing
- `criterion` 0.5 with HTML reports - Benchmarking

**E2E Testing:**
- `assert_cmd` 2.0 - Command execution assertions
- `predicates` 3.1 - Predicate assertions
- `assert_fs` 1.1 - Filesystem assertions

## Key Dependencies

**Critical:**
- `anyhow` 1.0 - Error handling with context
- `thiserror` 1.0 - Structured error types
- `url` 2.5 - URL parsing and manipulation (for Git repository URLs)
- `semver` 1.0 - Semantic version parsing and comparison (version checking)
- `regex` 1.10 - Regular expression support

**Infrastructure:**
- `walkdir` 2.5 - Recursive directory traversal
- `glob` 0.3 - Glob pattern matching
- `dirs` 5.0 - Standard directory locations (cache, config)
- `rayon` 1.10 - Data parallelism (parallel repository cloning)
- `rust-ini` 0.21.3 - INI file parsing
- `pulldown-cmark` 0.10 - Markdown parsing
- `tempfile` 3.0 - Temporary file management (testing)

## Configuration

**Environment:**
- Git configuration via `~/.gitconfig` (SSH keys, credentials)
- Git credential helpers for authentication
- SSH agent for key management
- Standard environment variables for CLI output:
  - `NO_COLOR` - Disable colored output
  - `CLICOLOR` - Control color mode (set to "0" to disable)
  - `CLICOLOR_FORCE` - Force color output
  - `TERM` - Terminal type detection
  - `CARGO_TERM_COLOR` - Cargo output color (CI config)
  - `CARGO_INCREMENTAL` - Incremental compilation (CI: disabled)
  - `RUSTFLAGS` - Compiler flags (CI: `-D warnings`)
  - `RUST_BACKTRACE` - Backtrace verbosity (CI: 1)

**Build:**
- `Cargo.toml` - Main package manifest with workspace members
- `xtask/Cargo.toml` - Build automation tasks (cargo xtask)
- `.config/nextest.toml` - Test execution configuration
- `rust-toolchain.toml` - Pinned stable Rust version
- `deny.toml` - Cargo-deny security and license audit configuration

**Release:**
- `release-please-config.json` - Semantic versioning and changelog automation

## Profiles & Optimization

**Release Profile:**
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for maximum optimization
- `strip = true` - Symbol stripping for smaller binaries

**Size-Optimized Profile** (release-small):
- `opt-level = "z"` - Optimize for minimal size
- `panic = "abort"` - Smaller unwinding strategy

## Platform Requirements

**Development:**
- Rust 1.85.0+ (MSRV enforced in CI)
- Git command-line tool installed and in PATH
- Python 3.x for pre-commit hooks
- Cargo with nextest support (cargo-nextest)
- Standard Unix utilities (find, grep, etc.)

**Production:**
- Git command-line tool for repository cloning
- SSH or HTTPS credentials for Git authentication
- Filesystem write permissions to cache directory
- macOS, Linux, or Unix-like environment

**CI/CD:**
- GitHub Actions runner (ubuntu-latest)
- Tools installed dynamically:
  - cargo-nextest - Parallel test execution
  - cargo-audit - Security vulnerability checking
  - cargo-deny - Dependency audit
  - cargo-tarpaulin - Code coverage
  - action-validator - GitHub Actions YAML validation

---

*Stack analysis: 2026-03-16*
