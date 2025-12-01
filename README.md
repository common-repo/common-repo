# common-repo

A CLI tool for managing repository configuration inheritance and file composition across multiple Git repositories.

## Overview

`common-repo` treats repository configuration files as software dependencies. Define your configuration inheritance in a `.common-repo.yaml` file and let the tool handle versioning, updates, and intelligent merging.

**Key capabilities:**
- **Repository Inheritance**: Pull and compose files from multiple Git repositories
- **File Operations**: Include, exclude, rename, and template files using declarative YAML configuration
- **Intelligent Merging**: Merge YAML, JSON, TOML, INI, and Markdown files with path-based targeting
- **Version Management**: Pin refs, detect updates, and upgrade dependencies
- **6-Phase Processing Pipeline**: Efficient processing with automatic caching

Perfect for standardizing CI/CD configs, pre-commit hooks, and other repository infrastructure across projects.

## Features

- **Automated Testing & CI/CD**: GitHub Actions workflows for continuous integration
- **Declarative Configuration**: YAML-based schema for complex file operations
- **Smart Caching**: Automatic caching of cloned repositories for performance
- **Type-Safe Operators**: Strongly-typed operation system with comprehensive error handling
- **Code Quality**: Pre-commit hooks with rustfmt and clippy
- **Conventional Commits**: Enforced commit message standards

## Getting Started

### Prerequisites

- **Rust**: Stable channel (automatically managed via `rust-toolchain.toml`)
  - Install from https://rustup.rs/
  - The project requires Rust stable with support for edition 2024 features
  - The toolchain file will automatically ensure you have the correct version
- **cargo-nextest**: Required for running tests (installed via setup script)
- **Python 3.7+**: Optional, for pre-commit hooks

### Installation

This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern for a normalized development workflow.

```bash
# Clone the repository
git clone <your-repo-url>
cd common-repo

# Run the setup script (recommended for first-time setup)
# This installs dependencies, configures hooks, and builds the project
./script/setup

# Run tests (automatically uses cargo-nextest if available)
./script/test
```

**Available Scripts:**
- `./script/bootstrap` - Install all dependencies (cargo-nextest, pre-commit, etc.)
- `./script/setup` - Set up project for first-time use (calls bootstrap + configures environment)
- `./script/update` - Update project after pulling changes
- `./script/test` - Run the test suite (uses cargo-nextest if available, falls back to cargo test)
- `./script/cibuild` - Run CI build locally (formatting, linting, tests)

### Testing

This project includes comprehensive testing with both unit tests and integration tests.

**Recommended: Use cargo-nextest** for faster test execution and better reporting. Install it via `./script/setup` or manually with `cargo install cargo-nextest --locked`.

**Quick test command:** Use `./script/test` which automatically uses cargo-nextest if available, or falls back to `cargo test` with a helpful message.

#### Unit Tests
Unit tests verify individual components and functions:

**Using cargo-nextest (recommended):**
```bash
# Run unit tests only (default, no network required)
cargo nextest run

# Run tests with verbose output
cargo nextest run --verbose

# Run a specific unit test
cargo nextest run test_name

# Run tests for a specific module
cargo nextest run -E 'test(mod::test_name)'
```

**Using standard cargo test:**
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

**Using cargo-nextest (recommended):**
```bash
# Run all tests including integration tests (requires network)
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
# Run all tests including integration tests (requires network)
cargo test --features integration-tests

# Run only integration tests
cargo test --test integration_test --features integration-tests

# Run integration tests with verbose output
cargo test --test integration_test --features integration-tests -- --nocapture

# Skip network-dependent integration tests
SKIP_NETWORK_TESTS=1 cargo test --features integration-tests
```

**Integration Tests Overview:**
- Verify end-to-end repository inheritance pipeline
- Test real GitHub repository cloning and caching
- Validate performance improvements from caching
- Confirm filesystem operations work correctly
- Disabled by default since they require network access

**Test Coverage:**
- **Comprehensive test suite** including unit tests, E2E CLI tests, integration tests, and doc tests
- **Integration tests** validating end-to-end workflows (feature-gated)
- **Datatest tests** for schema parsing automatically discover test cases from YAML files
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
├── .github/workflows/     # GitHub Actions CI/CD workflows
├── src/
│   ├── main.rs            # CLI entry point
│   ├── lib.rs             # Library entry point
│   ├── cli.rs             # CLI argument parsing
│   ├── commands/          # CLI command implementations
│   │   ├── apply.rs       # Apply configuration
│   │   ├── check.rs       # Validate and check for updates
│   │   ├── diff.rs        # Preview changes
│   │   ├── init.rs        # Initialize new config
│   │   └── ...            # Other commands
│   ├── phases/            # 6-phase processing pipeline
│   ├── merge/             # Format-specific merge logic (YAML, JSON, etc.)
│   ├── config.rs          # Configuration parsing
│   ├── operators.rs       # File operation implementations
│   └── ...                # Other modules
├── tests/                 # Unit, integration, and E2E tests
├── docs/                  # Design documentation
└── context/               # Development context and planning
```

## Usage

### CLI Commands

```bash
# Initialize a new .common-repo.yaml
common-repo init

# Apply configuration (runs full 6-phase pipeline)
common-repo apply

# Preview what would change
common-repo apply --dry-run
common-repo diff

# Check for available updates
common-repo check --updates

# Update refs to newer versions
common-repo update

# Other commands
common-repo validate    # Validate configuration
common-repo info        # Show configuration overview
common-repo ls          # List files that would be created
common-repo tree        # Show inheritance tree
common-repo cache       # Manage repository cache
```

### Example Configuration

```yaml
# .common-repo.yaml
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v1.0.0
    with:
      - include: [".*", "**/*"]
      - exclude: [".git/**"]

- template:
    - "**/*.template"

- yaml:
    source: ci-fragment.yml
    dest: .github/workflows/ci.yml
    path: jobs.test
    append: true
```

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
