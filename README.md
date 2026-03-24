# common-repo

[![CI](https://github.com/common-repo/common-repo/actions/workflows/ci.yml/badge.svg)](https://github.com/common-repo/common-repo/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE.md)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://common-repo.github.io/common-repo/)

Declare configuration files as versioned dependencies. Pull from multiple Git repositories, merge structured files at the key level, and track upstream updates with semver.

```yaml
# .common-repo.yaml
- repo:
    url: https://github.com/acme-corp/platform-defaults
    ref: v2.1.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]

- repo:
    url: https://github.com/acme-corp/rust-tooling
    ref: v1.4.0
    with:
      - include: ["rustfmt.toml", "clippy.toml", ".cargo/**"]

# Merge CI jobs into an existing workflow instead of replacing it
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```

```bash
common-repo diff    # preview changes
common-repo apply   # write files
```

Upstreams are pinned to git refs and cached locally. The same `.common-repo.yaml` always produces the same output. Upstreams can reference their own upstreams — `common-repo tree` shows the full chain.

## Install

### Shell installer (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
```

The installer creates both `common-repo` and the alias `cr`. To skip the alias: `SKIP_ALIAS=1 sh -s < install.sh`

### cargo-binstall (pre-built binary)

```bash
cargo binstall common-repo
```

### From source (latest development)

```bash
cargo install --git https://github.com/common-repo/common-repo
```

### GitHub Releases

Download the latest release for your platform from [GitHub Releases](https://github.com/common-repo/common-repo/releases) and add it to your PATH.

### Platform notes

- **Linux/macOS**: Shell installer auto-detects architecture (x86_64, aarch64)
- **Windows**: Use `cargo install` or download from GitHub Releases
- **Nix**: `nix run github:common-repo/common-repo` (flake available)

### Minimum Supported Rust Version (MSRV)

This project supports Rust 1.85.0 and later. MSRV updates are considered non-breaking changes.

## Quick start

```bash
# Initialize interactively — add repos, auto-detect versions
common-repo init

# Or initialize from an existing repo
common-repo init https://github.com/your-org/shared-configs
common-repo init your-org/shared-configs  # GitHub shorthand

# Add more repos to an existing config
common-repo add your-org/another-repo

# Preview what would change, then apply
common-repo diff
common-repo apply
```

## Configuration

A `.common-repo.yaml` is a list of operations applied in order:

```yaml
- repo:
    url: https://github.com/acme-corp/platform-defaults
    ref: v2.1.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
      - exclude: [".github/CODEOWNERS"]
      - rename:
          ".github/workflows/ci.yml": ".github/workflows/build.yml"
```

Operations within `with:` apply only to that upstream's files before merging.

### File merging

Merge operations write into existing files at a specific path instead of replacing them. Supported formats: YAML, JSON, TOML, INI, and Markdown ([full docs](docs/src/configuration.md)).

```yaml
# Merge into .jobs in an existing workflow
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true

# Merge into .scripts in package.json
- json:
    source: scripts.json
    dest: package.json
    path: scripts

# Merge into [dependencies] in Cargo.toml
- toml:
    source: common-deps.toml
    dest: Cargo.toml
    path: dependencies

# Merge into a section in .editorconfig
- ini:
    source: editor-rules.ini
    dest: .editorconfig
    section: "*"

# Insert or replace a heading in README.md
- markdown:
    source: contributing-section.md
    dest: README.md
    section: "## Contributing"
    create-section: true
```

`auto-merge:` merges files that share the same name on both sides. `defer:` marks merge operations for consumers to inherit.

### Inheritance

Upstreams can have their own `.common-repo.yaml` files referencing other upstreams:

```
your-project
├── acme-corp/platform-defaults v2.1.0
│   └── common-repo/ci-base v1.0.0
└── acme-corp/rust-tooling v1.4.0
    └── common-repo/rust-base v3.2.1
```

Resolution is depth-first post-order: ancestors are applied before their parents, parents before the local repo. Files with the same path are last-write-wins.

### The `self:` operator

`self:` runs an isolated pipeline whose output stays local — it is stripped when other repos inherit from this one:

```yaml
- self:
    - repo:
        url: https://github.com/org/ci-tooling
        ref: v2.0.0
- include: ["src/**"]  # only this is visible to consumers
```

## Updates

```bash
common-repo check --updates   # list available updates
common-repo update            # bump compatible versions (minor/patch)
common-repo update --latest   # include breaking changes (major)
```

### GitHub Actions

Automate upstream sync with the common-repo action:

```yaml
# .github/workflows/upstream-sync.yml
name: Sync Upstream Configuration

on:
  schedule:
    - cron: '0 9 * * 1'  # Weekly on Monday
  workflow_dispatch:

permissions:
  contents: write
  pull-requests: write

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: common-repo/common-repo@v1
```

See [GitHub Action documentation](docs/src/github-action.md) for all options.

## Commands

| Command | Description |
|---------|-------------|
| `init` | Create a new `.common-repo.yaml` |
| `add` | Add a repository to existing config |
| `apply` | Apply configuration (fetch, merge, write) |
| `diff` | Preview changes before applying |
| `ls` | List files that would be created |
| `tree` | Show the inheritance tree |
| `check` | Validate configuration, check for updates |
| `update` | Update refs to newer versions |
| `validate` | Validate configuration syntax |
| `info` | Show configuration overview |
| `cache` | Manage the local repository cache |
| `completions` | Generate shell completion scripts |

## Documentation

Full documentation at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/).

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/src/getting-started.md) | Installation and first steps |
| [Configuration](docs/src/configuration.md) | All operators and options |
| [CLI Reference](docs/src/cli.md) | Command documentation |
| [Recipes](docs/src/recipes.md) | Configuration examples |
| [Troubleshooting](docs/src/troubleshooting.md) | Fixes for common issues |
| [Authoring Upstream Repos](docs/src/authoring-upstream-repos.md) | Publishing repos for others to inherit |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and guidelines.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE.md).
