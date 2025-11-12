# common-repo

A Rust project with modern development tooling and automation.

## Features

- **Automated Testing & CI/CD**: GitHub Actions workflows for continuous integration
- **Code Quality**: Pre-commit hooks with rustfmt and clippy
- **Conventional Commits**: Enforced commit message standards
- **Semantic Versioning**: Automated version management and releases

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
- **93 unit tests** covering all core functionality
- **5 integration tests** validating end-to-end workflows
- **14 doctests** providing executable examples in documentation
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
│   └── workflows/       # GitHub Actions workflows
├── src/
│   └── main.rs          # Main application entry point
├── Cargo.toml           # Rust package manifest
├── .pre-commit-config.yaml  # Pre-commit hooks configuration
├── .commitlintrc.yml    # Commit message linting rules
└── README.md            # This file
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
