# GitHub Action

The common-repo GitHub Action checks for upstream updates and creates pull requests with the changes.

## Quick Start

Add this workflow to your repository:

```yaml
# .github/workflows/upstream-sync.yml
name: Sync Upstream Configuration

on:
  schedule:
    - cron: '0 9 * * 1'  # Weekly on Monday at 9am UTC
  workflow_dispatch:     # Allow manual trigger

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

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `token` | GitHub token for creating PRs | `github.token` |
| `config-path` | Path to .common-repo.yaml | `.common-repo.yaml` |
| `version` | Pin common-repo version (e.g., `v0.27.0`) | `latest` |
| `update-strategy` | `compatible` (minor/patch) or `latest` (all) | `compatible` |
| `force-sync` | Run apply even without version updates | `false` |
| `pr-title` | Pull request title | `chore(deps): update common-repo inherited files` |
| `pr-branch` | Branch name for the PR | `chore/upstream-sync` |
| `commit-message` | Commit message | `chore(deps): update common-repo files` |
| `dry-run` | Check without creating PR | `false` |

## Outputs

| Output | Description |
|--------|-------------|
| `has-updates` | `true` if version updates were available |
| `has-changes` | `true` if any file changes were made |
| `pr-url` | URL of created/updated PR |
| `pr-number` | PR number |
| `files-changed` | JSON array of changed files |

## Examples

### Update to latest versions (including breaking changes)

```yaml
- uses: common-repo/common-repo@v1
  with:
    update-strategy: latest
```

### Pin common-repo version for reproducibility

```yaml
- uses: common-repo/common-repo@v1
  with:
    version: v0.27.0
```

### Force re-sync (even without version updates)

Useful if someone manually edited a managed file:

```yaml
- uses: common-repo/common-repo@v1
  with:
    force-sync: true
```

### Dry-run to check for updates without creating PR

```yaml
- uses: common-repo/common-repo@v1
  id: check
  with:
    dry-run: true

- run: echo "Updates available: ${{ steps.check.outputs.has-updates }}"
```

### Custom PR settings

```yaml
- uses: common-repo/common-repo@v1
  with:
    pr-title: 'chore: sync shared configs'
    pr-branch: 'automation/config-sync'
    commit-message: 'chore: update shared configuration files'
```

## Requirements

### Permissions

The workflow needs these permissions:

```yaml
permissions:
  contents: write       # Push to PR branch
  pull-requests: write  # Create/update PRs
```

### Private inherited repos

If your `.common-repo.yaml` references private repositories, you need a Personal Access Token (PAT) with `repo` scope:

```yaml
- uses: common-repo/common-repo@v1
  with:
    token: ${{ secrets.PAT_TOKEN }}
```

### Self-hosted runners

The action requires `curl`, `git`, `gh` (GitHub CLI), and `jq`. These are pre-installed on GitHub-hosted runners but may need to be installed on self-hosted runners.

## How It Works

1. Installs the `common-repo` binary
2. Runs `common-repo check --updates` to detect available updates
3. If updates exist (or `force-sync: true`):
   - Runs `common-repo update` to bump refs in `.common-repo.yaml`
   - Runs `common-repo apply` to regenerate files
4. Creates or updates a PR with all changes

The action adds the `dependencies` label to PRs if that label exists in your repository.
