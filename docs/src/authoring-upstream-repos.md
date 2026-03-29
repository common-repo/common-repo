# Authoring Upstream Repositories

This guide explains how to create and maintain upstream repositories that other projects can inherit from using common-repo.

## What is an Upstream Repository?

An **upstream repository** contains configuration files, templates, and standards that other repositories (consumers) can inherit—essentially a library of reusable project configurations.

**Upstream repository** vs **consumer repository**:

| Aspect | Upstream Repository | Consumer Repository |
|--------|-------------------|---------------------|
| Purpose | Provides files for others to inherit | Uses files from upstream repos |
| Audience | Maintainers of shared standards | Individual projects |
| Config file | Optional `.common-repo.yaml` | Required `.common-repo.yaml` |
| Versioning | Semantic versioning with Git tags | References upstream repo versions |
| Can also consume | Yes, via `self:` blocks (local-only) | Yes, directly via `repo:` |

**Common use cases for upstream repositories:**

- Organization-wide coding standards (linting, formatting rules)
- CI/CD workflow templates (GitHub Actions, GitLab CI)
- Project scaffolding and boilerplate
- Security policies and configurations
- Documentation templates

## Getting Started

### Minimal Upstream Repository Structure

A upstream repository can be as simple as a directory with files to share:

```
my-upstream-repo/
├── .github/
│   └── workflows/
│       └── ci.yml
├── .pre-commit-config.yaml
└── README.md
```

No special configuration is required. Consumers reference your repository directly:

```yaml
# Consumer's .common-repo.yaml
- repo:
    url: https://github.com/your-org/my-upstream-repo
    ref: v1.0.0
```

### Optional: Configuration in Upstream Repos

Upstream repositories can include their own `.common-repo.yaml` to inherit from other upstreams, creating an inheritance chain:

```
org-base/          # Base standards
  └── rust-base/   # Rust-specific (inherits org-base)
      └── my-app/  # Consumer (inherits rust-base)
```

### Using `self:` for Local Consumption

An upstream repo often needs to consume tooling from *its own* upstreams — CI config, pre-commit hooks, release automation — without leaking those files to the repos that consume it. The `self:` operator solves this.

Operations inside a `self:` block run in an isolated pipeline. Their output is written to the local working directory but never enters the composite filesystem that consumers see. This lets a single `.common-repo.yaml` define both what the repo provides (its source API) and what it consumes locally.

```yaml
# .common-repo.yaml for an upstream repo

# Local consumption — pull tooling for this repo's own use.
# Consumers never see these operations.
- self:
    - repo:
        url: https://github.com/org/ci-tooling
        ref: v2.0.0
    - exclude:
        - ".releaserc.yaml"

# Source API — what consumers inherit
- include:
    - "src/**"
    - "src/.*"
- rename:
    - from: "^src/(.*)$"
      to: "$1"
```

Without `self:`, this repo would need a separate mechanism to pull its own tooling, or its consumers would inherit the CI tooling files unintentionally.

**Key points:**

- `self:` blocks are stripped when a consumer inherits from this repo — consumers never see them
- Any operator can appear inside `self:` (repo, include, exclude, rename, merge operators, etc.)
- Multiple `self:` blocks are allowed; each runs as an independent pipeline
- `self:` blocks cannot be nested
- The source pipeline runs first, then each `self:` block runs afterward as an independent pipeline invocation

