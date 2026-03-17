# External Integrations

**Analysis Date:** 2026-03-16

## APIs & External Services

**Git Repository Operations:**
- Git command-line tool - Core integration via system `git` command
  - SDK/Client: System `git` executable
  - Operations: Clone, branch/tag listing, shallow cloning
  - Authentication: SSH keys (~/.ssh/), Git credential helpers, PATs

**Version Detection:**
- Remote Git repository tags - Fetched to check for semantic version updates
  - Method: `git ls-remote --tags <url>`
  - Used by: Version checking and update detection

## Data Storage

**Databases:**
- None - Application is filesystem and Git-based
- No persistent data store required

**File Storage:**
- **Local filesystem** - Primary data storage model
  - Cache directory: System standard cache location (`dirs::cache_dir()`)
  - Repository cache: Stores cloned Git repositories
  - Configuration files: `.common-repo.yaml` files in project roots

**In-Memory Filesystem:**
- `MemoryFS` abstraction - Virtual filesystem implementation
  - Used for staging changes before disk write
  - Enables complex transformations and dry-run simulation
  - Implements read/write operations without touching disk

**Caching:**
- **Two-tier caching strategy:**
  1. **Disk Cache** - Persistent repository cache in system cache directory
     - Stores cloned repository snapshots
     - Cache key: Hash of URL + ref + optional path
  2. **In-Process Cache** - `RepoCache` for current execution
     - Thread-safe (Mutex-protected)
     - Caches processed repositories during orchestration
     - Prevents redundant processing of same repository

## Authentication & Identity

**Auth Provider:**
- Custom - Delegates to Git's built-in authentication
  - Implementation: Leverages user's Git configuration
  - Supported methods:
    - SSH keys from `~/.ssh/`
    - Git credential helpers (system keychain, credential-store, etc.)
    - Personal access tokens (PATs) via credential helpers
    - Basic auth via `.netrc`
  - No custom authentication layer - relies on system Git setup

**Authorization:**
- Git repository access control (read-only operations)
- User's existing Git credentials determine access

## Monitoring & Observability

**Error Tracking:**
- None - No external error tracking service integrated

**Logs:**
- Approach: Standard Rust `log` crate with `env_logger` backend
  - Configuration via `RUST_LOG` environment variable
  - Output: stderr
  - Levels: error, warn, info, debug, trace
  - No remote log aggregation

**Output:**
- Terminal output via `console` crate
- Color support detection via environment variables (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `TERM`)
- Progress indication via `indicatif` (progress bars, spinners)

## CI/CD & Deployment

**Hosting:**
- GitHub (source code repository)
- GitHub Actions (CI/CD pipeline)

**CI Pipeline:**
- **Provider:** GitHub Actions (`.github/workflows/`)
- **Workflows:**
  - `ci.yml` - Main CI pipeline: lint, test, MSRV check, security audit, coverage, formatting, linting
  - `commitlint.yml` - Commit message validation
  - `test-action.yml` - Testing the GitHub Action for setup-common-repo
  - `auto-merge.yml` - Automated PR merging
  - `release-please.yml` - Semantic versioning and release automation
  - `benchmark.yml` - Performance benchmarking
  - `docs.yml` - Documentation build and deployment

**Release Automation:**
- `release-please` - Automated versioning and CHANGELOG generation
  - Configuration: `release-please-config.json`
  - Release type: Rust (Cargo.toml versioning)
  - Triggers: Conventional commits
  - Outputs: GitHub releases, CHANGELOG.md updates

**Pre-commit Hooks:**
- Repository: https://github.com/doublify/pre-commit-rust
  - Hooks: rustfmt, cargo-check, clippy
- Repository: https://github.com/mpalmer/action-validator
  - Validates GitHub Actions workflow YAML syntax
- Repository: https://github.com/pre-commit/pre-commit-hooks
  - Trailing whitespace, EOF fixes, merge conflict checks

## Environment Configuration

**Required env vars:**
- `RUST_LOG` - Control logging verbosity (optional, defaults to errors only)
- `NO_COLOR` - Disable colored terminal output (if set)
- `CLICOLOR` - Control color mode ("0" to disable)
- `CLICOLOR_FORCE` - Force colored output (overrides NO_COLOR)
- `TERM` - Terminal type for capability detection

**Git Authentication (external):**
- SSH keys in `~/.ssh/` - Private key files
- `.gitconfig` - User's Git configuration
- Credential helpers - System-specific (macOS keychain, Linux secret-service, etc.)
- `.netrc` - Basic auth credentials (legacy)

**Secrets location:**
- None stored in application - relies on system Git configuration
- Git credentials come from user's environment setup:
  - SSH agent
  - System credential manager
  - Git credential helper (configured in .gitconfig)

## Webhooks & Callbacks

**Incoming:**
- None - Application is pull-only, not a server

**Outgoing:**
- None - Application does not send webhooks or HTTP requests
- Git-based only (clone, fetch operations)

## Integration Patterns

**Git Operations:**
- Shallow cloning with depth=1 for speed and disk efficiency
- Uses system `git` command (not a Git library)
- Supports all Git authentication methods automatically
- Branch/tag/commit reference support

**Repository Discovery:**
- Recursive inheritance through `.common-repo.yaml` files
- Parallel cloning with `rayon` for performance
- Cache-based storage to minimize network requests

**Configuration Composition:**
- Supports merging from multiple sources:
  - YAML (`serde_yaml`)
  - TOML (`toml`, `taplo`)
  - JSON (`serde_json`)
  - Markdown (`pulldown-cmark`)
  - INI (`rust-ini`)
- Deterministic merge ordering
- Local file precedence

**Testing Integration:**
- Data-driven tests via `datatest-stable` (YAML schema files)
- Snapshot testing with `insta` (YAML format)
- CLI testing via `assert_cmd` for E2E validation
- Property-based testing via `proptest`

---

*Integration audit: 2026-03-16*
