# Recipes and Patterns

Practical examples for common use cases.

## Standard Project Setups

### Rust CLI Project

A complete Rust CLI project with CI, pre-commit hooks, and conventional commits.

```yaml
# .common-repo.yaml

# Inherit base Rust CLI configuration
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v2.0.0
    with:
      - include: ["**/*"]
      - exclude: [".git/**", "target/**", "src/**"]

# Add pre-commit hooks for Rust
- repo:
    url: https://github.com/common-repo/pre-commit-rust
    ref: v1.0.0
    with:
      - include: [".pre-commit-config.yaml"]

# Project-specific variables
- template-vars:
    project_name: ${PROJECT_NAME:-my-cli}
    author: ${AUTHOR:-Your Name}
    description: A command-line tool

# Mark config files as templates
- template:
    - Cargo.toml
    - README.md
    - .github/workflows/*.yml

# Require essential tools
- tools:
    - rustc: ">=1.70"
    - cargo: "*"
    - pre-commit: ">=3.0"
```

### Python Project with UV

Modern Python project using UV for dependency management.

```yaml
# .common-repo.yaml

# Base Python project structure
- repo:
    url: https://github.com/common-repo/python-uv
    ref: v1.0.0
    with:
      - include: ["**/*"]
      - exclude: [".git/**", "__pycache__/**", ".venv/**"]

# Pre-commit hooks for Python
- repo:
    url: https://github.com/common-repo/pre-commit-python
    ref: v1.5.0
    with:
      - include: [".pre-commit-config.yaml"]

# Variables
- template-vars:
    project_name: ${PROJECT_NAME:-my-python-project}
    python_version: "3.11"
    author: ${AUTHOR:-Your Name}

- template:
    - pyproject.toml
    - README.md

- tools:
    - python: ">=3.11"
    - uv: "*"
```

### Node.js/TypeScript Project

TypeScript project with ESLint, Prettier, and GitHub Actions.

```yaml
# .common-repo.yaml

# TypeScript base configuration
- repo:
    url: https://github.com/common-repo/typescript
    ref: v3.0.0
    with:
      - include:
          - tsconfig.json
          - .eslintrc.json
          - .prettierrc
          - .github/**

# Add package.json dependencies
- json:
    source: dev-deps.json
    dest: package.json
    path: devDependencies

- template-vars:
    project_name: ${npm_package_name:-my-project}
    node_version: "20"

- tools:
    - node: ">=20"
    - npm: ">=10"
```

## CI/CD Patterns

### GitHub Actions Workflow Inheritance

Inherit and customize GitHub Actions workflows.

```yaml
# .common-repo.yaml

# Base CI workflows
- repo:
    url: https://github.com/your-org/ci-templates
    ref: v2.0.0
    with:
      - include: [".github/**"]

# Add project-specific jobs to CI
- yaml:
    source: local-ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true

# Customize workflow variables
- template-vars:
    default_branch: main
    node_version: "20"
    deploy_environment: production

- template:
    - ".github/workflows/*.yml"
```

**local-ci-jobs.yml:**
```yaml
integration-tests:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run integration tests
      run: ./scripts/integration-test.sh
```

### Multi-Stage Deployment

Different configurations for different environments.

```yaml
# .common-repo.yaml

# Base deployment configuration
- repo:
    url: https://github.com/your-org/deploy-configs
    ref: v1.0.0
    path: kubernetes  # Only kubernetes configs

# Environment-specific overrides
- yaml:
    source: env/production.yml
    dest: kubernetes/deployment.yml
    path: spec.template.spec

- template-vars:
    environment: ${DEPLOY_ENV:-staging}
    replicas: ${REPLICAS:-3}
    image_tag: ${IMAGE_TAG:-latest}

- template:
    - "kubernetes/*.yml"
```

## Organization Standards

### Company-Wide Defaults

Create a base configuration that all projects inherit.

**In your org's base repo (`github.com/your-org/base-config`):**

```yaml
# .common-repo.yaml (this repo exports these files)

- include:
    - .editorconfig
    - .gitignore
    - .github/CODEOWNERS
    - .github/PULL_REQUEST_TEMPLATE.md
    - .github/ISSUE_TEMPLATE/**
    - LICENSE

- template-vars:
    org_name: Your Organization
    support_email: support@your-org.com
```

**In each project:**

```yaml
# .common-repo.yaml

# Inherit org-wide defaults
- repo:
    url: https://github.com/your-org/base-config
    ref: v1.0.0

# Add language-specific configuration
- repo:
    url: https://github.com/your-org/rust-standards
    ref: v2.0.0

# Project-specific customization
- template-vars:
    project_name: my-service
    team: platform
```

### Security Configurations

Standardize security tooling across projects.

```yaml
# .common-repo.yaml

# Security scanning configuration
- repo:
    url: https://github.com/your-org/security-configs
    ref: v1.2.0
    with:
      - include:
          - .github/workflows/security.yml
          - .snyk
          - .trivyignore

# Merge security checks into existing CI
- yaml:
    source: security-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```