See the [Configuration Reference](configuration.md#self---local-only-operations) for the full operator specification.

## Upstream-Declared File Filtering

Upstream repositories can define their "public API" by specifying which files are exposed to consumers. This is useful when a repository contains internal files that should not be inherited.

### Filtering Operations in Upstream Repos

Upstream repos can use these operations to control which files consumers receive:

| Operation | Purpose |
|-----------|---------|
| `include` | Allowlist of files to expose (all others excluded) |
| `exclude` | Blocklist of files to hide (all others included) |
| `rename` | Transform file paths before consumers see them |

### Example: Exposing Only Public Files

An upstream repo with internal test fixtures that should not be inherited:

```yaml
# Upstream repo: .common-repo.yaml
- include:
    patterns:
      - "templates/**"
      - "configs/**"
      - ".github/**"
# Internal test fixtures, scripts, and docs are NOT exposed
```

Consumers automatically receive only the declared public files.

### Example: Hiding Internal Files

An upstream repo that exposes everything except internal directories:

```yaml
# Upstream repo: .common-repo.yaml
- exclude:
    patterns:
      - "internal/**"
      - "scripts/dev-*.sh"
      - ".internal-*"
```

### Example: Renaming Template Files

An upstream repo that provides templates with a different naming convention:

```yaml
# Upstream repo: .common-repo.yaml
- rename:
    - from: "templates/(.*)\\.template"
      to: "$1"
```

This transforms `templates/config.yaml.template` to `config.yaml` in consumers.

### Operation Order

Operations are applied in this order:

1. **Upstream filtering** (include/exclude/rename from upstream's config)
2. **Upstream merge declarations** (deferred merge operations)
3. **Consumer's with: clause** (further filtering/transforms by consumer)

This ensures upstream repos control their exposed files, while consumers can further filter (but not expand) what they receive.

### Config File Auto-Exclusion

Upstream repository config files (`.common-repo.yaml` and `.commonrepo.yaml`) are automatically excluded and never copied to consumers. This prevents upstream configs from overwriting consumer configs.

## Upstream-Declared Merge Behavior

By default, files from upstream repositories overwrite files in consumer repositories. However, upstream authors often know best how their files should integrate. The **defer** mechanism allows upstream repos to declare merge behavior that automatically applies when consumers inherit from them.

### When to Use Upstream-Declared Merges

Use upstream-declared merges when:

- Your upstream provides partial content meant to augment consumer files (e.g., additional CLAUDE.md rules)
- Files need intelligent merging rather than overwriting (e.g., shared dependencies in Cargo.toml)
- You want to reduce boilerplate in consumer configurations

### Two Syntax Forms

**1. `auto-merge` - When source and destination have the same filename (most common):**

```yaml
# Upstream repo: .common-repo.yaml
- markdown:
    auto-merge: CLAUDE.md
    section: "## Inherited Rules"
    append: true
```

This is shorthand for: source=CLAUDE.md, dest=CLAUDE.md, defer=true.

**2. `defer: true` - When source and destination paths differ:**

```yaml
# Upstream repo: .common-repo.yaml
- yaml:
    source: config/labels.yaml
    dest: kubernetes.yaml
    path: metadata.labels
    defer: true
```

### Example: Sharing CLAUDE.md Rules

An organization wants all repos to inherit coding guidelines:

```yaml
# Upstream repo: org-standards/.common-repo.yaml
- markdown:
    auto-merge: CLAUDE.md
    section: "## Organization Standards"
    append: true
    create-section: true
```

Consumer repos automatically get merged CLAUDE.md content:

```yaml
# Consumer repo: .common-repo.yaml
- repo:
    url: https://github.com/org/org-standards
    ref: v1.0.0
# No 'with:' clause needed - CLAUDE.md merges automatically
```

### Example: Sharing Dependencies

A base Rust configuration shares common dependencies:

```yaml
# Upstream repo: rust-base/.common-repo.yaml
- toml:
    auto-merge: Cargo.toml
    path: dependencies
```

Consumers inherit the base dependencies merged into their Cargo.toml.

### Consumer Override

Consumers can always override upstream-declared behavior using the `with:` clause:

```yaml
# Consumer .common-repo.yaml
- repo:
    url: https://github.com/org/org-standards
    ref: v1.0.0
    with:
      # Override: copy instead of merge
      - include: ["CLAUDE.md"]
      # This replaces the upstream-declared merge with a simple copy
```

When a consumer specifies merge operations for the same destination file, the consumer's merge operations run after deferred (upstream-declared) merge operations, so consumer parameters take effect last.

### Supported Merge Operators

All merge operators support `defer` and `auto-merge`:

| Operator | Example Use Case |
|----------|------------------|
| `markdown` | Shared CLAUDE.md rules, README sections |
| `yaml` | Kubernetes labels, GitHub Actions workflow steps |
| `json` | package.json dependencies, tsconfig settings |
| `toml` | Cargo.toml dependencies, pyproject.toml settings |
| `ini` | Git config defaults, editor settings |

### Publishing Your First Version

1. **Commit your configuration files** to the repository
2. **Create a Git tag** following semantic versioning:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
3. **Consumers can now reference it**:
   ```yaml
   - repo:
       url: https://github.com/your-org/my-upstream-repo
       ref: v1.0.0
   ```

## File Organization

### Repository Root vs Subdirectory

By default, consumers inherit from the repository root. Use the `path` option to expose a subdirectory:

```
my-upstream-repo/
├── templates/
│   ├── rust/          # Rust project templates
│   │   ├── .github/
│   │   └── Cargo.toml
│   └── python/        # Python project templates
│       ├── .github/
│       └── pyproject.toml
└── README.md
```

Consumers select which template to use:

```yaml
# Consumer's .common-repo.yaml
- repo:
    url: https://github.com/your-org/my-upstream-repo
    ref: v1.0.0
    path: templates/rust  # Only inherit from this subdirectory
```

### Organizing Files by Concern

Group related files together to make selective inheritance easier:

```
upstream-repo/
├── ci/                    # CI/CD configurations
│   ├── .github/
│   └── .gitlab-ci.yml
├── quality/               # Code quality tools
│   ├── .pre-commit-config.yaml
│   ├── .editorconfig
│   └── rustfmt.toml
├── security/              # Security configurations
│   ├── .github/dependabot.yml
│   └── SECURITY.md
└── README.md
```

Consumers can pick specific concerns:

```yaml
- repo:
    url: https://github.com/your-org/upstream-repo
    ref: v1.0.0
    with:
      - include: ["ci/**", "quality/**"]
```

### Dotfiles and Hidden Files

Dotfiles (files starting with `.`) are included by default. Structure them naturally:

```
upstream-repo/
├── .github/
│   ├── workflows/
│   │   └── ci.yml
│   └── dependabot.yml
├── .pre-commit-config.yaml
├── .editorconfig
└── .gitignore
```

Files you typically **should not** include in upstream repos:
- `.git/` directory (automatically excluded)
- `.env` files with secrets
- Local IDE configurations (`.vscode/`, `.idea/`)

## Template Variables

Template variables let consumers customize inherited files.

### Naming Conventions

Use descriptive, lowercase names with underscores:

```yaml
# Good variable names
project_name: my-app
rust_version: "1.75"
org_name: acme-corp
enable_coverage: true

# Avoid
PROJECT: my-app      # Uppercase
proj: my-app         # Abbreviation
projectName: my-app  # CamelCase
```

### Required vs Optional Variables

Document which variables consumers must provide. In your template files, use sensible defaults where possible:

```yaml
# .github/workflows/ci.yml (in upstream repo)
name: CI

env:
  RUST_VERSION: ${{ vars.rust_version || '1.75' }}  # Optional with default
  PROJECT_NAME: ${{ vars.project_name }}            # Required
```

### Documenting Variables

Include a README section or separate file listing available variables:

```markdown
## Template Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `project_name` | Yes | - | Name of the project |
| `rust_version` | No | 1.75 | Rust toolchain version |
| `enable_coverage` | No | false | Enable code coverage |
```

### Variable Overrides

Child repos can override variables defined by their parents:

```yaml
# Consumer's .common-repo.yaml
- template-vars:
    project_name: my-project
    ci_timeout: "30"
```

## Versioning and Releases

### Semantic Versioning

Follow [semantic versioning](https://semver.org/) for upstream repositories:

- **MAJOR** (`v2.0.0`): Breaking changes that require consumer updates
- **MINOR** (`v1.1.0`): New files or features, backward compatible
- **PATCH** (`v1.0.1`): Bug fixes, documentation updates

### When to Bump Versions

| Change Type | Version Bump | Example |
|------------|--------------|---------|
| Removed file that consumers depend on | Major | Deleting `.pre-commit-config.yaml` |
| Renamed template variable | Major | `project_name` → `name` |
| Changed file structure consumers reference | Major | Moving `ci.yml` to new path |
| Added new optional files | Minor | New workflow file |
| Added new template variable with default | Minor | New `enable_feature` variable |
| Fixed bug in configuration | Patch | Corrected YAML syntax |
| Updated documentation | Patch | Clarified usage instructions |

### Git Tagging Guidelines

```bash
# Create an annotated tag with message
git tag -a v1.2.0 -m "Add Python support and coverage workflows"

# Push the tag
git push origin v1.2.0

# List existing tags
git tag -l "v*"
```

Annotated tags (`-a`) are preferred as they include author and date information.

### Changelog Maintenance

Maintain a `CHANGELOG.md` to document changes:

```markdown
# Changelog

## [1.2.0] - 2024-01-15
### Added
- Python project template in `templates/python/`
- Coverage workflow for all language templates

### Changed
- Updated Rust version default to 1.75

## [1.1.0] - 2024-01-01
### Added
- Security policy template
```

## Testing Your Upstream Repository

### Testing Locally

Test your upstream repo against a local consumer before publishing:

```bash
# In consumer repository
cd my-consumer-project

# Reference local upstream repo instead of remote
# Edit .common-repo.yaml temporarily:
# - repo:
#     url: /path/to/local/upstream-repo
#     ref: HEAD

# Or use a file:// URL
# - repo:
#     url: file:///absolute/path/to/upstream-repo
#     ref: HEAD

common-repo ls      # Verify expected files
common-repo diff    # Check for issues
common-repo apply --dry-run
```

### Creating a Test Consumer

Maintain a test consumer repository or directory:

```
upstream-repo/
├── .github/
├── templates/
├── tests/
│   └── test-consumer/    # Test consumer project
│       ├── .common-repo.yaml
│       └── validate.sh
└── README.md
```

The test consumer validates that your upstream repo works correctly:

```yaml
# tests/test-consumer/.common-repo.yaml
- repo:
    url: ../..  # Relative path to upstream repo root
    ref: HEAD
    with:
      - include: ["templates/rust/**"]
```

### CI Testing Strategies

Add CI workflows that validate your upstream repository:

```yaml
# .github/workflows/test.yml
name: Test Upstream Repo

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install common-repo
        run: cargo install common-repo

      - name: Validate against test consumer
        run: |
          cd tests/test-consumer
          common-repo validate
          common-repo apply --dry-run
```

## Composability

### Designing for Multiple Upstream Inheritance

Consumers often inherit from multiple upstream repositories. Design yours to work alongside others:

```yaml
# Consumer inheriting from multiple upstream repos
- repo:
    url: https://github.com/org/base-standards
    ref: v1.0.0

- repo:
    url: https://github.com/org/rust-config
    ref: v2.0.0

- repo:
    url: https://github.com/org/security-policies
    ref: v1.5.0
```

### Avoiding File Conflicts

When multiple upstream repos provide similar files, conflicts can occur. Strategies to avoid this:

**Use specific subdirectories:**
```
# Instead of:
upstream-repo/
└── ci.yml

# Use:
upstream-repo/
└── .github/workflows/
    └── source-name-ci.yml   # Prefixed to avoid conflicts
```

**Document file ownership:**
```markdown
## Files Provided

This upstream repository provides:
- `.github/workflows/lint.yml` - Linting workflow
- `.github/workflows/test.yml` - Test workflow

If you inherit from other upstream repos, ensure they don't provide the same files,
or use `exclude` to skip conflicting files.
```

### Namespace Considerations

Consider prefixing files with your upstream repo's purpose:

```
org-security/
└── .github/
    └── workflows/
        └── security-scan.yml   # Clear ownership

org-quality/
└── .github/
    └── workflows/
        └── quality-lint.yml    # No conflict with security-scan.yml
```

## What to Include and Exclude

### Good Candidates for Upstream Repos

| File Type | Examples | Why Share |
|-----------|----------|-----------|
| CI/CD workflows | `.github/workflows/`, `.gitlab-ci.yml` | Standardize build/test/deploy |
| Code quality | `.pre-commit-config.yaml`, `rustfmt.toml` | Consistent formatting |
| Editor configs | `.editorconfig`, `.vscode/settings.json` | Shared development experience |
| Security | `SECURITY.md`, `dependabot.yml` | Org-wide security policies |
| Documentation templates | `CONTRIBUTING.md`, issue templates | Consistent contributor experience |
| Build configurations | `Cargo.toml` fragments, `tsconfig.json` | Shared build settings |

### What to Avoid

**Do not include:**

- **Secrets or credentials**: API keys, tokens, passwords
- **Environment-specific paths**: `/Users/yourname/...`
- **Large binary files**: Images, compiled artifacts
- **Generated files**: `target/`, `node_modules/`, `dist/`
- **Personal IDE settings**: User-specific configurations
- **Repository-specific data**: Git history, issues, PRs

### Handling Sensitive Data

If your templates reference secrets, use placeholders:

```yaml
# Good: Reference secrets by name
env:
  API_KEY: ${{ secrets.API_KEY }}

# Bad: Never include actual secrets
env:
  API_KEY: sk-1234567890abcdef
```

Document required secrets in your README:

```markdown
## Required Secrets

Consumers must configure these repository secrets:

| Secret | Description |
|--------|-------------|
| `API_KEY` | API key for external service |
| `DEPLOY_TOKEN` | Token for deployment |
```

## Next Steps

- [Configuration Reference](configuration.md) - All operators for consumers
- [Recipes](recipes.md) - Common inheritance patterns
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
