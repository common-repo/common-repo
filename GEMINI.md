# GEMINI.md

This file provides guidance to Gemini when working with code in this repository.

## Project Overview

This is a Rust project with automated tooling for code quality, conventional commits, and semantic versioning. The project is configured for modern development practices with comprehensive CI/CD automation.

## LLM Context Files

The `context/` directory contains task tracking for LLM assistants:

- `context/current-task.json` - Points to active task and its plan file
- `context/completed/` - Archived plans and status files (feature-status.json, implementation plans, testing guides)

For human-readable documentation, see:
- `docs/purpose.md` - Project purpose and goals
- `docs/design.md` - Implementation architecture and design philosophy
- `README.md` - User-facing documentation

## Requirements

- **Rust**: Stable channel (automatically managed via `rust-toolchain.toml`)
  - The project requires Rust stable with support for edition 2024 features
  - The toolchain file will automatically ensure you have the correct version
- **cargo-nextest**: Required for running tests
- **prek**: Recommended for pre-commit hooks (Rust-based, faster than Python pre-commit)
  - **IMPORTANT**: Install from GitHub (`cargo install --git https://github.com/j178/prek`) as crates.io version is outdated
  - Alternative: **pre-commit** (Python-based, works as fallback)

## Quick Setup

This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern for a normalized development workflow.

For first-time setup:

```bash
# Install development tools individually to avoid compilation timeouts
cargo install cargo-nextest --locked
cargo install --git https://github.com/j178/prek --locked

# Then run setup to configure hooks and build the project
./script/setup
```

### Available Scripts

- `./script/bootstrap` - Install all dependencies
- `./script/setup` - Set up project for first-time use (calls bootstrap)
- `./script/update` - Update project after pulling changes
- `./script/test` - Run the test suite (uses cargo-nextest if available)
- `./script/cibuild` - Run CI build locally (formatting, linting, tests)

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Build release binary
cargo build --release

# Run the application
cargo run
```

### Testing

This project has comprehensive testing with both unit tests and integration tests.

**Recommended: Use cargo-nextest** for faster test execution and better reporting.

#### Unit Tests (Recommended for development)

```bash
# Run unit tests only (fast, no network required)
cargo nextest run

# Run tests with verbose output
cargo nextest run --verbose

# Run a specific unit test
cargo nextest run test_name
```

#### Integration Tests (Requires network)
Integration tests verify end-to-end functionality with real repositories:

```bash
# Run all tests including integration tests
cargo nextest run --features integration-tests

# Run only integration tests
cargo nextest run --test integration_test --features integration-tests

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo nextest run --features integration-tests
```

#### Finding Slow Tests

```bash
# Run tests with the CI profile to see slow test warnings
cargo nextest run --profile ci
```

**Important Notes:**
- **Use unit tests during development** - they're fast and don't require network
- **Run integration tests before major changes** - they verify real-world functionality
- **Integration tests are disabled by default** to avoid network dependencies
- **All tests must pass** for CI/CD to succeed

#### Test Coverage (Tarpaulin)

```bash
# Generate coverage report (HTML output)
cargo tarpaulin --out Html
```

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

## Documentation style guide

- Create documentation following the [Rustdoc guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html) and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Documentation should link to files or other documentation appropriately.
- Do not use emojis or overly enthusiastic or hype language in documentation.
- Do not write specific numbers that will change (e.g. "over 73.5% coverage").
- Do not write specific call outs of line numbers.
- When you are done modifying a document, review it for consistency, and accuracy.

## Pre-Commit Checklist

**IMPORTANT**: Always follow this checklist before committing to avoid CI failures:

1. **Run prek (RECOMMENDED)**: Run `prek run --all-files` to automatically format, lint, and validate all files
   - This runs cargo fmt, cargo clippy, and all other pre-commit hooks automatically
2. **Alternative - Run checks individually**:
   - **Format code**: `cargo fmt`
   - **Run linting**: `cargo clippy --all-targets --all-features -- -D warnings`
   - **Run tests**: `./script/test`
3. **Write conventional commit**: Ensure commit message is < 100 characters and follows format: `type(scope): description`

**Quick verification before push**:
```bash
# Run all CI checks locally
./script/cibuild
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
- `cargo fmt`
- `cargo check`
- `cargo clippy`
- Conventional commit validation
- Trailing whitespace and YAML checks

## CI/CD Architecture

### GitHub Actions Workflows

1. **CI Pipeline** (`.github/workflows/ci.yml`)
   - Triggers on push/PR to `main`
   - **Lint job**: Runs pre-commit checks on all files
   - **Test job**: Runs all tests
   - **Rustfmt job**: Checks code formatting
   - **Clippy job**: Runs linting checks
   - **Build job**: Creates release binary

2. **Commit Linting** (`.github/workflows/commitlint.yml`)
   - Validates commit messages in PRs

## Important Notes for Development

- **Lint job is required**: The CI pipeline includes a dedicated Lint job that runs pre-commit on all files. This job must pass for PRs to be merged.
- **Clippy is strict**: The project treats all clippy warnings as errors (`-D warnings`). Fix all warnings before committing.
- **Formatting is mandatory**: Code must be formatted with `cargo fmt` before commits will be accepted.
- **Commit messages are validated**: Both pre-commit hooks and CI will reject improperly formatted commit messages.
- **Prek installation**: Always install prek from GitHub as the crates.io version is outdated.
- Binary name is `common-repo`.
