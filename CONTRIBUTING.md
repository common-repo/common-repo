# Contributing to common-repo

Thank you for your interest in contributing to common-repo! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Getting Started

### Prerequisites

- Rust stable channel (automatically managed via `rust-toolchain.toml`)
- Python 3.7+ (for pre-commit hooks)
- Git
- Familiarity with Rust and Git workflows

### Setting Up Your Development Environment

This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern:

```bash
git clone https://github.com/YOUR-USERNAME/common-repo.git
cd common-repo
./script/setup    # First-time setup (installs deps, configures hooks, builds)
./script/test     # Run test suite
```

**Available scripts:**
- `./script/bootstrap` — Install dependencies (cargo-nextest, pre-commit)
- `./script/setup` — First-time setup (runs bootstrap + configures environment)
- `./script/update` — Update after pulling changes
- `./script/test` — Run tests (uses cargo-nextest)
- `./script/cibuild` — Run full CI checks locally

### cargo xtask

This project uses [cargo xtask](https://github.com/matklad/cargo-xtask) for complex development automation tasks. Available commands:

```bash
cargo xtask coverage       # Run test coverage with cargo-tarpaulin
cargo xtask release-prep   # Prepare a new release
```

**Coverage options:**
```bash
cargo xtask coverage                     # HTML report (default)
cargo xtask coverage --format json       # JSON report
cargo xtask coverage --fail-under 80     # Fail if coverage < 80%
cargo xtask coverage --open              # Open report in browser
```

**Release preparation:**
```bash
cargo xtask release-prep --dry-run       # Preview changes without applying
cargo xtask release-prep                 # Bump patch version (0.22.0 -> 0.22.1)
cargo xtask release-prep --version 1.0.0 # Set specific version
```

## Development Workflow

### Making Changes

1. **Create a new branch** for your work
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

2. **Make your changes** following our coding standards (see below)

3. **Write or update tests** for your changes
   - Unit tests should be co-located with the code they test
   - Integration tests go in the `tests/` directory
   - Aim for high test coverage (80%+ target)

4. **Run tests locally**
   ```bash
   # Run all tests
   cargo nextest run

   # Run with integration tests
   cargo nextest run --features integration-tests

   # Check for slow tests
   cargo nextest run --profile ci
   ```

5. **Ensure code quality**
   ```bash
   # Format code
   cargo fmt

   # Check for linting issues
   cargo clippy --all-targets --all-features -- -D warnings

   # Run all quality checks
   cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
   ```

### Commit Guidelines

This project uses [Conventional Commits](https://www.conventionalcommits.org/). All commit messages must follow this format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

#### Commit Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, missing semicolons, etc.)
- `refactor`: Code refactoring without changing functionality
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Changes to build system or dependencies
- `ci`: Changes to CI configuration
- `chore`: Other changes that don't modify src or test files
- `revert`: Reverting a previous commit

#### Examples

```
feat(cache): add LRU eviction policy

fix(git): handle authentication errors gracefully

docs: update installation instructions

test(phases): add integration test for deep repo chains
```

#### Breaking Changes

Breaking changes should be indicated with either:
- An exclamation mark: `feat!: change API signature`
- A `BREAKING CHANGE:` footer in the commit message

### Submitting a Pull Request

1. **Push your branch** to your fork
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub
   - Use a clear, descriptive title
   - Reference any related issues (e.g., "Fixes #123")
   - Describe what changes you made and why
   - Include testing notes if applicable

3. **Address review feedback**
   - Make requested changes in new commits
   - Push updates to your branch
   - Respond to reviewer comments

4. **Wait for CI to pass**
   - All tests must pass
   - Code must be formatted correctly
   - Clippy must report no warnings
   - Commit messages must follow conventional commits

5. **Merging**
   - Maintainers will add the `ready to merge` label when approved
   - This enables auto-merge: the PR will automatically rebase and merge once all required checks pass
   - Branch is deleted after merge, and you'll be notified

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Address all `cargo clippy` warnings (treated as errors in CI)
- Write clear, descriptive comments for public APIs
- Use meaningful variable and function names

### Documentation

- Add rustdoc comments for public APIs
- Include examples in documentation where helpful
- Update relevant documentation files when making changes
- Keep README.md up to date with new features

#### Table of Contents Generation

This project uses [mktoc](https://github.com/kevingimbel/mktoc) to generate and maintain tables of contents in markdown files. To regenerate a ToC:

```bash
# Install mktoc (first time only)
cargo install mktoc

# Regenerate ToC for a specific file
mktoc docs/src/configuration.md

# Preview ToC without modifying the file
mktoc --stdout docs/src/configuration.md
```

ToC markers in markdown files look like:
```markdown
<!-- BEGIN mktoc {"min_depth": 2, "max_depth": 3} -->
... generated content ...
<!-- END mktoc -->
```

The inline JSON configuration controls which heading levels are included.

### Testing

- Write unit tests for all new functions
- Add integration tests for end-to-end workflows
- Test error cases and edge conditions
- Aim for 80%+ code coverage
- Use descriptive test names: `test_function_does_something_when_condition()`

#### Snapshot Testing

This project uses [insta](https://insta.rs) for snapshot testing of CLI output. Snapshot tests capture CLI help text and error messages to detect unintended changes.

**Running snapshot tests:**
```bash
cargo test --test cli_snapshot_tests
```

**Updating snapshots after intentional changes:**
```bash
# Review and accept pending snapshots interactively
cargo insta review

# Or accept all pending snapshots at once
cargo insta accept
```

**Adding new snapshot tests:**
1. Add a new test function in `tests/cli_snapshot_tests.rs`
2. Use `insta::assert_snapshot!("name", output)` to capture output
3. Run the test to generate a `.snap.new` file
4. Review and accept the snapshot
5. Commit both the test and the `.snap` file

Snapshot files are stored in `tests/snapshots/` and should be committed to version control.

### Performance

- Avoid unnecessary allocations
- Use appropriate data structures
- Profile performance-critical code
- Document performance characteristics of public APIs

## Project Structure

```
common-repo/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # CLI binary entry point
│   ├── cache.rs            # In-memory caching
│   ├── config.rs           # Configuration parsing
│   ├── error.rs            # Error types
│   ├── filesystem.rs       # In-memory filesystem
│   ├── git.rs              # Git operations
│   ├── operators.rs        # File operations
│   ├── path.rs             # Path utilities
│   ├── phases.rs           # 6-phase pipeline
│   ├── repository.rs       # Repository management
│   ├── version.rs          # Version detection
│   └── commands/           # CLI commands
├── xtask/                  # Development automation (cargo xtask)
├── tests/                  # Integration tests
├── docs/                   # Documentation
├── examples/               # Example configurations
└── .github/                # CI/CD workflows
```

## Areas for Contribution

We welcome contributions in these areas:

- **Bug fixes**: Fix reported issues
- **Features**: Implement features from the roadmap
- **Tests**: Improve test coverage
- **Documentation**: Improve docs, examples, and guides
- **Performance**: Optimize slow operations
- **Tooling**: Improve developer experience

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bug reports**: Open a GitHub Issue with reproduction steps
- **Feature requests**: Open a GitHub Issue with use case description
- **Security issues**: See SECURITY.md (if it exists) or email maintainers directly

## License

By contributing to common-repo, you agree that your contributions will be licensed under the same license as the project (see LICENSE file).

## Recognition

Contributors will be recognized in the project's release notes and GitHub contributor list.

Thank you for contributing to common-repo!
