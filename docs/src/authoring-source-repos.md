# Authoring Source Repositories

This guide explains how to create and maintain source repositories that other projects can inherit from using common-repo.

## What is a Source Repository?

A **source repository** contains configuration files, templates, and standards that other repositories (consumers) can inherit. Think of it as a library of reusable project configurations.

**Source repository** vs **consumer repository**:

| Aspect | Source Repository | Consumer Repository |
|--------|-------------------|---------------------|
| Purpose | Provides files for others to inherit | Uses files from source repos |
| Audience | Maintainers of shared standards | Individual projects |
| Config file | Optional `.common-repo.yaml` | Required `.common-repo.yaml` |
| Versioning | Semantic versioning with Git tags | References source repo versions |

**Common use cases for source repositories:**

- Organization-wide coding standards (linting, formatting rules)
- CI/CD workflow templates (GitHub Actions, GitLab CI)
- Project scaffolding and boilerplate
- Security policies and configurations
- Documentation templates

## Getting Started

### Minimal Source Repository Structure

A source repository can be as simple as a directory with files to share:

```
my-source-repo/
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
    url: https://github.com/your-org/my-source-repo
    ref: v1.0.0
```

### Optional: Configuration in Source Repos

Source repositories can include their own `.common-repo.yaml` to inherit from other sources, creating an inheritance chain:

```
org-base/          # Base standards
  └── rust-base/   # Rust-specific (inherits org-base)
      └── my-app/  # Consumer (inherits rust-base)
```

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
       url: https://github.com/your-org/my-source-repo
       ref: v1.0.0
   ```

## File Organization

### Repository Root vs Subdirectory

By default, consumers inherit from the repository root. Use the `path` option to expose a subdirectory:

```
my-source-repo/
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
    url: https://github.com/your-org/my-source-repo
    ref: v1.0.0
    path: templates/rust  # Only inherit from this subdirectory
```

### Organizing Files by Concern

Group related files together to make selective inheritance easier:

```
source-repo/
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
    url: https://github.com/your-org/source-repo
    ref: v1.0.0
    with:
      - include: ["ci/**", "quality/**"]
```

### Dotfiles and Hidden Files

Dotfiles (files starting with `.`) are included by default. Structure them naturally:

```
source-repo/
├── .github/
│   ├── workflows/
│   │   └── ci.yml
│   └── dependabot.yml
├── .pre-commit-config.yaml
├── .editorconfig
└── .gitignore
```

Files you typically **should not** include in source repos:
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
# .github/workflows/ci.yml (in source repo)
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

### Environment Variable Defaults

Variables can reference environment variables as defaults:

```yaml
# Consumer's .common-repo.yaml
- template-vars:
    project_name: ${PROJECT_NAME:-my-project}
    ci_timeout: ${CI_TIMEOUT:-30}
```

## Versioning and Releases

### Semantic Versioning

Follow [semantic versioning](https://semver.org/) for source repositories:

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

### Git Tagging Best Practices

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

## Testing Your Source Repository

### Testing Locally

Test your source repo against a local consumer before publishing:

```bash
# In consumer repository
cd my-consumer-project

# Reference local source repo instead of remote
# Edit .common-repo.yaml temporarily:
# - repo:
#     url: /path/to/local/source-repo
#     ref: HEAD

# Or use a file:// URL
# - repo:
#     url: file:///absolute/path/to/source-repo
#     ref: HEAD

common-repo ls      # Verify expected files
common-repo diff    # Check for issues
common-repo apply --dry-run
```

### Creating a Test Consumer

Maintain a test consumer repository or directory:

```
source-repo/
├── .github/
├── templates/
├── tests/
│   └── test-consumer/    # Test consumer project
│       ├── .common-repo.yaml
│       └── validate.sh
└── README.md
```

The test consumer validates that your source repo works correctly:

```yaml
# tests/test-consumer/.common-repo.yaml
- repo:
    url: ../..  # Relative path to source repo root
    ref: HEAD
    with:
      - include: ["templates/rust/**"]
```

### CI Testing Strategies

Add CI workflows that validate your source repository:

```yaml
# .github/workflows/test.yml
name: Test Source Repo

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

### Designing for Multi-Source Inheritance

Consumers often inherit from multiple source repositories. Design yours to work alongside others:

```yaml
# Consumer inheriting from multiple sources
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

When multiple sources provide similar files, conflicts can occur. Strategies to avoid this:

**Use specific subdirectories:**
```
# Instead of:
source-repo/
└── ci.yml

# Use:
source-repo/
└── .github/workflows/
    └── source-name-ci.yml   # Prefixed to avoid conflicts
```

**Document file ownership:**
```markdown
## Files Provided

This source repository provides:
- `.github/workflows/lint.yml` - Linting workflow
- `.github/workflows/test.yml` - Test workflow

If you inherit from other sources, ensure they don't provide the same files,
or use `exclude` to skip conflicting files.
```

### Namespace Considerations

Consider prefixing files with your source repo's purpose:

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

### Good Candidates for Source Repos

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
