# DEVIN.md

This file provides guidance to Devin when working with code in this repository.

## Project Overview

This is a Rust library for managing repository inheritance and file composition across multiple Git repositories. The project uses automated tooling for code quality, conventional commits, and semantic versioning with comprehensive CI/CD automation.

## LLM Context Files

The `context/` directory contains detailed implementation plans, progress tracking, and design documents specifically for LLM assistants. These files provide deep context about the project's architecture, implementation status, and future plans:

- `context/feature-status.json` - Structured JSON tracking of all feature implementation status
- `context/current-task.json` - Points to active task and its plan file
- `context/implementation-plan.md` - Detailed technical implementation plans and architecture decisions
- `context/cli-*.md` - CLI design, implementation plans, and testing strategies
- `context/merge-operator-testing-guide.md` - Testing guidance for merge operators
- `context/improving-test-coverage-plan.md` - Test coverage analysis and improvement plans

These files are temporary and will be removed once the project reaches maturity. For human-readable documentation, see:
- `docs/purpose.md` - Project purpose and goals
- `docs/design.md` - Implementation architecture and design philosophy
- `README.md` - User-facing documentation

## Quick Start

The repository has been set up and is ready to use. All dependencies are installed, tests pass, and pre-commit hooks are configured.

### Prerequisites

- Rust: Stable channel (automatically managed via `rust-toolchain.toml`)
- cargo-nextest: Required for running tests (see CLAUDE.md for setup instructions)
- prek: Recommended for pre-commit hooks (Rust-based, faster than Python pre-commit)
  - Install from GitHub: `cargo install --git https://github.com/j178/prek`
  - Alternative: pre-commit (Python-based, works as fallback)

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

**Using cargo-nextest (if installed):**
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
cargo test --features integration-tests

# Run only integration tests
cargo test --test integration_test --features integration-tests

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo test --features integration-tests
```

**Important Notes:**
- **Use unit tests during development** - they're fast and don't require network
- **Run integration tests before major changes** - they verify real-world functionality
- **Integration tests are disabled by default** to avoid network dependencies
- **All tests must pass** for CI/CD to succeed (both unit and integration tests)
- See CLAUDE.md for canonical test counts and coverage targets

### Code Quality

```bash
# Format code (must pass before committing)
cargo fmt

# Check formatting without modifying files
cargo fmt --check

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

The hooks are already installed. If you need to reinstall them:

**Recommended (Rust-based, faster):**
```bash
# Install latest version from GitHub (crates.io version is outdated)
cargo install --git https://github.com/j178/prek --locked
prek install
prek install --hook-type commit-msg
```

**Alternative (Python-based):**
```bash
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
- **Prek installation**: Always install prek from GitHub (`cargo install --git https://github.com/j178/prek`) as the crates.io version is outdated.
- Binary name is `common-repo` (matches the package name in Cargo.toml).

## Documentation Guidelines

When updating documentation files:
- Do not write specific numbers that will change, like line counts or percentages
- Do not write specific call outs of line numbers, since they will change over time
- When you are done modifying a document, review it for consistency and accuracy
- See CLAUDE.md for canonical test counts, coverage targets, and tooling guidance

## Project Structure

```
.
├── .github/
│   └── workflows/         # GitHub Actions CI/CD workflows
├── src/
│   ├── lib.rs             # Library entry point and public API
│   ├── main.rs            # Binary entry point (placeholder)
│   ├── cache.rs           # In-process repository caching
│   ├── config.rs          # YAML configuration parsing
│   ├── error.rs           # Error types and handling
│   ├── filesystem.rs      # In-memory filesystem implementation
│   ├── git.rs             # Git operations (clone, tags, etc.)
│   ├── operators.rs       # File operation implementations
│   ├── path.rs            # Path manipulation utilities
│   ├── phases.rs          # 6-phase processing pipeline
│   └── repository.rs      # High-level repository management
├── tests/
│   └── integration_test.rs # Integration tests (feature-gated)
├── Cargo.toml             # Rust package manifest
├── .pre-commit-config.yaml # Pre-commit hooks configuration
├── .commitlintrc.yml      # Commit message linting rules
├── CLAUDE.md              # Claude Code guidance
├── DEVIN.md               # This file - Devin guidance
└── README.md              # Project documentation
```

Note: This is primarily a library crate. Use it by adding `common-repo` as a dependency in your Cargo.toml.

## Workflow for Making Changes

1. **Create a feature branch** following the naming convention: `devin/{timestamp}-{descriptive-slug}`
   ```bash
   git checkout -b devin/$(date +%s)-feature-name
   ```

2. **Make your changes** following the existing code conventions

3. **Run tests** to ensure nothing breaks:
   ```bash
   cargo test
   ```

4. **Run formatting and linting** to ensure code quality:
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features -- -D warnings
   ```

5. **Commit using conventional commit format**:
   ```bash
   git add <files>
   git commit -m "feat: add new feature"
   ```
   The pre-commit hooks will automatically run and validate your changes.

6. **Push and create a PR**:
   ```bash
   git push origin devin/$(date +%s)-feature-name
   ```
   Then use the git_create_pr tool to create a pull request.

7. **Wait for CI to pass** using the git_check_pr tool before notifying the user.

## Common Tasks

### Adding a new feature
1. Create a feature branch
2. Implement the feature in the appropriate module
3. Add unit tests for the new functionality
4. Update documentation if needed
5. Run `cargo test` and `cargo clippy`
6. Commit with `feat:` prefix
7. Create a PR and wait for CI

### Fixing a bug
1. Create a bugfix branch
2. Write a failing test that reproduces the bug
3. Fix the bug
4. Verify the test now passes
5. Run `cargo test` and `cargo clippy`
6. Commit with `fix:` prefix
7. Create a PR and wait for CI

### Updating documentation
1. Create a docs branch
2. Update the relevant documentation files
3. Commit with `docs:` prefix
4. Create a PR

## Troubleshooting

### Rust version issues
If you encounter errors about missing features or edition requirements, the project uses rust-toolchain.toml to manage the Rust version automatically. Ensure rustup respects the toolchain file, or run:
```bash
./script/setup
```

### Pre-commit hook failures
If pre-commit hooks fail, fix the issues and try again:
```bash
# Format code
cargo fmt

# Fix clippy warnings
cargo clippy --all-targets --all-features --fix

# Re-run pre-commit (recommended)
prek run --all-files

# Or use Python pre-commit
pre-commit run --all-files
```

### Test failures
If tests fail, run them with verbose output to see details:
```bash
cargo test -- --nocapture
```

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Pre-commit Documentation](https://pre-commit.com/)
- Project README.md for detailed usage examples
- CLAUDE.md for additional Claude-specific guidance
