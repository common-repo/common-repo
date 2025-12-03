# CI/CD Best Practices Research

Research compiled from OSS best practices (2024-2025) and analysis of exemplar Rust projects.

## Reference Projects Analyzed

1. **uv** (astral-sh/uv) - Python package manager written in Rust
2. **ruff** (astral-sh/ruff) - Python linter/formatter written in Rust
3. **ripgrep** (BurntSushi/ripgrep) - Line-oriented search tool
4. **deno** (denoland/deno) - JavaScript/TypeScript runtime

## Workflow Architecture

### Tiered Job Structure

Modern CI workflows use hierarchical job dependencies:

```yaml
jobs:
  # Tier 1: Gate jobs (fast, filter unnecessary work)
  determine_changes:
    outputs:
      parser: ${{ steps.changes.outputs.parser }}

  # Tier 2: Validation (lint, format, clippy)
  lint:
    needs: determine_changes
    if: needs.determine_changes.outputs.code == 'true'

  # Tier 3: Build and test
  test:
    needs: lint

  # Tier 4: Integration/system tests
  integration:
    needs: test
```

**Key Pattern**: Gate jobs analyze git diffs to conditionally skip downstream work:

```yaml
- uses: dorny/paths-filter@v3
  id: changes
  with:
    filters: |
      code:
        - 'src/**'
        - 'Cargo.toml'
        - 'Cargo.lock'
```

### Concurrency Management

Prevent duplicate runs and wasted resources:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref_name }}-${{ github.event.pull_request.number || github.sha }}
  cancel-in-progress: true
```

This cancels in-progress jobs when new commits arrive on the same PR or branch.

### Workflow Environment Defaults

Set consistent defaults across all jobs:

```yaml
defaults:
  run:
    shell: bash

env:
  CARGO_INCREMENTAL: 0      # Disable incremental compilation (faster CI)
  CARGO_NET_RETRY: 10       # Retry failed downloads
  RUSTFLAGS: -D warnings    # Treat warnings as errors
```

## Caching Strategies

### Swatinem/rust-cache (Recommended)

Smart caching with sensible defaults:

```yaml
- uses: actions/checkout@v5
- run: rustup toolchain install stable --profile minimal
- uses: Swatinem/rust-cache@v2
  with:
    # Only save cache on main branch (reduces storage costs)
    save-if: ${{ github.ref == 'refs/heads/main' }}
    # Share cache across similar jobs
    shared-key: ${{ runner.os }}-cargo
```

**Best Practices**:
- Install toolchain before cache action (cache key includes rustc version)
- Use `save-if` to only persist cache from main branch
- Use `shared-key` for cross-job cache sharing
- Commit `Cargo.lock` for reproducible caching

**What rust-cache cleans automatically**:
- Unused dependencies
- Non-dependency artifacts
- Incremental build artifacts
- Artifacts older than one week

### Alternative: Manual Cache Setup

For more control, use `actions/cache` directly:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

### sccache for Distributed Caching

For large projects, sccache enables concurrent cache fetching:

```yaml
- uses: mozilla-actions/sccache-action@v0.0.7
- name: Build
  env:
    SCCACHE_GHA_ENABLED: "true"
    RUSTC_WRAPPER: "sccache"
  run: cargo build --release
```

**Advantage**: Build can start immediately while cache downloads in parallel.

## Matrix Build Strategies

### Platform Coverage Matrix

Comprehensive cross-platform testing:

```yaml
strategy:
  fail-fast: false  # Don't cancel other jobs if one fails
  matrix:
    include:
      # Linux
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: ubuntu-latest
        target: x86_64-unknown-linux-musl
      - os: ubuntu-latest
        target: aarch64-unknown-linux-gnu

      # macOS
      - os: macos-latest
        target: x86_64-apple-darwin
      - os: macos-14  # Apple Silicon
        target: aarch64-apple-darwin

      # Windows
      - os: windows-latest
        target: x86_64-pc-windows-msvc
```

### Rust Version Matrix

Test across Rust versions for compatibility:

```yaml
strategy:
  matrix:
    rust:
      - 1.75.0          # MSRV (Minimum Supported Rust Version)
      - stable
      - beta
      - nightly
```

### Conditional Runners

Use different runners based on context:

```yaml
runs-on: ${{ github.repository == 'org/repo' && 'custom-runner' || 'ubuntu-latest' }}
```

## Job Types

### Lint Job

Fast checks that run first:

```yaml
lint:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - uses: Swatinem/rust-cache@v2

    - name: Check formatting
      run: cargo fmt --all --check

    - name: Clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
```

### Test Job

Comprehensive testing with nextest:

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - uses: taiki-e/install-action@nextest

    - name: Run tests
      run: cargo nextest run --profile ci

    - name: Run doctests
      run: cargo test --doc
```

### MSRV Verification

Ensure minimum supported Rust version:

```yaml
msrv:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - name: Get MSRV
      id: msrv
      run: echo "version=$(grep rust-version Cargo.toml | cut -d'"' -f2)" >> $GITHUB_OUTPUT
    - uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ steps.msrv.outputs.version }}
    - run: cargo check --all-features
```

