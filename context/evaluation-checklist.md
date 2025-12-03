# Open Source Rust Project Evaluation Checklist

Use this checklist to evaluate a project against open source best practices. Each item can be marked:
- **Pass** - Fully implemented
- **Partial** - Partially implemented or needs improvement
- **Fail** - Not implemented or missing
- **N/A** - Not applicable to this project

---

## 1. Documentation

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 1.1 | README follows standard structure (logo/badges, tagline, features, installation, quick start, links) | | |
| 1.2 | Multiple installation methods documented (3+ for libraries, 5+ for CLI tools) | | |
| 1.3 | All public items have rustdoc with summary, description, and examples | | |
| 1.4 | README content synced with crate docs via `doc = include_str!` | | |
| 1.5 | Standard badges present (CI, version, license, docs) | | |
| 1.6 | CHANGELOG.md exists and follows Keep a Changelog format | | |
| 1.7 | Getting started tutorial exists (first 5 minutes experience) | | |
| 1.8 | API reference is auto-generated and published (docs.rs or custom) | | |
| 1.9 | External docs site for complex features (mkdocs-material or mdBook) | | |
| 1.10 | FAQ or troubleshooting section exists | | |

**Section Score**: ___ / 10

---

## 2. Testing

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 2.1 | Unit tests inline with code using `#[cfg(test)]` modules | | |
| 2.2 | Integration tests in `tests/` directory | | |
| 2.3 | Uses cargo-nextest for faster test execution | | |
| 2.4 | Doctests run separately (`cargo test --doc`) | | |
| 2.5 | Test coverage measured (cargo-tarpaulin or cargo-llvm-cov) | | |
| 2.6 | Coverage target defined and enforced (recommended: 80%+) | | |
| 2.7 | Test helpers in shared location (`tests/common/` or dev-dep crate) | | |
| 2.8 | CI profile configured for test timeouts and retries | | |
| 2.9 | Snapshot testing used where appropriate (insta) | | |
| 2.10 | Property-based testing for edge cases (proptest) | | |

**Section Score**: ___ / 10

---

## 3. CI/CD

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 3.1 | Tiered job architecture (lint → build → test → integration) | | |
| 3.2 | Rust cache configured (Swatinem/rust-cache with save-if) | | |
| 3.3 | Concurrency cancellation enabled for in-progress jobs | | |
| 3.4 | cargo-audit runs on every PR | | |
| 3.5 | cargo-deny configured for license/source checking | | |
| 3.6 | MSRV verification in CI matrix | | |
| 3.7 | Release automation configured (cargo-dist, cargo-release, or release-plz) | | |
| 3.8 | Environment defaults set (CARGO_INCREMENTAL=0, RUSTFLAGS=-D warnings) | | |
| 3.9 | Coverage reporting in CI | | |
| 3.10 | Action versions pinned with SHA | | |

**Section Score**: ___ / 10

---

## 4. Architecture

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 4.1 | Workspace layout uses `crates/` directory (if multi-crate) | | |
| 4.2 | Workspace dependencies centralized in root Cargo.toml | | |
| 4.3 | Folder names match crate names | | |
| 4.4 | Clear separation between library and binary crates | | |
| 4.5 | Error handling uses appropriate strategy (thiserror/anyhow) | | |
| 4.6 | Public API follows Rust API Guidelines naming | | |
| 4.7 | Standard traits implemented (Debug, Clone, PartialEq, etc.) | | |
| 4.8 | Builder pattern used for complex construction | | |
| 4.9 | Internal items use `pub(crate)` over `pub` | | |
| 4.10 | cargo xtask for complex automation (vs shell scripts) | | |

**Section Score**: ___ / 10

---

## 5. CLI/UX

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 5.1 | Help accessible via `-h`, `--help`, and `help` subcommand | | |
| 5.2 | Help text leads with usage examples | | |
| 5.3 | Error messages are human-readable with context | | |
| 5.4 | Error messages suggest fixes when possible | | |
| 5.5 | Progress indicators used for long operations (indicatif) | | |
| 5.6 | Progress auto-hidden when not TTY | | |
| 5.7 | Colors follow conventions (red=error, green=success, etc.) | | |
| 5.8 | Respects NO_COLOR, TERM=dumb, and non-TTY detection | | |
| 5.9 | Shell completions provided (bash, zsh, fish, powershell) | | |
| 5.10 | Exit codes are meaningful (0=success, 1=error, 2=usage) | | |

**Section Score**: ___ / 10

---

