# common-repo

A Rust library for managing repository inheritance and file composition across multiple Git repositories.

## Overview

`common-repo` provides a sophisticated system for:
- **Repository Inheritance**: Pull and compose files from multiple Git repositories
- **File Operations**: Include, exclude, rename, and template files using declarative YAML configuration
- **6-Phase Processing Pipeline**: Efficient parallel processing with automatic caching
- **In-Memory Filesystem**: Fast file manipulation without disk I/O during processing

Perfect for projects that need to compose shared configurations, templates, or code from multiple sources.

## Features

- **Automated Testing & CI/CD**: GitHub Actions workflows for continuous integration
- **Declarative Configuration**: YAML-based schema for complex file operations
- **Smart Caching**: Automatic caching of cloned repositories for performance
- **Type-Safe Operators**: Strongly-typed operation system with comprehensive error handling
- **Code Quality**: Pre-commit hooks with rustfmt and clippy
- **Conventional Commits**: Enforced commit message standards

## Getting Started

### Prerequisites

- Rust 1.90.0 or later
- Python 3.7+ (for pre-commit hooks)

### Installation

```bash
# Clone the repository
git clone <your-repo-url>
cd common-repo

# Build the project
cargo build

# Run tests
cargo test
```

### Testing

This project includes comprehensive testing with both unit tests and integration tests:

#### Unit Tests
Unit tests verify individual components and functions:

```bash
# Run unit tests only (default, no network required)
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run a specific unit test
cargo test test_name

# Run tests for a specific module
cargo test mod::test_name
```

#### Integration Tests
Integration tests verify end-to-end functionality with real repositories and network operations:

```bash
# Run all tests including integration tests (requires network)
cargo test --features integration-tests

# Run only integration tests
cargo test --test integration_test --features integration-tests

# Run integration tests with verbose output
cargo test --test integration_test --features integration-tests -- --nocapture

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo test --features integration-tests

# Run integration tests only (skip unit tests)
SKIP_NETWORK_TESTS=1 cargo test --test integration_test --features integration-tests
```

**Integration Tests Overview:**
- Verify end-to-end repository inheritance pipeline
- Test real GitHub repository cloning and caching
- Validate performance improvements from caching
- Confirm filesystem operations work correctly
- Disabled by default since they require network access

**Test Coverage:**
- **183+ unit tests** covering all core functionality
- **5 integration tests** validating end-to-end workflows
- **14 datatest tests** for schema parsing (automatically discover test cases from YAML files)
- **Test coverage analysis** available via cargo-tarpaulin

#### Test Coverage Analysis

This project uses [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for test coverage metrics:

```bash
# Install tarpaulin (if not already installed)
cargo install cargo-tarpaulin

# Generate HTML coverage report
cargo tarpaulin --out Html

# Generate terminal coverage report
cargo tarpaulin

# Generate coverage report for CI integration
cargo tarpaulin --out Xml

# Set minimum coverage threshold
cargo tarpaulin --fail-under 80

# Generate detailed HTML report
cargo tarpaulin --out Html --output-dir target/tarpaulin
```

Coverage reports are saved to `target/tarpaulin/`. Open `target/tarpaulin/tarpaulin-report.html` in a browser to view the HTML report.

For detailed coverage analysis and areas for improvement, see `TEST_COVERAGE_ANALYSIS.md`.

## Development Workflow

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. Install them with:

```bash
# Install pre-commit
pip install pre-commit

# Install the git hooks
pre-commit install
pre-commit install --hook-type commit-msg
```

The hooks will automatically:
- Format code with `rustfmt`
- Run `cargo check`
- Lint code with `clippy`
- Validate commit messages follow conventional commits
- Check for trailing whitespace and other common issues

### Commit Message Format

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system changes
- `ci`: CI/CD changes
- `chore`: Other changes that don't modify src or test files

**Examples:**
```
feat: add user authentication module
fix: resolve memory leak in data parser
docs: update installation instructions
```

### Running Tests

See the comprehensive [Testing](#testing) section above for detailed instructions on running unit tests and integration tests.

### Code Formatting & Linting

```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run clippy
cargo clippy

# Run clippy with all features
cargo clippy --all-targets --all-features -- -D warnings
```

## CI/CD Pipeline

### Continuous Integration

On every push and pull request to `main`, GitHub Actions will:
1. Run all tests
2. Check code formatting
3. Run clippy linting
4. Build release binary

### Automated Releases

The project uses [Release Please](https://github.com/googleapis/release-please) for automated releases:

1. Commits following conventional commits are analyzed
2. Version numbers are automatically bumped (major/minor/patch)
3. CHANGELOG.md is automatically updated
4. GitHub releases are created with compiled binaries

**Version Bumping Rules:**
- `feat`: Minor version bump (0.1.0 -> 0.2.0)
- `fix`: Patch version bump (0.1.0 -> 0.1.1)
- `feat!` or `BREAKING CHANGE`: Major version bump (0.1.0 -> 1.0.0)

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
└── README.md              # This file
```

Note: This is primarily a library crate. Use it by adding `common-repo` as a dependency
in your Cargo.toml.

## Contributing

1. Create a feature branch
2. Make your changes
3. Ensure tests pass: `cargo test`
4. Ensure formatting is correct: `cargo fmt`
5. Ensure linting passes: `cargo clippy`
6. Commit using conventional commit format
7. Push and create a pull request

## License

[Add your license here]
