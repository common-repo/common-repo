# Open Source Rust Project Best Practices Reference

Consolidated reference synthesized from research on documentation, testing, CI/CD, architecture, CLI/UX, security, distribution, community, and performance best practices. Based on analysis of leading Rust projects including uv, ruff, ripgrep, tokio, starship, and clap.

## Quick Reference: Exemplar Projects by Area

| Area | Primary Exemplars | Notable For |
|------|-------------------|-------------|
| Documentation | uv, ruff | mkdocs-material sites, README structure |
| Testing | ripgrep, serde | Comprehensive test organization, fixtures |
| CI/CD | uv, ruff, ripgrep | Tiered workflows, matrix builds, cargo-dist |
| Architecture | tokio, clap | Workspace organization, crate separation |
| CLI/UX | ripgrep, gh, starship | Error messages, progress, color handling |
| Security | tokio, uv | SECURITY.md, cargo-audit integration |
| Distribution | uv, ruff, starship | Multi-channel, shell installers |
| Community | Rust, Tokio | Governance, CONTRIBUTING.md, templates |
| Performance | ripgrep, uv, tokio | Benchmarking, profiling, optimization |

---

## 1. Documentation

### Key Takeaways

1. **README structure**: Logo/badges → tagline → key features → installation → quick start → docs links → contributing → license
2. **Multi-method installation**: CLI tools should offer 5+ installation methods (shell installer, cargo, homebrew, etc.)
3. **Rustdoc essentials**: Use `///` with one-line summary, detailed explanation, examples, and standard sections (Errors, Panics, Safety)
4. **Doctests**: Keep examples tested via doctests; use `#![doc = include_str!("../README.md")]` to sync README and crate docs
5. **External docs**: Complex tools benefit from mkdocs-material or mdBook sites; simple utilities can use in-repo markdown

### Badges to Include

- CI/build status
- Version (crates.io)
- License
- Documentation link
- Community (Discord/Discussions)

### Documentation Types

| Type | Purpose | Example |
|------|---------|---------|
| Getting Started | Onboard new users | Quick 5-minute tutorial |
| User Guide | Comprehensive features | Full documentation site |
| API Reference | Auto-generated | rustdoc |
| FAQ | Common questions | In-repo markdown |
| Contributing | Development setup | CONTRIBUTING.md |
| Changelog | Release history | CHANGELOG.md |

---

## 2. Testing

### Key Takeaways

1. **Two-tier model**: Unit tests inline with `#[cfg(test)]`, integration tests in `tests/` directory
2. **Use cargo-nextest**: Up to 3x faster than cargo test, better CI integration, flaky test retry
3. **Run doctests separately**: `cargo nextest run` + `cargo test --doc`
4. **Target 80% coverage**: Use cargo-llvm-cov or cargo-tarpaulin; focus on behavior, not just lines
5. **Test helpers**: Put shared utilities in `tests/common/mod.rs` or dedicated dev-dependency crate

### Recommended Libraries

| Library | Purpose | When to Use |
|---------|---------|-------------|
| rstest | Fixtures, parameterized tests | Complex setup, many test cases |
| insta | Snapshot testing | Output-heavy assertions |
| proptest | Property-based testing | Edge case discovery |
| serde_test | Serialization testing | Testing Serialize/Deserialize |
| tokio::test | Async testing | Async code |

### CI Test Configuration

```toml
# .config/nextest.toml
[profile.ci]
retries = 2
slow-timeout = { period = "60s", terminate-after = 3 }
fail-fast = false
```

---

## 3. CI/CD

### Key Takeaways

1. **Tiered job architecture**: Gate jobs → lint → build → test → integration
2. **Use Swatinem/rust-cache**: With `save-if: ${{ github.ref == 'refs/heads/main' }}` to reduce cache storage
3. **Concurrency cancellation**: Cancel in-progress jobs when new commits arrive
4. **Security scanning**: cargo-audit + cargo-deny + Dependabot in every project
5. **Release automation**: cargo-dist for multi-platform binaries, installers, and checksums

### Essential CI Jobs

| Job | Tools | Purpose |
|-----|-------|---------|
| Lint | cargo fmt, clippy | Code quality gates |
| Test | cargo-nextest | Fast test execution |
| Security | cargo-audit, cargo-deny | Vulnerability scanning |
| MSRV | Specific toolchain | Compatibility verification |
| Coverage | cargo-llvm-cov | Track test coverage |

### Release Automation Stack

1. **cargo-dist**: Generates workflows, builds binaries, creates installers
2. **cargo-release**: Local workflow for version bumping, tagging, publishing
3. **release-plz**: Automated release PRs with changelogs