### Cross-Compilation Job

Build for non-native targets:

```yaml
cross-build:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      target:
        - aarch64-unknown-linux-gnu
        - armv7-unknown-linux-gnueabihf
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}
    - uses: taiki-e/install-action@cross

    - name: Build
      run: cross build --target ${{ matrix.target }} --release
```

## Security Scanning

### cargo-audit

Scan for known vulnerabilities:

```yaml
security:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - uses: rustsec/audit-check@v2
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
```

### cargo-deny

Comprehensive dependency checking:

```yaml
- uses: EmbarkStudios/cargo-deny-action@v1
  with:
    command: check
    arguments: --all-features
```

**Checks performed**:
- Security advisories (RustSec database)
- License compatibility
- Banned crates/sources
- Duplicate dependencies

### Dependabot Integration

Enable in `.github/dependabot.yml`:

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    groups:
      dependencies:
        patterns:
          - "*"
```

### Dependency Review Action

Block PRs that introduce vulnerabilities:

```yaml
dependency-review:
  runs-on: ubuntu-latest
  if: github.event_name == 'pull_request'
  steps:
    - uses: actions/checkout@v5
    - uses: actions/dependency-review-action@v4
```

## Release Automation

### cargo-dist (Recommended)

Modern release automation with broad platform support:

```bash
cargo install cargo-dist
cargo dist init
```

**Features**:
- Multi-platform binary builds
- Installer scripts (shell, PowerShell)
- Homebrew formula generation
- Checksums and attestations
- Changelog integration

**Configuration** (`Cargo.toml`):

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.27.0"
ci = "github"
installers = ["shell", "powershell", "homebrew"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
pr-run-mode = "upload"
```

### cargo-release

Local release workflow automation:

```bash
cargo install cargo-release
cargo release patch --execute  # Bump, tag, publish
```

**Features**:
- Version bumping
- Git tagging
- Changelog updates
- crates.io publishing
- Dry-run by default

### Release-plz + cargo-dist

Automated release PRs with dist:

```yaml
# .github/workflows/release-plz.yml
release-plz:
  runs-on: ubuntu-latest
  steps:
    - uses: release-plz/action@v0.5
      with:
        config: release-plz.toml
```

**Configuration** (`release-plz.toml`):

```toml
[workspace]
git_release_enable = false  # Let cargo-dist handle releases
changelog_config = "cliff.toml"
```

### Manual Release Workflow Pattern

Trigger-based releases (ripgrep pattern):

```yaml
on:
  push:
    tags:
      - "[0-9]+.[0-9]+.[0-9]+"

jobs:
  validate:
    steps:
      - name: Validate version
        run: |
          VERSION=${GITHUB_REF#refs/tags/}
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
          if [ "$VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version ($VERSION) != Cargo.toml version ($CARGO_VERSION)"
            exit 1
          fi
```

### Artifact Building

Cross-platform binary builds:

```yaml
build-binaries:
  strategy:
    matrix:
      include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
          archive: tar.gz
        - os: macos-latest
          target: x86_64-apple-darwin
          archive: tar.gz
        - os: windows-latest
          target: x86_64-pc-windows-msvc
          archive: zip

  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}

    - name: Build
      run: cargo build --release --target ${{ matrix.target }}

    - name: Package (Unix)
      if: matrix.archive == 'tar.gz'
      run: |
        tar -czvf myapp-${{ matrix.target }}.tar.gz \
          -C target/${{ matrix.target }}/release myapp

    - name: Upload
      uses: actions/upload-artifact@v4
      with:
        name: myapp-${{ matrix.target }}
        path: myapp-${{ matrix.target }}.*
```

### Publishing to crates.io

Secure publishing with OIDC:

```yaml
publish:
  runs-on: ubuntu-latest
  permissions:
    id-token: write
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable

    - name: Publish
      run: cargo publish
      env:
        CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

## Workflow File Organization

### Single vs. Multiple Workflow Files

**Single file** (recommended for smaller projects):
- `.github/workflows/ci.yml` - All checks and tests

**Multiple files** (larger projects like uv, ruff):
- `.github/workflows/ci.yml` - Core CI (lint, test)
- `.github/workflows/release.yml` - Release automation
- `.github/workflows/docs.yml` - Documentation builds
- `.github/workflows/security.yml` - Security scanning

### Reusable Workflows

Share common logic across repositories:

```yaml
# .github/workflows/reusable-test.yml
on:
  workflow_call:
    inputs:
      rust-version:
        type: string
        default: stable

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ inputs.rust-version }}
      - run: cargo test
```

Usage:

```yaml
jobs:
  test:
    uses: ./.github/workflows/reusable-test.yml
    with:
      rust-version: "1.75.0"
