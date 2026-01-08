# Security Guidelines for Open Source Rust Projects

Research compiled from industry sources, OSS project analysis, and security frameworks (December 2025).

## Executive Summary

Security in open source Rust projects spans multiple areas: dependency management, vulnerability disclosure, supply chain integrity, code auditing, and release signing. This document synthesizes recommendations from leading projects and security standards.

## 1. Dependency Security

### cargo-audit

[cargo-audit](https://crates.io/crates/cargo-audit) scans `Cargo.lock` against the [RustSec Advisory Database](https://rustsec.org/) to identify known vulnerabilities.

**Installation and Usage:**
```bash
cargo install cargo-audit --locked
cargo audit                    # Basic scan
cargo audit fix               # Auto-fix (experimental, requires --features=fix)
```

**CI Integration:**
- Use [rust-audit-check](https://github.com/rustsec/audit-check) GitHub Action for automated scanning
- Schedule regular dependency audits via GitHub Actions cron
- Block PRs with known vulnerabilities

### cargo-deny

[cargo-deny](https://crates.io/crates/cargo-deny) provides full supply chain checks:

1. **Licenses** - Verify all dependencies use acceptable licenses
2. **Bans** - Deny specific crates or detect duplicate versions
3. **Advisories** - Check against RustSec database
4. **Sources** - Ensure crates come from trusted sources only

**Setup:**
```bash
cargo install --locked cargo-deny
cargo deny init               # Create deny.toml
cargo deny check              # Run all checks
cargo deny check licenses     # License check only
```

**Example deny.toml:**
```toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause"]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
deny = ["openssl"]            # Prefer rustls

[advisories]
db-path = "~/.cargo/advisory-db"
vulnerability = "deny"
unmaintained = "warn"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

**CI Integration:** Use [cargo-deny-action](https://github.com/EmbarkStudios/cargo-deny-action) GitHub Action.

### Dependabot

GitHub's [Dependabot](https://docs.github.com/en/code-security/dependabot) provides:
- Automated vulnerability alerts from GitHub Advisory Database
- Security update PRs for vulnerable dependencies
- Version update PRs to keep dependencies current
- Support for `Cargo.toml` and `Cargo.lock`

**Enable via `.github/dependabot.yml`:**
```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
```

## 2. Vulnerability Disclosure

### SECURITY.md

Every project should have a `SECURITY.md` file (in root, `docs/`, or `.github/`) containing:

1. **Supported Versions** - Which versions receive security updates
2. **Reporting Process** - How to report vulnerabilities (email, not public issues)
3. **Response Timeline** - Expected acknowledgment and fix timelines
4. **Disclosure Policy** - Coordinated disclosure process
5. **Scope** - What constitutes a security vulnerability

**Example SECURITY.md (based on [uv](https://github.com/astral-sh/uv) and [tokio](https://github.com/tokio-rs/tokio)):**

```markdown
# Security Policy

## Reporting a Vulnerability

Please report security vulnerabilities to security@example.com.
Do NOT create public GitHub issues for security vulnerabilities.

## Response Process

1. We will acknowledge receipt within 48 hours
2. We will investigate and provide an initial assessment within 7 days
3. We will work with you on a coordinated disclosure timeline

## Disclosure

We will disclose security issues via:
- GitHub Security Advisories
- RustSec Advisory Database (via cargo-audit)
- Release notes

## Scope

The following are NOT considered security vulnerabilities:
- [List expected behaviors that might appear risky]
```

### GitHub Security Advisories

Use GitHub's [Security Advisories](https://docs.github.com/en/code-security/security-advisories) feature to:
- Privately discuss and fix vulnerabilities
- Request CVE identifiers
- Coordinate disclosure with downstream users
- Publish advisories that integrate with Dependabot

### RustSec Integration

Submit advisories to [rustsec/advisory-db](https://github.com/rustsec/advisory-db) so users can detect issues via `cargo audit`. The GitHub Advisory Database imports RustSec advisories automatically.

## 3. Unsafe Code and Memory Safety

### Scale of Unsafe Usage

Per [Rust Foundation research](https://rustfoundation.org/media/unsafe-rust-in-the-wild-notes-on-the-current-state-of-unsafe-rust/):
- ~19% of crates use `unsafe` directly
- ~34% call functions in crates that use `unsafe`
- Over 50% of RustSec advisories (2016-2021) involved memory safety issues

### Miri

[Miri](https://github.com/rust-lang/miri) interprets Rust MIR to detect undefined behavior in unsafe code:

```bash
rustup +nightly component add miri
cargo +nightly miri test
cargo +nightly miri run
```

**Detects:**
- Out-of-bounds access
- Use after free
- Memory leaks
- Data races
- Use of uninitialized data

**Limitations:**
- Requires nightly Rust
- Cannot interpret FFI/C code
- Only catches issues in executed code paths

### Sanitizers

Rust supports LLVM sanitizers for runtime analysis:

```bash
# AddressSanitizer (memory errors)
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test

# ThreadSanitizer (data races)
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test

# MemorySanitizer (uninitialized reads)
RUSTFLAGS="-Z sanitizer=memory" cargo +nightly test
```

### Guidelines for Unsafe Code

1. **Minimize scope** - Keep `unsafe` blocks as small as possible
2. **Safe abstractions** - Wrap unsafe code in safe APIs
3. **Document invariants** - Explain safety requirements in comments
4. **Test with Miri** - Run `cargo +nightly miri test` in CI
5. **Review carefully** - Unsafe code requires extra scrutiny

### RUDRA

[RUDRA](https://github.com/sslab-gatech/Rudra) is a static analyzer for detecting unsafe bug patterns across many crates. It found bugs in heavily-tested crates that existed for years.

## 4. Supply Chain Security

### SLSA Standard

[SLSA](https://slsa.dev/) (Supply-chain Levels for Software Artifacts) defines levels of supply chain security:

| Level | Requirements |
|-------|--------------|
| 0 | No guarantees |
| 1 | Provenance generated (who, where, how built) |
| 2 | Signed provenance from hosted build platform |
| 3 | Hardened build platform, two-person review |

**Key Concepts:**
- **Provenance** - Attestation describing how an artifact was built
- **Attestations** - Signed, machine-readable statements about artifacts
- **In-toto** - Standard format for attestations

### GitHub Artifact Attestations

GitHub provides built-in support for SLSA provenance:

```yaml
# .github/workflows/release.yml
jobs:
  build:
    permissions:
      id-token: write
      contents: read
      attestations: write
    steps:
      - uses: actions/attest-build-provenance@v1
        with:
          subject-path: 'target/release/myapp'
```

### Sigstore and Cosign

[Sigstore](https://www.sigstore.dev/) provides free code signing via:
- **Cosign** - Sign and verify artifacts
- **Fulcio** - Certificate authority
- **Rekor** - Transparency log

**Sign a binary:**
```bash
cosign sign-blob --yes myapp > myapp.sig
```

**Verify:**
```bash
cosign verify-blob --signature myapp.sig myapp
```

### Software Bill of Materials (SBOM)

SBOMs list all components in software for vulnerability tracking.

**Generate SBOM with cargo-sbom:**
```bash
cargo install cargo-sbom
cargo sbom --output-format spdx_json > sbom.spdx.json
cargo sbom --output-format cyclonedx_json > sbom.cdx.json
```

**Formats:**
- **SPDX** - Linux Foundation standard
- **CycloneDX** - OWASP standard

Attach SBOMs to GitHub releases for downstream consumers.

## 5. GitHub Security Features

### Code Scanning

Enable [CodeQL](https://docs.github.com/en/code-security/code-scanning) for static analysis:

```yaml
# .github/workflows/codeql.yml
name: CodeQL
on: [push, pull_request]
jobs:
  analyze:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: github/codeql-action/init@v3
      - uses: github/codeql-action/analyze@v3
```

### Secret Scanning

Enable secret scanning to detect accidentally committed credentials:
- Repository Settings > Security > Secret scanning
- Define custom patterns for internal secrets

### Dependency Review

Block PRs introducing vulnerabilities:

```yaml
# .github/workflows/dependency-review.yml
name: Dependency Review
on: [pull_request]
jobs:
  dependency-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/dependency-review-action@v4
        with:
          fail-on-severity: moderate
```

### Branch Protection

Enforce security requirements via branch protection:
- Require PR reviews before merging
- Require status checks (including security scans)
- Require signed commits
- Restrict who can push to protected branches

## 6. OpenSSF Scorecard and Certification Badge

### OpenSSF Scorecard

[Scorecard](https://scorecard.dev/) automatically assesses security practices:

```yaml
# .github/workflows/scorecard.yml
name: Scorecard
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly
  push:
    branches: [main]
jobs:
  analysis:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
      id-token: write
    steps:
      - uses: ossf/scorecard-action@v2
        with:
          publish_results: true
```

**Display badge in README:**
```markdown
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/OWNER/REPO/badge)](https://scorecard.dev/viewer/?uri=github.com/OWNER/REPO)
```

**Checks include:**
- Branch protection enabled
- CI tests run
- Dependencies pinned
- Security policy present
- Signed releases
- Code review required
- Fuzzing enabled

### OpenSSF Certification Badge

[Certification Badge](https://www.bestpractices.dev/) is a self-certification program with three tiers: passing, silver, and gold.

**Badge in README:**
```markdown
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/XXXXX/badge)](https://www.bestpractices.dev/projects/XXXXX)
```

Only ~10% of pursuing projects achieve passing status.

## 7. Recommended Security Toolchain

### Minimal Setup

```yaml
# .github/workflows/security.yml
name: Security
on: [push, pull_request]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### Full Setup

| Tool | Purpose | Frequency |
|------|---------|-----------|
| cargo-audit | Vulnerability scanning | Every PR, daily |
| cargo-deny | License/source checking | Every PR |
| Dependabot | Automated updates | Weekly |
| Miri | Unsafe code testing | Every PR (nightly) |
| Scorecard | Security posture | Weekly |
| CodeQL | Static analysis | Every PR |
| Secret scanning | Credential detection | Continuous |

### Pre-commit Integration

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/DevinR528/cargo-sort
    rev: v1.0.9
    hooks:
      - id: cargo-sort
  - repo: local
    hooks:
      - id: cargo-deny
        name: cargo-deny
        entry: cargo deny check
        language: system
        pass_filenames: false
```

## 8. Exemplar Projects

### uv (Astral)

- Clear SECURITY.md with scope exclusions
- Security email: security@astral.sh
- GitHub Security Advisories for disclosure
- No bug bounty (common for OSS)

### tokio

- Private reporting via security@tokio.rs
- Coordinates with downstream users
- Publishes to both GitHub releases and RustSec
- Dual-channel disclosure approach

### General Patterns Observed

1. **Dedicated security email** - Not public issues
2. **GitHub Security Advisories** - Standard disclosure mechanism
3. **RustSec integration** - Enables cargo-audit detection
4. **Clear scope definition** - What is/isn't a vulnerability
5. **No bug bounty** - Common for smaller OSS projects

## References

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-audit Documentation](https://crates.io/crates/cargo-audit)
- [cargo-deny Documentation](https://crates.io/crates/cargo-deny)
- [GitHub Code Security](https://docs.github.com/en/code-security)
- [OpenSSF Scorecard](https://scorecard.dev/)
- [OpenSSF Certification Badge](https://www.bestpractices.dev/)
- [SLSA Standard](https://slsa.dev/)
- [Sigstore](https://www.sigstore.dev/)
- [OpenSSF Vulnerability Disclosure Guide](https://openssf.org/blog/2021/09/27/announcing-the-openssf-vulnerability-disclosure-wg-guide-to-disclosure-for-oss-projects/)
- [Rust Foundation: Unsafe Rust in the Wild](https://rustfoundation.org/media/unsafe-rust-in-the-wild-notes-on-the-current-state-of-unsafe-rust/)
- [Making Unsafe Rust Safer](https://blog.colinbreck.com/making-unsafe-rust-a-little-safer-tools-for-verifying-unsafe-code/)