### Environment Defaults

```yaml
env:
  CARGO_INCREMENTAL: 0      # Faster CI builds
  CARGO_NET_RETRY: 10       # Handle network flakiness
  RUSTFLAGS: -D warnings    # Treat warnings as errors
```

---

## 4. Architecture

### Key Takeaways

1. **Flat workspace layout**: Use `crates/` directory with virtual manifest at root
2. **Workspace dependencies**: Centralize common dependencies in root Cargo.toml
3. **Match folder to crate names**: `crates/my-crate/` for `my-crate` package
4. **Use cargo xtask**: Replace shell scripts with Rust for complex automation
5. **Visibility**: Prefer `pub(crate)` over `pub` for internal items

### Error Handling Decision Tree

```
Should caller handle error variants differently?
├── Yes → Use thiserror (enum with variants)
└── No → Is this library or application code?
    ├── Library → Use thiserror
    └── Application → Use anyhow
        └── Complex workspace? → Consider snafu
```

### Standard Workspace Structure

```
project-root/
├── Cargo.toml          # Virtual manifest (no [package])
├── Cargo.lock
├── crates/
│   ├── project-core/
│   ├── project-cli/
│   └── project-utils/
├── tests/              # Integration tests
├── benches/            # Benchmarks
└── examples/           # Example programs
```

### API Design Essentials

- Follow Rust API Guidelines naming conventions
- Implement standard traits: Debug, Clone, PartialEq, Eq, Hash, Default
- Use builder pattern for complex construction (typed-builder or derive_builder)
- Document all public items with examples

---

## 5. CLI/UX

### Key Takeaways

1. **Human-first design**: Prioritize interactive users, maintain composability
2. **Help text**: Accept `-h`, `--help`, and `help`; lead with examples
3. **Error messages**: Rewrite for humans, add context, suggest fixes
4. **Progress indicators**: Use indicatif; auto-hide when not TTY
5. **Color handling**: Respect NO_COLOR, TERM=dumb, non-TTY detection

### Progress Pattern Selection

| Pattern | When to Use |
|---------|-------------|
| Spinner | Unknown duration, single task |
| X of Y | Known count, discrete items |
| Progress Bar | Known size/percentage |

### Error Message Format

```
error: can't write to file.txt

cause: permission denied (os error 13)

hint: Try running 'chmod +w file.txt' or run with sudo
```

### Color Conventions

- **Red**: Errors, failures
- **Yellow**: Warnings, deprecations
- **Green**: Success, completion
- **Cyan/Blue**: Info, progress, paths
- **Bold**: Emphasis, commands to run
- **Dim**: Secondary information, hints

### Shell Completion

Support Bash, Zsh, Fish, PowerShell using clap_complete.

---

## 6. Security

### Key Takeaways

1. **SECURITY.md required**: Document reporting process, response timeline, disclosure policy
2. **cargo-audit in CI**: Scan Cargo.lock against RustSec database on every PR
3. **cargo-deny**: Check licenses, ban crates, verify sources
4. **Dependabot**: Enable for automated vulnerability alerts and updates
5. **Miri for unsafe code**: `cargo +nightly miri test` in CI for undefined behavior detection

### Essential Security Toolchain

| Tool | Purpose | Frequency |
|------|---------|-----------|
| cargo-audit | Vulnerability scanning | Every PR, daily |
| cargo-deny | License/source checking | Every PR |
| Dependabot | Automated updates | Weekly |
| Miri | Unsafe code testing | Every PR (nightly) |
| Secret scanning | Credential detection | Continuous |

### SECURITY.md Template

```markdown
# Security Policy

## Reporting a Vulnerability

Please report security vulnerabilities to security@example.com.
Do NOT create public GitHub issues for security vulnerabilities.

## Response Process

1. Acknowledgment within 48 hours
2. Initial assessment within 7 days
3. Coordinated disclosure timeline

## Disclosure

We disclose via GitHub Security Advisories and RustSec.
```

### Supply Chain Security

- Generate SBOMs with cargo-sbom
- Consider Sigstore signing for releases
- Use GitHub artifact attestations for SLSA provenance
- Pin action versions with SHA in workflows

---

## 7. Distribution

### Key Takeaways

1. **Multi-channel approach**: Meet users where they are (shell, cargo, homebrew, etc.)
2. **Shell installers reduce friction**: Single-command installation for any user
3. **cargo-dist simplifies everything**: Handles cross-platform builds, installers, checksums
4. **Support cargo-binstall**: Just publish standard GitHub Releases with proper naming
5. **Static binaries**: MUSL builds work on any Linux distribution

### Distribution Channels Priority