## Monorepo Patterns

### Shared Configuration Across Packages

Use path filtering to share configs in a monorepo.

```yaml
# packages/web/.common-repo.yaml

# Inherit from monorepo root configs
- repo:
    url: https://github.com/your-org/your-monorepo
    ref: main
    path: shared/web-configs
    with:
      - include: ["**/*"]

# Package-specific settings
- template-vars:
    package_name: web
    port: "3000"
```

### Multiple Config Repositories

Compose from multiple specialized config repos.

```yaml
# .common-repo.yaml

# Linting standards
- repo:
    url: https://github.com/your-org/lint-configs
    ref: v2.0.0
    with:
      - include: [".eslintrc.json", ".prettierrc"]

# Testing configuration
- repo:
    url: https://github.com/your-org/test-configs
    ref: v1.5.0
    with:
      - include: ["jest.config.js", "vitest.config.ts"]

# CI/CD templates
- repo:
    url: https://github.com/your-org/ci-configs
    ref: v3.0.0
    with:
      - include: [".github/**"]
```

## Configuration Merging

### Extending package.json

Add dependencies without overwriting the whole file.

```yaml
# .common-repo.yaml

# Base project
- repo:
    url: https://github.com/your-org/node-base
    ref: v1.0.0

# Add shared dev dependencies
- json:
    source: shared-dev-deps.json
    dest: package.json
    path: devDependencies

# Add shared scripts
- json:
    source: shared-scripts.json
    dest: package.json
    path: scripts
    append: true
```

**shared-dev-deps.json:**
```json
{
  "eslint": "^8.0.0",
  "prettier": "^3.0.0",
  "typescript": "^5.0.0"
}
```

### Merging Cargo.toml Dependencies

Add common Rust dependencies.

```yaml
# .common-repo.yaml

- repo:
    url: https://github.com/your-org/rust-base
    ref: v1.0.0

# Add logging dependencies
- toml:
    source: logging-deps.toml
    dest: Cargo.toml
    path: dependencies

# Add dev dependencies
- toml:
    source: dev-deps.toml
    dest: Cargo.toml
    path: dev-dependencies
```

**logging-deps.toml:**
```toml
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Extending Pre-commit Hooks

Add hooks to an existing pre-commit configuration.

```yaml
# .common-repo.yaml

# Base pre-commit config
- repo:
    url: https://github.com/your-org/pre-commit-base
    ref: v1.0.0
    with:
      - include: [".pre-commit-config.yaml"]

# Add additional hooks
- yaml:
    source: extra-hooks.yml
    dest: .pre-commit-config.yaml
    path: repos
    append: true
```

**extra-hooks.yml:**
```yaml
- repo: local
  hooks:
    - id: custom-check
      name: Custom Check
      entry: ./scripts/check.sh
      language: script
```

## Advanced Patterns

### Conditional Configuration via Environment

Use environment variables for conditional behavior.

```yaml
# .common-repo.yaml

- repo:
    url: https://github.com/your-org/base-config
    ref: v1.0.0

- template-vars:
    # Default to development settings
    log_level: ${LOG_LEVEL:-debug}
    enable_metrics: ${ENABLE_METRICS:-false}

    # CI-specific overrides
    ci_mode: ${CI:-false}

- template:
    - config/*.yml
```

### Version Pinning Strategy

Pin specific versions while allowing updates.

```yaml
# .common-repo.yaml

# Pin to specific major version for stability
- repo:
    url: https://github.com/your-org/stable-configs
    ref: v2.0.0  # Update manually for major versions

# Use branch for frequently updated configs
- repo:
    url: https://github.com/your-org/evolving-configs
    ref: main  # Always get latest (use with caution)

# Use commit SHA for maximum reproducibility
- repo:
    url: https://github.com/your-org/critical-configs
    ref: abc123def456  # Exact commit
```

### Template File Patterns

Use template files for customization.

**In config repo:**
```
templates/
  Cargo.toml.template
  README.md.template
```

**In .common-repo.yaml:**
```yaml
- repo:
    url: https://github.com/your-org/rust-templates
    ref: v1.0.0
    with:
      - include: ["templates/**"]
      - rename:
          - "templates/(.+)\\.template$": "%[1]s"

- template-vars:
    project_name: my-project
    description: A great project
    license: MIT

- template:
    - Cargo.toml
    - README.md
```

## Debugging Tips

### Verbose Output

```bash
# See detailed processing (debug level)
common-repo apply --verbose

# Maximum verbosity (trace level)
common-repo apply --verbose --verbose
```

### Inspect Before Applying

```bash
# List what would be created
common-repo ls -l

# See the diff
common-repo diff

# Dry run
common-repo apply --dry-run
```

### Check Inheritance

```bash
# View inheritance tree
common-repo tree

# Get configuration overview
common-repo info
```

### Validate Configuration

```bash
# Check syntax
common-repo validate

# Also check repo accessibility
common-repo validate --check-repos
```
