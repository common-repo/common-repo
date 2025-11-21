# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project with automated tooling for code quality, conventional commits, and semantic versioning. The project is configured for modern development practices with comprehensive CI/CD automation.

## LLM Context Files

The `context/` directory contains detailed implementation plans, progress tracking, and design documents specifically for LLM assistants. These files provide deep context about the project's architecture, implementation status, and future plans:

- `context/implementation-progress.md` - Comprehensive tracking of completed features and implementation status
- `context/implementation-plan.md` - Detailed technical implementation plans and architecture decisions
- `context/cli-*.md` - CLI design, implementation plans, and testing strategies
- `context/merge-operator-testing-guide.md` - Testing guidance for merge operators
- `context/improving-test-coverage-plan.md` - Test coverage analysis and improvement plans

These files are temporary and will be removed once the project reaches maturity. For human-readable documentation, see:
- `docs/purpose.md` - Project purpose and goals
- `docs/design.md` - Implementation architecture and design philosophy
- `README.md` - User-facing documentation

## Requirements

- **Rust**: Stable channel (automatically managed via `rust-toolchain.toml`)
  - The project requires Rust stable with support for edition 2024 features
  - Install Rust from https://rustup.rs/
  - The toolchain file will automatically ensure you have the correct version
- **cargo-nextest**: Required for running tests (see setup instructions below)
- **prek**: Recommended for pre-commit hooks (Rust-based, faster than Python pre-commit)
  - **IMPORTANT**: Install from GitHub (`cargo install --git https://github.com/j178/prek`) as crates.io version is outdated
  - Alternative: **pre-commit** (Python-based, works as fallback)

## Quick Setup

This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern for a normalized development workflow.

For first-time setup after cloning:

**Option 1: Manual installation (RECOMMENDED to avoid timeouts):**
```bash
# Install development tools individually to avoid compilation timeouts
cargo install cargo-nextest --locked
cargo install --git https://github.com/j178/prek --locked

# Then run setup to configure hooks and build the project
./script/setup
```

**Option 2: Automated setup (may timeout during compilation):**
```bash
# Set up the project (installs dependencies and configures environment)
# WARNING: This compiles cargo-nextest and prek, which may timeout in resource-constrained environments
./script/setup
```

The setup process will:
- Install Rust toolchain (via rust-toolchain.toml)
- Install cargo-nextest (using cargo-binstall if available, otherwise cargo install)
- Install prek (Rust-based pre-commit tool) and configure hooks
- Build the project to warm the cache

**Note**: If `./script/setup` times out during cargo-nextest or prek installation, use Option 1 to install these tools individually first.

### Available Scripts

- `./script/bootstrap` - Install all dependencies
- `./script/setup` - Set up project for first-time use (calls bootstrap)
- `./script/update` - Update project after pulling changes
- `./script/test` - Run the test suite (uses cargo-nextest if available)
- `./script/cibuild` - Run CI build locally (formatting, linting, tests)

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

**Automated installation (recommended):**
```bash
# Run the setup script (installs cargo-nextest and sets up pre-commit hooks)
./script/setup
```

**Manual installation:**
```bash
# Install cargo-nextest (one-time setup)
cargo install cargo-nextest --locked

# Or use cargo-binstall for faster installation (if available)
cargo binstall cargo-nextest
```

**Note:** The `./script/test` command will automatically use cargo-nextest if available, or fall back to `cargo test` with a helpful message.

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
- Target: 90%+ line coverage for all modules
- See `context/improving-test-coverage-plan.md` for detailed coverage analysis and improvement areas

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

- Create documentation following the [Rustdoc guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html) and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) as well as the [Rustdoc std-dev style guide](https://std-dev-guide.rust-lang.org/development/how-to-write-documentation.html).
- Documentation should link to files or other documentation appropriately.
- Do not use emojis or overly enthusiastic or hype language in documentation.
- Do not write specific numbers that will change, like "with over 73.5% coverage".
- Do not write specific call outs of line numbers like, "see fileblah.rs (line 123)", since they will change over time.
- When you are done modifying a document, review it for consistency, and accuracy.

## Committing and Pushing Guidelines for Claude Code

**CRITICAL**: When working with Claude Code, follow these guidelines:

1. **NEVER commit and push without explicit user approval**
   - Always ask the user before committing changes
   - Always ask the user before pushing to remote
   - Exception: User explicitly says "commit and push" or similar

2. **Avoid hardcoding ANY values in tests that will change over time**
   - No specific version numbers (v0.9.0, v1.0.0, etc.)
   - No specific dates or timestamps
   - No major version assumptions (v0., v1., etc.)
   - Use comparisons, regex parsing, or dynamic checks instead
   - Parse from source files (Cargo.toml) if you need current values

3. **When fixing tests**:
   - Understand what behavior the test is validating
   - Fix the underlying issue, not just update expectations
   - If expectations need updating, make them flexible/dynamic
   - Always run tests locally before claiming they're fixed

4. **Keep summaries brief**:
   - 1-2 sentences maximum
   - No code samples in summaries unless explicitly requested
   - Focus on what changed and why

## Pre-Commit Checklist

**IMPORTANT**: Always follow this checklist before committing to avoid CI failures:

1. **Run prek (RECOMMENDED)**: Run `prek run --all-files` to automatically format, lint, and validate all files
   - This runs cargo fmt, cargo clippy, and all other pre-commit hooks automatically
   - Catches issues before committing
2. **Alternative - Run checks individually**:
   - **Format code**: Run `cargo fmt` to ensure consistent code formatting
   - **Run linting**: Run `cargo clippy --all-targets --all-features -- -D warnings` to catch warnings
   - **Run tests**: Run `./script/test` or `cargo test` to ensure all tests pass
3. **Update documentation**: If you've completed a feature, update `context/implementation-progress.md`
4. **Write conventional commit**: Ensure commit message is < 100 characters and follows format: `type(scope): description`
5. **Check branch name**: For claude/agent branches, ensure name ends with session ID (e.g., `claude/feature-018evyqR5BZFzuZW5AuM9XRR`)

**Quick verification before push**:
```bash
# Run all CI checks locally
./script/cibuild

# Or run pre-commit hooks on all files (recommended)
prek run --all-files

# Or run checks individually
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

**Common CI failures to avoid**:
- ❌ Commit message too long (>100 chars) → Use concise conventional commit format
- ❌ Code not formatted → Run `prek run --all-files` or `cargo fmt` before committing
- ❌ Clippy warnings → Run `prek run --all-files` or fix with `cargo clippy`
- ❌ Pre-commit hooks failed → Always run `prek run --all-files` before committing
- ❌ Branch name doesn't end with session ID → Rename branch to include session ID

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

**Recommended (Rust-based, faster):**
```bash
# Install latest version from GitHub (crates.io version is outdated)
cargo install --git https://github.com/j178/prek --locked
prek install
prek install --hook-type commit-msg
```

**Alternative (Python-based):**
```bash
pip install pre-commit
pre-commit install
pre-commit install --hook-type commit-msg
```

**Note**: The `./script/setup` command automatically installs and configures prek if available.

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
- **Prek installation**: Always install prek from GitHub (`cargo install --git https://github.com/j178/prek`) as the crates.io version (0.0.1) is outdated. The bootstrap script handles this automatically.
- Binary name is `common-repo` (matches the package name in Cargo.toml).