1. Shell/PowerShell installer (primary for CLI tools)
2. Pre-compiled binaries (GitHub Releases)
3. cargo install / crates.io
4. cargo-binstall support
5. Homebrew (macOS)
6. System package managers (as demand grows)

### Target Platform Matrix

| OS | Architecture | Target Triple |
|----|--------------|---------------|
| Linux | x86_64 | x86_64-unknown-linux-gnu |
| Linux | x86_64 (static) | x86_64-unknown-linux-musl |
| Linux | ARM64 | aarch64-unknown-linux-gnu |
| macOS | x86_64 | x86_64-apple-darwin |
| macOS | ARM64 | aarch64-apple-darwin |
| Windows | x86_64 | x86_64-pc-windows-msvc |

### Container Best Practices

- Use `distroless/static` or `chainguard/static` for production
- Multi-stage builds with MUSL for smallest images
- Include CA certs and non-root user

---

## 8. Community

### Key Takeaways

1. **Start with basics**: CONTRIBUTING.md, CODE_OF_CONDUCT.md, issue templates
2. **Governance matches maturity**: BDFL for early projects, distribute as you grow
3. **Enforce code of conduct**: Unenforced codes are worse than none
4. **Use GitHub Discussions**: Separate conversation from work tracking
5. **Label consistently**: Enables discovery and triage

### Essential Community Files

| File | Purpose |
|------|---------|
| CONTRIBUTING.md | How to contribute |
| CODE_OF_CONDUCT.md | Community standards (use Contributor Covenant) |
| CODEOWNERS | Review routing |
| Issue templates | Structured bug reports, feature requests |
| PR template | Consistent PR descriptions |

### Issue Label Categories

- **Type**: bug, feature, enhancement, documentation
- **Priority**: P0/critical, P1/high, P2/medium, P3/low
- **Status**: needs-triage, needs-design, in-progress
- **Difficulty**: good-first-issue, help-wanted, expert-needed
- **Area**: specific components or modules

### Governance Models

| Model | Best For | Examples |
|-------|----------|----------|
| BDFL | Early-stage, strong vision | Python (historically) |
| Meritocratic | Mature, active contributors | Apache, Rust |
| Hybrid | Growing projects | Most successful OSS |

---

## 9. Performance

### Key Takeaways

1. **Benchmark with Criterion or Divan**: Don't rely on ad-hoc timing
2. **Profile before optimizing**: Use flamegraphs to find actual bottlenecks
3. **Continuous benchmarking in CI**: Use Iai for deterministic instruction counting
4. **Measure memory too**: Use dhat-rs or heaptrack for allocation profiling
5. **Build optimizations**: LTO + codegen-units=1 for maximum performance

### Benchmarking Tools

| Tool | Best For |
|------|----------|
| Criterion | Standard benchmarking, statistical analysis |
| Divan | Simpler API, built-in allocation profiling |
| Iai | CI environments, deterministic results |

### Profiling Tools

| Tool | Platform | Purpose |
|------|----------|---------|
| cargo-flamegraph | Linux, macOS | CPU profiling with flame graphs |
| samply | Cross-platform | Firefox Profiler UI |
| dhat-rs | Cross-platform | Heap analysis in tests |
| heaptrack | Linux | Allocation tracking |

### Release Profile for Performance

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```

### Release Profile for Size

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Implementation Priority

For a new Rust project, implement in this order:

### Phase 1: Foundation
- [ ] README with standard structure
- [ ] CONTRIBUTING.md
- [ ] CODE_OF_CONDUCT.md (Contributor Covenant)
- [ ] Basic CI (lint, test)
- [ ] cargo fmt + clippy in pre-commit

### Phase 2: Quality
- [ ] cargo-nextest for faster testing
- [ ] cargo-audit in CI
- [ ] SECURITY.md
- [ ] Issue templates
- [ ] PR template

### Phase 3: Distribution
- [ ] cargo-dist setup
- [ ] Shell/PowerShell installers
- [ ] cargo-binstall support
- [ ] Basic benchmarks (Criterion)

### Phase 4: Maturity
- [ ] External documentation site
- [ ] Continuous benchmarking
- [ ] CODEOWNERS
- [ ] Dependabot configuration
- [ ] OpenSSF Scorecard

---

## Sources

This reference synthesizes findings from:
- context/research/documentation-practices.md
- context/research/testing-practices.md
- context/research/cicd-practices.md
- context/research/architecture-practices.md
- context/research/cli-ux-practices.md
- context/research/security-practices.md
- context/research/distribution-practices.md
- context/research/community-practices.md
- context/research/performance-practices.md

Last updated: 2025-12-03
