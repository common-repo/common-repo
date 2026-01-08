# Testing Guidelines Research

Research compiled from OSS recommendations (2024-2025) and analysis of exemplar Rust projects.

## Reference Projects Analyzed

1. **uv** (astral-sh/uv) - Python package manager written in Rust
2. **ripgrep** (BurntSushi/ripgrep) - Line-oriented search tool
3. **serde** (serde-rs/serde) - Serialization library
4. **axum** (tokio-rs/axum) - Web server library built on Tokio/Tower

## Test Organization

### Rust's Two-Category Model

Rust distinguishes between two primary test types:

1. **Unit Tests**: Small, focused tests in the same file as the code they test
   - Use `#[cfg(test)]` to exclude from production builds
   - Can test private functions
   - Fast to run

2. **Integration Tests**: External tests using only the public API
   - Located in `tests/` directory at project root
   - Each file compiles as a separate crate
   - Exercise multiple modules together

### Directory Structure Pattern

```
project/
├── src/
│   ├── lib.rs          # Unit tests inline with #[cfg(test)]
│   └── module.rs       # Module-specific unit tests
├── tests/
│   ├── integration.rs  # Integration test file
│   ├── common/         # Shared test utilities
│   │   └── mod.rs      # Helpers available to all tests
│   └── fixtures/       # Test data files
└── benches/            # Benchmarks (optional)
```

### Test Helpers for Integration Tests

When sharing code across integration tests, create a helper module:

```rust
// tests/common/mod.rs
pub fn setup_test_environment() -> TestContext {
    // Shared setup logic
}
```

Then in each test file:
```rust
mod common;
use common::setup_test_environment;
```

**Best Practice**: For large projects, create a local unpublished crate for test utilities and import as a dev dependency.

## Test Runners

### cargo test (Built-in)

Standard test runner included with Rust:
- Runs unit tests, integration tests, and doctests
- Parallel execution within each test binary
- Supports filtering: `cargo test test_name`

### cargo-nextest (Recommended for CI)

New test runner with modern execution model:

| Feature | cargo test | cargo-nextest |
|---------|-----------|---------------|
| Execution | Per-binary parallel | Per-test parallel |
| Speed | Baseline | Up to 3x faster |
| Output | Basic | Detailed with timing |
| Retries | Manual | Built-in flaky test retry |
| Partitioning | None | CI sharding support |
| Slow tests | None | Detection/timeout |

**Limitations**:
- Doctests not supported (run separately with `cargo test --doc`)
- Each test runs in a separate process (no shared memory)

**Installation & Usage**:
```bash
cargo install cargo-nextest --locked
cargo nextest run
cargo nextest run --profile ci       # Use CI profile
cargo nextest run --partition hash:1/3  # Sharding
```

**Configuration** (`.config/nextest.toml`):
```toml
[profile.ci]
retries = 2
slow-timeout = { period = "60s", terminate-after = 3 }
fail-fast = false
```

## Code Coverage

### cargo-llvm-cov (Recommended)

Uses LLVM source-based instrumentation for precise coverage:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov                    # Summary
cargo llvm-cov --html             # HTML report
cargo llvm-cov --lcov --output-path lcov.info  # For CI
```

**Pros**: Fast, accurate, cross-platform, multiple output formats
**Cons**: Branch coverage requires nightly, doctests need nightly

### cargo-tarpaulin

Alternative coverage tool, Linux-focused:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
cargo tarpaulin --fail-under 80   # CI enforcement
```

**Pros**: Simpler setup, integrates with coverage services
**Cons**: Best on Linux, relies on ptrace instrumentation

### Coverage Guidelines

- **Target meaningful coverage**: 80% is a common threshold, but quality matters more than quantity
- **Exclude generated code**: Use `#[cfg_attr(coverage_nightly, coverage(off))]`
- **Merge multiple test runs**: Combine unit, integration, and feature-flagged tests
- **CI enforcement**: Fail builds below threshold, but avoid gaming metrics

## Testing Libraries

### Fixture Testing: rstest

Dependency injection and parameterized tests:

