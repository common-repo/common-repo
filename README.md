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

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

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
