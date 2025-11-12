# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project with automated tooling for code quality, conventional commits, and semantic versioning. The project is configured for modern development practices with comprehensive CI/CD automation.

## Development Commands

### Building and Running
```bash
# Source cargo environment (if needed in new shells)
. "$HOME/.cargo/env"

# Build the project
cargo build

# Build release binary
cargo build --release

# Run the application
cargo run
```

### Testing

This project has comprehensive testing with both unit tests and integration tests:

#### Unit Tests (Recommended for development)
```bash
# Run unit tests only (fast, no network required)
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run a specific unit test
cargo test test_name

# Run tests for a specific module
cargo test mod::test_name
```

#### Integration Tests (Requires network)
Integration tests verify end-to-end functionality with real repositories:

```bash
# Run all tests including integration tests
cargo test --features integration-tests

# Run only integration tests
cargo test --test integration_test --features integration-tests

# Run integration tests with verbose output
cargo test --test integration_test --features integration-tests -- --nocapture

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo test --features integration-tests
```

**Important Notes:**
- **Use unit tests during development** - they're fast and don't require network
- **Run integration tests before major changes** - they verify real-world functionality
- **Integration tests are disabled by default** to avoid network dependencies
- **All tests must pass** for CI/CD to succeed (both unit and integration tests)
- **93 unit tests** cover individual components and functions (including repository sub-path filtering)
- **5 integration tests** validate end-to-end repository inheritance workflows

### Code Quality
```bash
# Format code (must pass before committing)
cargo fmt

# Check formatting without modifying files
cargo fmt -- --check

# Run clippy linting (configured to fail on warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Run all quality checks at once
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
```

## Commit Message Requirements

This repository enforces **conventional commits** via pre-commit hooks and CI. All commits must follow this format:

```
<type>(<scope>): <description>
```

Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

Examples:
- `feat: add user authentication module`
- `fix: resolve memory leak in data parser`
- `docs: update installation instructions`

Breaking changes require either:
- `feat!: breaking change description` or
- `BREAKING CHANGE:` in the commit footer

## Pre-commit Hooks

Pre-commit hooks are configured in `.pre-commit-config.yaml` and will automatically run:
- `cargo fmt` (formatting)
- `cargo check` (compilation)
- `cargo clippy` (linting with `-D warnings`)
- Conventional commit validation
- Trailing whitespace and YAML checks

If pre-commit hooks are not installed, install them with:
```bash
pip install pre-commit
pre-commit install
pre-commit install --hook-type commit-msg
```

## CI/CD Architecture

### GitHub Actions Workflows

1. **CI Pipeline** (`.github/workflows/ci.yml`)
   - Triggers on push/PR to `main`
   - **Lint job**: Runs pre-commit checks on all files (rustfmt, clippy, conventional commits validation, etc.)
   - **Test job**: Runs all tests with caching for cargo registry/index/build artifacts
   - **Rustfmt job**: Checks code formatting
   - **Clippy job**: Runs linting checks
   - **Build job**: Creates release binary

2. **Commit Linting** (`.github/workflows/commitlint.yml`)
   - Validates commit messages in PRs
   - Enforces conventional commit format

## Important Notes for Development

- **Lint job is required**: The CI pipeline includes a dedicated Lint job that runs pre-commit on all files. This job must pass for PRs to be merged.
- **Clippy is strict**: The project treats all clippy warnings as errors (`-D warnings`). Fix all warnings before committing.
- **Formatting is mandatory**: Code must be formatted with `cargo fmt` before commits will be accepted.
- **Commit messages are validated**: Both pre-commit hooks and CI will reject improperly formatted commit messages.
- Binary name is `common-repo` (matches the package name in Cargo.toml).