## 6. Security

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 6.1 | SECURITY.md exists with reporting process | | |
| 6.2 | Response timeline documented in SECURITY.md | | |
| 6.3 | cargo-audit runs in CI on every PR | | |
| 6.4 | cargo-deny configured and runs in CI | | |
| 6.5 | Dependabot enabled for automated updates | | |
| 6.6 | Miri tests for unsafe code (if applicable) | | |
| 6.7 | GitHub secret scanning enabled | | |
| 6.8 | Action versions pinned with SHA (not tags) | | |
| 6.9 | SBOM generation configured (cargo-sbom) | | |
| 6.10 | Release artifacts signed or attested | | |

**Section Score**: ___ / 10

---

## 7. Distribution

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 7.1 | Pre-compiled binaries published to GitHub Releases | | |
| 7.2 | Shell/PowerShell installer available | | |
| 7.3 | cargo install supported (published to crates.io) | | |
| 7.4 | cargo-binstall compatible (standard GitHub Release naming) | | |
| 7.5 | Homebrew formula available (macOS) | | |
| 7.6 | Target platforms include Linux (x86_64, ARM64), macOS (x86_64, ARM64), Windows | | |
| 7.7 | Static MUSL builds available for Linux | | |
| 7.8 | Checksums published with releases | | |
| 7.9 | Container images available (if applicable) | | |
| 7.10 | Installation instructions cover all channels | | |

**Section Score**: ___ / 10

---

## 8. Community

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 8.1 | CONTRIBUTING.md exists with development setup | | |
| 8.2 | CODE_OF_CONDUCT.md exists (Contributor Covenant recommended) | | |
| 8.3 | Bug report issue template configured | | |
| 8.4 | Feature request issue template configured | | |
| 8.5 | Pull request template configured | | |
| 8.6 | CODEOWNERS file defines review routing | | |
| 8.7 | GitHub Discussions enabled for Q&A | | |
| 8.8 | Consistent label system for issues | | |
| 8.9 | Good first issues labeled for new contributors | | |
| 8.10 | License clearly stated (LICENSE file + Cargo.toml) | | |

**Section Score**: ___ / 10

---

## 9. Performance

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 9.1 | Benchmarks exist using Criterion, Divan, or Iai | | |
| 9.2 | Benchmarks run in CI (at least nightly or weekly) | | |
| 9.3 | Release profile optimized (LTO, codegen-units=1) | | |
| 9.4 | Profiling documented or scripted (flamegraphs) | | |
| 9.5 | Memory usage tracked (dhat-rs or heaptrack) | | |
| 9.6 | Performance regression detection in CI | | |
| 9.7 | Size-optimized profile available (for size-sensitive deployments) | | |
| 9.8 | Compile time optimization (workspace hack or similar) | | |
| 9.9 | Critical paths identified and optimized | | |
| 9.10 | Performance documentation exists | | |

**Section Score**: ___ / 10

---

## Summary

| Area | Score | Priority |
|------|-------|----------|
| 1. Documentation | /10 | |
| 2. Testing | /10 | |
| 3. CI/CD | /10 | |
| 4. Architecture | /10 | |
| 5. CLI/UX | /10 | |
| 6. Security | /10 | |
| 7. Distribution | /10 | |
| 8. Community | /10 | |
| 9. Performance | /10 | |
| **Total** | **/90** | |

### Scoring Guide

- **80-90**: Excellent - Ready for production OSS
- **60-79**: Good - Solid foundation, some gaps to address
- **40-59**: Fair - Core practices in place, significant improvements needed
- **20-39**: Needs Work - Basic structure exists, many gaps
- **0-19**: Early Stage - Focus on fundamentals first

### Priority Recommendations

Based on Implementation Priority from best-practices-reference.md:

**Phase 1 (Foundation)**: Documentation 1.1-1.5, Community 8.1-8.2, 8.10, CI/CD 3.1
**Phase 2 (Quality)**: Testing 2.1-2.6, Security 6.1-6.5, Community 8.3-8.5
**Phase 3 (Distribution)**: Distribution 7.1-7.4, Performance 9.1
**Phase 4 (Maturity)**: Remaining items, advanced features

---

## How to Use This Checklist

1. **Initial Audit**: Go through each criterion and mark current status
2. **Prioritize**: Use the Implementation Priority phases to order work
3. **Create Sub-Plans**: For each area needing work, create a detailed JSON plan
4. **Track Progress**: Update this checklist as improvements are made
5. **Re-evaluate**: Periodically re-audit to measure progress

---

Last updated: 2025-12-03
