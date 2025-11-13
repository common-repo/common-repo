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

This project has comprehensive testing with both unit tests and integration tests.

**Recommended: Use cargo-nextest** for faster test execution and better reporting.

#### Installing cargo-nextest
```bash
# Install cargo-nextest (one-time setup)
cargo install cargo-nextest --locked

# Or use cargo-binstall for faster installation
cargo binstall cargo-nextest
```

#### Unit Tests (Recommended for development)

**Using cargo-nextest (preferred):**
```bash
# Run unit tests only (fast, no network required)
cargo nextest run

# Run tests with verbose output
cargo nextest run --verbose

# Run a specific unit test
cargo nextest run test_name

# Run tests for a specific module
cargo nextest run -E 'test(mod::test_name)'

# Identify slow tests
cargo nextest run --profile ci
```

**Using standard cargo test:**
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

**Using cargo-nextest (preferred):**
```bash
# Run all tests including integration tests
cargo nextest run --features integration-tests

# Run only integration tests
cargo nextest run --test integration_test --features integration-tests

# Run integration tests with verbose output
cargo nextest run --test integration_test --features integration-tests --verbose

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo nextest run --features integration-tests
```

**Using standard cargo test:**
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

#### Finding Slow Tests

Cargo-nextest can identify tests that exceed configured slow test thresholds:

```bash
# Run tests with the CI profile to see slow test warnings
cargo nextest run --profile ci

# Run with specific slow test threshold
cargo nextest run --profile default --slow-timeout 60s

# Generate detailed timing report
cargo nextest run --verbose
```

Configuration is in `.config/nextest.toml`. Slow tests will be highlighted in CI output.

**Important Notes:**
- **Use unit tests during development** - they're fast and don't require network
- **Run integration tests before major changes** - they verify real-world functionality
- **Integration tests are disabled by default** to avoid network dependencies
- **All tests must pass** for CI/CD to succeed (both unit and integration tests)
- **186 total tests** (167 unit, 5 integration, 14 datatest) cover individual components and full workflows
- **5 integration tests** validate end-to-end repository inheritance workflows
- **14 datatest tests** for schema parsing (automatically discover test cases from YAML files)
- **cargo-nextest is used in CI** for faster execution and slow test detection

#### Test Coverage (Tarpaulin)

This project uses [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for test coverage analysis:

```bash
# Install tarpaulin (if not already installed)
cargo install cargo-tarpaulin

# Generate coverage report (HTML output)
cargo tarpaulin --out Html

# Generate coverage report (terminal output)
cargo tarpaulin

# Generate coverage report with specific output format
cargo tarpaulin --out Xml  # For CI integration
cargo tarpaulin --out Stdout  # Terminal output

# Exclude integration tests from coverage
cargo tarpaulin --tests

# Generate coverage for specific modules
cargo tarpaulin --tests --lib

# Set minimum coverage threshold (fails if below threshold)
cargo tarpaulin --fail-under 80

# Generate detailed coverage report
cargo tarpaulin --out Html --output-dir target/tarpaulin
```

Coverage reports are generated in `target/tarpaulin/` directory. HTML reports can be viewed by opening `target/tarpaulin/tarpaulin-report.html` in a browser.

**Coverage Goals:**
- Target: 80%+ line coverage for all modules
- Critical modules: 90%+ coverage (phases, operators, repository)
- See `TEST_COVERAGE_ANALYSIS.md` for detailed coverage analysis and improvement areas

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