```rust
use rstest::*;

#[fixture]
fn database() -> Database {
    Database::new_test()
}

#[rstest]
fn test_query(database: Database) {
    assert!(database.query("SELECT 1").is_ok());
}

// Parameterized tests
#[rstest]
#[case(0, 0)]
#[case(1, 1)]
#[case(2, 1)]
fn test_fibonacci(#[case] input: u32, #[case] expected: u32) {
    assert_eq!(fib(input), expected);
}
```

**Features**:
- Fixtures with `#[fixture]`
- Parameterized tests with `#[case]`
- Matrix testing with `#[values]`
- Async support with `#[future]`
- Once fixtures with `#[once]`

### Snapshot Testing: insta

Capture and compare output snapshots:

```rust
use insta::assert_snapshot;

#[test]
fn test_output() {
    let output = generate_report();
    assert_snapshot!(output);
}

// For serializable data
use insta::assert_yaml_snapshot;

#[test]
fn test_data() {
    let data = fetch_data();
    assert_yaml_snapshot!(data);
}
```

**Workflow**:
```bash
cargo insta test --review  # Run tests, review new snapshots
cargo insta review         # Interactive review
```

**Features**:
- Inline or file-based snapshots
- Redactions for unstable values (timestamps, IDs)
- Multiple formats: YAML, JSON, TOML, debug, display

### Property-Based Testing

#### proptest (Recommended)

Hypothesis-like library with strategies:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parse_roundtrip(s in "[a-zA-Z0-9]+") {
        let parsed = parse(&s).unwrap();
        assert_eq!(serialize(&parsed), s);
    }
}
```

**Advantages over quickcheck**:
- Per-value strategies (not per-type)
- Constraint-aware generation
- Better shrinking

#### quickcheck

Simpler API, type-based generation:

```rust
use quickcheck_macros::quickcheck;

#[quickcheck]
fn prop_reverse_reverse(xs: Vec<i32>) -> bool {
    xs.iter().rev().rev().eq(xs.iter())
}
```

### Serialization Testing: serde_test

Test Serialize/Deserialize implementations:

```rust
use serde_test::{assert_tokens, Token};

#[test]
fn test_serialization() {
    let value = MyStruct { field: 42 };

    assert_tokens(&value, &[
        Token::Struct { name: "MyStruct", len: 1 },
        Token::Str("field"),
        Token::I32(42),
        Token::StructEnd,
    ]);
}
```

## Async Testing

### tokio::test

Standard async test attribute:

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

**Time simulation** (with `test-util` feature):
```rust
#[tokio::test(start_paused = true)]
async fn test_timeout() {
    let start = Instant::now();
    tokio::time::sleep(Duration::from_secs(60)).await;
    // Time advances instantly in paused mode
    assert!(start.elapsed() >= Duration::from_secs(60));
}
```

### Loom: Concurrency Testing

Permutation testing for concurrent code:

```rust
use loom::sync::Arc;
use loom::thread;

#[test]
fn test_concurrent_access() {
    loom::model(|| {
        let data = Arc::new(AtomicUsize::new(0));
        // Loom explores all possible interleavings
    });
}
```

### Turmoil: Network Simulation

Simulated networking for distributed systems testing. See the [Turmoil repository](https://github.com/tokio-rs/turmoil) for details.

## Web Application Testing (Axum Pattern)

### Tower Service Testing

Test routers without starting HTTP server:

```rust
use axum::{Router, body::Body};
use tower::ServiceExt;
use http::Request;