```

## Exemplar Project Patterns

### uv

- **Tiered architecture**: Gate jobs filter work based on path changes
- **Ecosystem testing**: Tests against real Python projects (Flask, Pydantic)
- **Registry testing**: Validates against multiple package indexes
- **Smoke tests**: Quick validation per platform
- **Trusted publishing**: PyPI via OIDC tokens

### ruff

- **Change detection**: Conditional jobs based on component changes
- **WASM builds**: WebAssembly target for playground
- **Ecosystem comparison**: Compares baseline vs. candidate outputs
- **Fuzzing**: Parser fuzzing with diff detection
- **MSRV enforcement**: Verifies minimum Rust version

### ripgrep

- **Extensive matrix**: 16+ platform configurations
- **Cross-compilation**: Uses `cross` tool for ARM, PowerPC, etc.
- **Static linking**: PCRE2 statically linked for portability
- **Debian packaging**: Dedicated job for `.deb` creation
- **Shell completions**: Generated for Bash, Zsh, Fish, PowerShell

### deno

- **Draft PR handling**: Skip expensive builds for draft PRs
- **Multi-level caching**: Cargo home + build output caching
- **Code signing**: macOS (rcodesign) and Windows (Azure signing)
- **Canary releases**: Automated uploads to cloud storage
- **Web Platform Tests**: WPT suite for compatibility

## Security Best Practices

### Minimal Permissions

Explicitly declare permissions:

```yaml
permissions:
  contents: read  # All others default to none
```

For releases:

```yaml
permissions:
  contents: write     # Create releases
  id-token: write     # OIDC tokens
  attestations: write # Provenance
```

### Secret Management

- Use GitHub secrets for tokens
- Prefer OIDC over long-lived tokens
- Never echo secrets in logs

### Supply Chain Security

- Pin action versions with SHA
- Use Dependabot for action updates
- Enable dependency review for PRs
- Generate SBOMs with `cargo sbom`

## Performance Optimization

### Build Speed

1. **Disable incremental builds**: `CARGO_INCREMENTAL=0`
2. **Use sccache**: Distributed compilation cache
3. **Split jobs**: Parallel lint/test/build
4. **Gate expensive jobs**: Skip based on path changes
5. **Use faster runners**: Self-hosted or paid runners

### Cache Efficiency

1. **Prune cache**: rust-cache auto-cleans old artifacts
2. **Shared keys**: Reuse cache across similar jobs
3. **Save only on main**: Reduce cache storage
4. **Commit Cargo.lock**: Reproducible builds

### Job Parallelization

1. **Independent jobs**: Run lint, test, docs in parallel
2. **Matrix builds**: Parallelize across platforms
3. **Test sharding**: Split tests with nextest partitioning

## Key Takeaways

### Workflow Structure

1. Use tiered job architecture with gate jobs
2. Enable concurrency cancellation for PRs
3. Set consistent environment defaults
4. Organize workflows by concern for larger projects

### Caching

1. Use `Swatinem/rust-cache` with `save-if` optimization
2. Install toolchain before cache action
3. Commit `Cargo.lock` for reproducible caching
4. Consider sccache for large projects

### Testing

1. Use cargo-nextest for faster CI execution
2. Run doctests separately (`cargo test --doc`)
3. Gate tests based on path changes
4. Test across multiple Rust versions

### Security

1. Run cargo-audit and cargo-deny in CI
2. Enable Dependabot for Cargo dependencies
3. Use dependency review action for PRs
4. Declare minimal permissions

### Releases

1. Use cargo-dist for multi-platform releases
2. Combine with release-plz for automation
3. Generate checksums for all artifacts
4. Use OIDC for secure publishing

## Sources

### GitHub Actions
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain)
- [taiki-e/install-action](https://github.com/taiki-e/install-action)

### Best Practices
- [GitHub Actions best practices for Rust projects](https://www.infinyon.com/blog/2021/04/github-actions-best-practices/)
- [Cargo Book - Continuous Integration](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [actions-rs/meta](https://github.com/actions-rs/meta) - Workflow recipes

### Caching
- [Fast Rust Builds with sccache](https://depot.dev/blog/sccache-in-github-actions)
- [Optimizing Rust Builds for GitHub Actions](https://www.uffizzi.com/blog/optimizing-rust-builds-for-faster-github-actions-pipelines)

### Release Automation
- [cargo-dist](https://opensource.axo.dev/cargo-dist/)
- [cargo-release](https://github.com/crate-ci/cargo-release)
- [Fully Automated Releases for Rust Projects](https://blog.orhun.dev/automated-rust-releases/)
- [release-plz](https://release-plz.dev/)

### Security
- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [actions-rust-lang/audit](https://github.com/actions-rust-lang/audit)
- [GitHub Supply Chain Security for Rust](https://github.blog/2022-06-06-github-brings-supply-chain-security-features-to-the-rust-community/)

### Reference Projects
- [uv GitHub](https://github.com/astral-sh/uv)
- [ruff GitHub](https://github.com/astral-sh/ruff)
- [ripgrep GitHub](https://github.com/BurntSushi/ripgrep)
- [deno GitHub](https://github.com/denoland/deno)
