# common-repo

Manage repository configuration files as dependencies. Define inheritance in `.common-repo.yaml`, pull files from multiple Git repositories, and merge them with version pinning.

## Why common-repo?

Modern software repositories require extensive configuration infrastructure: CI/CD pipelines, pre-commit hooks, linters, formatters, and countless dotfiles. Managing these across multiple projects typically means:

- **Manual copy-paste** - Configuration files copied between projects lead to inconsistency and drift
- **No versioning** - Unlike source dependencies, configs aren't semantically versioned or tracked
- **Difficult updates** - No automated way to propagate best practices across repositories
- **No inheritance** - Can't easily extend standard configurations

common-repo treats repository configuration as software dependencies.

Configuration files become:
- **Semantically versioned** - Track exactly which version you're using
- **Automatically updateable** - Detect outdated configs and upgrade deterministically
- **Composable** - Pull from multiple sources and merge intelligently
- **Inheritable** - Build upon standards that themselves extend other standards

## Quick Start

Install common-repo:

```bash
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
```

Create `.common-repo.yaml` in your repository:

```yaml
- repo:
    url: https://github.com/your-org/shared-configs
    ref: v1.0.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
```

Apply configuration:

```bash
common-repo ls      # List files that would be created
common-repo diff    # Preview changes
common-repo apply   # Apply configuration
```

## Next Steps

- [Getting Started](getting-started.md) - Installation and first steps
- [Configuration Reference](configuration.md) - All operators and options
- [CLI Reference](cli.md) - Command documentation
- [Recipes](recipes.md) - Common patterns