#[tokio::test]
async fn test_handler() {
    let app = create_router();

    let response = app
        .oneshot(Request::builder()
            .uri("/api/users")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

### App Factory Pattern

Extract router creation for testability:

```rust
// src/lib.rs
pub fn create_app() -> Router {
    Router::new()
        .route("/", get(handler))
}

// tests/integration.rs
#[tokio::test]
async fn test_app() {
    let app = create_app();
    // Test the app
}
```

## CI Integration Patterns

### GitHub Actions Example

```yaml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Install nextest
        uses: taiki-e/install-action@nextest

      - name: Run tests
        run: cargo nextest run --profile ci

      - name: Run doctests
        run: cargo test --doc

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
```

### Test Caching

Preserve compilation artifacts between runs:
- Use `Swatinem/rust-cache` action
- Consider `CARGO_INCREMENTAL=0` for CI (reduces cache size)
- Run `uv cache prune --ci` pattern for package managers

### Test Partitioning

Split tests across multiple CI jobs:

```yaml
jobs:
  test:
    strategy:
      matrix:
        partition: [1, 2, 3]
    steps:
      - run: cargo nextest run --partition hash:${{ matrix.partition }}/3
```

## Exemplar Project Patterns

### ripgrep

- **Organization**: Workspace with multiple crates, each with unit tests
- **Integration tests**: `tests/` directory with full CLI tests
- **Regression tests**: Dedicated `tests/regression.rs` for bug fixes
- **CI scripts**: `ci/script.sh` for test orchestration

### serde

- **Test suite crate**: Separate `test_suite/` for integration testing
- **serde_test**: Published crate for testing implementations
- **Generated code tests**: Verify derive macros compile correctly
- **Token-based assertions**: Precise serialization verification

### axum

- **Examples as tests**: `examples/testing/` demonstrates patterns
- **Test helpers**: `axum/src/test_helpers/` for internal use
- **Tower integration**: Tests use `ServiceExt::oneshot` pattern
- **WebSocket testing**: Dedicated examples for async testing

### uv

- **Cache testing**: Intentional invalid data injection for robustness
- **Integration tests**: Feature-gated behind flags
- **CI matrix**: Multiple Python versions across platforms

## Summary

### Test Organization

1. Unit tests in same file with `#[cfg(test)]`
2. Integration tests in `tests/` directory
3. Shared helpers in `tests/common/` module
4. Consider separate test utility crate for large projects

### Test Tooling

1. Use `cargo-nextest` for faster CI execution
2. Combine with `cargo test --doc` for doctests
3. Consider `rstest` for fixtures and parameterization
4. Use `insta` for output-heavy assertions

### Coverage Strategy

1. Target meaningful coverage (80% is reasonable)
2. Use `cargo-llvm-cov` for accurate instrumentation
3. Integrate with CI coverage services (Codecov, Coveralls)
4. Focus on testing behavior, not just lines

### Advanced Testing

1. Property-based testing with `proptest` for edge cases
2. Snapshot testing with `insta` for complex outputs
3. Async testing with `tokio::test` and time simulation
4. Concurrency testing with `loom` for low-level code

## Sources

### General Rust Testing
- [The Rust Book - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [Shuttle - Everything you need to know about testing in Rust](https://www.shuttle.dev/blog/2024/03/21/testing-in-rust)
- [LogRocket - How to organize Rust tests](https://blog.logrocket.com/how-to-organize-rust-tests/)
- [Rust Project Primer - Test runners](https://rustprojectprimer.com/testing/runners.html)

### Test Runners
- [cargo-nextest](https://nexte.st/)
- [cargo-nextest GitHub](https://github.com/nextest-rs/nextest)

### Coverage Tools
- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [cargo-tarpaulin GitHub](https://github.com/xd009642/tarpaulin)
- [Rust Project Primer - Coverage](https://rustprojectprimer.com/measure/coverage.html)

### Testing Libraries
- [rstest GitHub](https://github.com/la10736/rstest)
- [insta GitHub](https://github.com/mitsuhiko/insta) | [insta.rs](https://insta.rs/)
- [proptest GitHub](https://github.com/proptest-rs/proptest)
- [quickcheck GitHub](https://github.com/BurntSushi/quickcheck)
- [serde_test Documentation](https://serde.rs/unit-testing.html)

### Async Testing
- [Tokio - Unit Testing](https://tokio.rs/tokio/topics/testing)
- [loom GitHub](https://github.com/tokio-rs/loom)

### Reference Projects
- [ripgrep GitHub](https://github.com/BurntSushi/ripgrep)
- [serde GitHub](https://github.com/serde-rs/serde)
- [axum GitHub](https://github.com/tokio-rs/axum)
- [uv GitHub](https://github.com/astral-sh/uv)
