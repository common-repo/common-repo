# common-repo

[![CI](https://github.com/common-repo/common-repo/actions/workflows/ci.yml/badge.svg)](https://github.com/common-repo/common-repo/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE.md)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://common-repo.github.io/common-repo/)

Every repository accumulates the same files: CI pipelines, linter configs, pre-commit hooks, editor settings, Dockerfiles. These get copied between projects by hand. When something needs to change — a security fix, a new lint rule, a CI image update — there is no way to propagate it. Every copy must be found and patched individually.

common-repo fixes this by treating configuration files as dependencies. You declare which upstream repositories to inherit from, pin versions, and define how files merge. common-repo fetches everything, applies your operations, and writes the result. When upstreams publish updates, you get a diff and a pull request.

## Beyond scaffolding

Cookiecutter and copier generate files once. After that, you're on your own — no mechanism exists to push a security fix or a new lint rule back to every project that was generated. Configuration fossilizes on day one.

common-repo keeps configuration current. It tracks upstream changes continuously. When a dependency updates, you see the diff and decide whether to pull it in.

## Composing and inheriting configs

You can pull from multiple focused repositories — Rust tooling from one, semantic versioning config from another, Python linting from a third — and the results merge without conflicts. common-repo merges at the structural level (YAML keys, JSON objects, TOML tables, INI sections, Markdown headings), not file paths. Add a CI job to an existing workflow or a dependency to `Cargo.toml` without replacing the whole file.

Upstreams can themselves inherit from other upstreams, forming chains. A team-level Rust config extends a company-wide base, which extends a community standard. A change at any level propagates down the chain. `common-repo tree` shows you exactly how a repository's inheritance is structured.

Every upstream is pinned to a git ref. `common-repo check --updates` reports what's newer. `common-repo update` bumps compatible versions. `--latest` includes breaking changes when you're ready.

common-repo is not a code generator or a template engine. It copies, filters, renames, and merges files. If you need conditional logic or code scaffolding, this is the wrong tool.

## How it works

Add a `.common-repo.yaml` to your repository listing upstreams and operations:

```yaml
- repo:
    url: https://github.com/your-org/shared-configs
    ref: v1.0.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
```

Run `common-repo apply`. The tool clones the upstream (or loads it from cache), filters files through `include`/`exclude` rules, applies merge operations, and writes the result into the working tree. Upstreams can reference their own upstreams, forming inheritance chains. The resolution order is deterministic — the same config always produces the same output. Repositories are cached after first fetch, so repeated runs skip the network entirely.

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

A `.common-repo.yaml` is a list of operations applied in order. Two upstreams with filtering, renaming, and merging:

```yaml
# Pull CI and pre-commit configs from shared-configs
- repo:
    url: https://github.com/your-org/shared-configs
    ref: v1.0.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
      - exclude: [".git/**"]

# Pull CI workflows from ci-templates, renaming one
- repo:
    url: https://github.com/your-org/ci-templates
    ref: main
    with:
      - include: [".github/workflows/*.yml"]
      - rename:
          ".github/workflows/ci.yml": ".github/workflows/build.yml"
```

Operations within `with:` apply only to that upstream's files before merging.

### Merging structured files

common-repo can deep-merge into existing files rather than replacing them. Supported formats: YAML, JSON, TOML, INI, and Markdown ([full docs](docs/src/configuration.md)).

Add jobs to a CI workflow:
```yaml
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```

Add scripts to package.json:
```yaml
- json:
    source: scripts.json
    dest: package.json
    path: scripts
```

Add dependencies to Cargo.toml:
```yaml
- toml:
    source: common-deps.toml
    dest: Cargo.toml
    path: dependencies
```

INI and Markdown merges work similarly — INI targets sections, Markdown targets headings:
```yaml
- ini:
    source: editor-rules.ini
    dest: .editorconfig
    section: "*"

- markdown:
    source: contributing-section.md
    dest: README.md
    section: "## Contributing"
    create-section: true
```

Merge operations support `auto-merge:` when source and destination share the same filename, and `defer:` when upstream repos want consumers to inherit their merge rules.

### The `self:` operator

An upstream repo may have its own tooling — CI scripts, test fixtures — that consumers should not inherit. `self:` runs an isolated pipeline whose output stays local:

```yaml
- self:
    - repo:
        url: https://github.com/org/ci-tooling
        ref: v2.0.0
- include: ["src/**"]  # only this is visible to consumers
```

When another repository inherits from this one, common-repo skips the `self:` block.

## Updates and automation

### Checking for updates

```bash
common-repo check --updates   # See available updates
common-repo update            # Update to compatible versions (minor/patch)
common-repo update --latest   # Include breaking changes (major versions)
```

### Automated updates with GitHub Actions

The common-repo GitHub Action creates PRs when upstream configurations change:

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

Full documentation is available at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/).

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/src/getting-started.md) | Installation and first steps |
| [Configuration](docs/src/configuration.md) | All operators and options |
| [CLI Reference](docs/src/cli.md) | Command documentation |
| [Recipes](docs/src/recipes.md) | Configuration examples |
| [Troubleshooting](docs/src/troubleshooting.md) | Common issues and solutions |
| [Authoring Upstream Repos](docs/src/authoring-upstream-repos.md) | Create your own upstream repos |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and guidelines.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE.md).
