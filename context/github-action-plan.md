# GitHub Action: Upstream Sync

## Overview

A composite GitHub Action at the repository root that allows downstream repositories (those with a `.common-repo.yaml`) to automatically receive updates from their inherited configuration sources.

## Context from Project Design

From `context/purpose.md`, `context/design.md`, and `README.md`:

- **common-repo treats repository configuration files as software dependencies**
- Repositories define inheritance in `.common-repo.yaml` with pinned git refs
- The tool can detect when inherited repos have newer versions available
- Key commands: `check --updates`, `update`, `apply`, `diff`
- **Binary distribution**: Shell installer at `install.sh` handles cross-platform installation

## How It Works

The action leverages existing CLI commands:

1. **Install** - Use existing `install.sh` to get the binary
2. **`common-repo check --updates`** - Detects if any inherited repos have newer versions
3. **`common-repo update`** - Updates refs in `.common-repo.yaml` to newer versions
4. **`common-repo apply`** - Regenerates files from the updated configuration
5. **`gh pr create`** - Creates PR with all changes

## Design Decisions (Resolved)

| Decision | Choice |
|----------|--------|
| Binary distribution | Use existing `install.sh` |
| PR creation | Native `gh pr create` |
| Update + Apply | Yes - PR shows actual file changes |

## Action Structure

```
action.yml              # Composite action definition (repo root)
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `token` | GitHub token for creating PRs | No | `${{ github.token }}` |
| `config-path` | Path to .common-repo.yaml | No | `.common-repo.yaml` |
| `version` | Pin `common-repo` binary version (e.g., `v0.27.0`) | No | `latest` |
| `update-strategy` | How to select versions: `compatible`, `latest` | No | `compatible` |
| `force-sync` | Run apply even without version updates (handles drift) | No | `false` |
| `pr-title` | Title for the created PR | No | `chore(deps): update common-repo inherited files` |
| `pr-branch` | Branch name for the PR | No | `chore/upstream-sync` |
| `commit-message` | Commit message | No | `chore(deps): update common-repo files` |
| `dry-run` | Check for updates without creating PR | No | `false` |

## Outputs

| Output | Description |
|--------|-------------|
| `has-updates` | `true` if updates were available |
| `pr-url` | URL of created/updated PR (empty if no changes) |
| `pr-number` | PR number (empty if no changes) |
| `updated-repos` | JSON array of repos that were updated |
| `files-changed` | JSON array of files that changed |

## Implementation

```yaml
name: 'common-repo sync'
description: 'Update inherited configuration files from upstream common-repo sources'
author: 'common-repo'

inputs:
  token:
    description: 'GitHub token for creating PRs'
    required: false
    default: ${{ github.token }}
  config-path:
    description: 'Path to .common-repo.yaml'
    required: false
    default: '.common-repo.yaml'
  version:
    description: 'Pin common-repo binary version (e.g., v0.27.0). Use "latest" for most recent.'
    required: false
    default: 'latest'
  update-strategy:
    description: 'Version strategy: compatible (minor/patch) or latest (including major)'
    required: false
    default: 'compatible'
  force-sync:
    description: 'Run apply even without version updates (handles config drift)'
    required: false
    default: 'false'
  pr-title:
    description: 'Pull request title'
    required: false
    default: 'chore(deps): update common-repo inherited files'
  pr-branch:
    description: 'Branch name for the PR'
    required: false
    default: 'chore/upstream-sync'
  commit-message:
    description: 'Commit message'
    required: false
    default: 'chore(deps): update common-repo files'
  dry-run:
    description: 'Check for updates without creating PR'
    required: false
    default: 'false'

outputs:
  has-updates:
    description: 'Whether version updates were available'
    value: ${{ steps.check.outputs.has-updates }}
  has-changes:
    description: 'Whether any file changes were made'
    value: ${{ steps.apply.outputs.has-changes }}
  pr-url:
    description: 'URL of created/updated PR'
    value: ${{ steps.pr.outputs.pr-url }}
  pr-number:
    description: 'PR number'
    value: ${{ steps.pr.outputs.pr-number }}
  updated-repos:
    description: 'JSON array of updated repos'
    value: ${{ steps.check.outputs.updated-repos }}
  files-changed:
    description: 'JSON array of changed files'
    value: ${{ steps.apply.outputs.files-changed }}

runs:
  using: composite
  steps:
    - name: Validate config exists
      shell: bash
      run: |
        if [ ! -f "${{ inputs.config-path }}" ]; then
          echo "::error::Config file not found: ${{ inputs.config-path }}"
          exit 1
        fi

    - name: Install common-repo
      shell: bash
      env:
        VERSION: ${{ inputs.version != 'latest' && inputs.version || '' }}
      run: |
        curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
        echo "$HOME/.local/bin" >> $GITHUB_PATH

    - name: Check for updates
      id: check
      shell: bash
      run: |
        # Check for available updates
        if common-repo check --updates --config "${{ inputs.config-path }}" 2>&1 | tee /tmp/updates.txt | grep -q "updates available"; then
          echo "has-updates=true" >> $GITHUB_OUTPUT
        else
          echo "has-updates=false" >> $GITHUB_OUTPUT
        fi

    - name: Update configuration
      if: steps.check.outputs.has-updates == 'true'
      shell: bash
      run: |
        strategy_flag=""
        if [ "${{ inputs.update-strategy }}" = "latest" ]; then
          strategy_flag="--latest"
        else
          strategy_flag="--compatible"
        fi
        common-repo update --config "${{ inputs.config-path }}" $strategy_flag --yes

    - name: Apply configuration
      id: apply
      if: steps.check.outputs.has-updates == 'true' || inputs.force-sync == 'true'
      shell: bash
      run: |
        common-repo apply --config "${{ inputs.config-path }}"

        # Check for any changes (staged, unstaged, or untracked)
        if git diff --quiet && git diff --cached --quiet && [ -z "$(git status --porcelain)" ]; then
          echo "has-changes=false" >> $GITHUB_OUTPUT
          echo "files-changed=[]" >> $GITHUB_OUTPUT
        else
          echo "has-changes=true" >> $GITHUB_OUTPUT
          # Capture all changed/new files
          files=$(git status --porcelain | awk '{print $2}' | jq -R -s -c 'split("\n") | map(select(length > 0))')
          echo "files-changed=$files" >> $GITHUB_OUTPUT
        fi

    - name: Create Pull Request
      id: pr
      if: steps.apply.outputs.has-changes == 'true' && inputs.dry-run != 'true'
      shell: bash
      env:
        GH_TOKEN: ${{ inputs.token }}
      run: |
        # Configure git
        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"

        # Create branch and commit
        git checkout -B "${{ inputs.pr-branch }}"
        git add -A
        git commit -m "${{ inputs.commit-message }}"
        git push -u origin "${{ inputs.pr-branch }}" --force

        # Check if dependencies label exists
        label_flag=""
        if gh label list --json name --jq '.[].name' | grep -q "^dependencies$"; then
          label_flag="--label dependencies"
        fi

        # Create or update PR
        pr_url=$(gh pr list --head "${{ inputs.pr-branch }}" --json url --jq '.[0].url')
        if [ -z "$pr_url" ]; then
          pr_url=$(gh pr create \
            --title "${{ inputs.pr-title }}" \
            --body "## Upstream Configuration Updates

        This PR updates inherited configuration files from common-repo sources.

        ### Changed Files
        ${{ steps.apply.outputs.files-changed }}

        ---
        *Generated by [common-repo](https://github.com/common-repo/common-repo)*" \
            --head "${{ inputs.pr-branch }}" \
            $label_flag)
        fi

        echo "pr-url=$pr_url" >> $GITHUB_OUTPUT
        pr_number=$(echo "$pr_url" | grep -oE '[0-9]+$')
        echo "pr-number=$pr_number" >> $GITHUB_OUTPUT
```

## Usage Example

```yaml
# .github/workflows/upstream-sync.yml
name: Sync Upstream Configuration

on:
  schedule:
    - cron: '0 9 * * 1'  # Weekly on Monday at 9am
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
        with:
          update-strategy: compatible
```

## Edge Cases & Missing Items

### Must Handle

| Issue | Current State | Resolution |
|-------|---------------|------------|
| No `.common-repo.yaml` | Would fail with unclear error | Check file exists, fail with clear message |
| No changes after apply | `git commit` fails on empty | Check `git diff --quiet` before committing |
| Config file exists but no `repo:` entries | Unclear behavior | Skip gracefully, set `has-updates=false` |
| Private inherited repos | Token lacks access | Document: PAT needed for private repos |
| `install.sh` fails | Action fails | Add error handling, suggest manual install |

### Required Permissions

Document in usage example:
```yaml
permissions:
  contents: write       # Push to PR branch
  pull-requests: write  # Create/update PRs
```

For private inherited repos, users need a PAT with `repo` scope.

### Tool Dependencies

The action assumes these are available (true for `ubuntu-latest`):
- `curl` - download installer
- `gh` - GitHub CLI for PR creation
- `jq` - JSON parsing (used in apply step)
- `git` - version control

**Self-hosted runners**: May need to install `gh` CLI. Document this.

### Resolved Inputs

| Input | Resolution |
|-------|------------|
| `version` | Added - pins binary version for reproducibility |
| `force-sync` | Added - optional, runs apply even without version updates |
| `labels` | Added - auto-applies `dependencies` label if it exists in repo |
| `base-branch` | Skipped - defaults to repo default, rarely needed |

### Behavioral Edge Cases

1. **Files out of sync but refs unchanged**: Handled via `force-sync` input - runs `apply` even without version updates.

2. **Existing PR with conflicts**: If the PR branch has merge conflicts with base, `gh pr create` succeeds but PR can't merge. Expected behavior - user must resolve.

3. **Commit signing required**: If repo requires signed commits, action will fail. Document as limitation.

4. **Branch protection on default branch**: Not an issue - we're creating a PR, not pushing to protected branch.

5. **Rate limiting**: If config references many repos, `check --updates` might hit GitHub API limits. The CLI should handle this, but document potential for failures.

### Implementation Fixes Needed

1. **Empty commit handling**:
```bash
# Before git commit
if git diff --quiet && git diff --cached --quiet; then
  echo "No changes to commit"
  echo "has-changes=false" >> $GITHUB_OUTPUT
  exit 0
fi
```

2. **Config file check**:
```bash
# At start of action
if [ ! -f "${{ inputs.config-path }}" ]; then
  echo "::error::Config file not found: ${{ inputs.config-path }}"
  exit 1
fi
```

3. **Include staged and unstaged in diff**:
```bash
# Current only checks unstaged
files=$(git diff --name-only)
# Should also include new files
files=$(git status --porcelain | awk '{print $2}' | jq -R -s -c 'split("\n") | map(select(length > 0))')
```

## Resolved Questions

| Question | Decision |
|----------|----------|
| `force-sync` input? | Yes - optional, not default |
| Version pinning? | Yes - important for reproducibility |
| Labels? | Yes - auto-apply `dependencies` if it exists |

## Linting & Testing

### Linting Tools

| Tool | Validates action.yml? | Validates workflows? | Notes |
|------|----------------------|---------------------|-------|
| [actionlint](https://github.com/rhysd/actionlint) | No | Yes | Only workflow files, not action definitions |
| [action-validator](https://github.com/mpalmer/action-validator) | Yes | Yes | Validates against JSON schemas, checks glob patterns |

**Recommendation**: Use `action-validator` since it validates `action.yml` files (composite actions), not just workflows.

### Pre-commit Configuration

Add to `.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/mpalmer/action-validator
  rev: v0.6.0
  hooks:
    - id: action-validator
      # Default only targets .github/workflows/, add root action.yml
      files: (^\.github/(workflows|actions)/.*\.ya?ml$|^action\.ya?ml$)
```

Note: The default hook only targets `.github/workflows/`. We override `files` to also include the root `action.yml`.

### Local Testing with `act`

[nektos/act](https://github.com/nektos/act) runs GitHub Actions locally using Docker:

```bash
# Install act
brew install act  # or see https://github.com/nektos/act#installation

# Run a specific workflow
act -W .github/workflows/test-action.yml

# Run with specific event
act workflow_dispatch
```

**Limitations for composite actions:**
- `act` works best for testing workflows that *use* the action
- Create a test workflow in `.github/workflows/test-action.yml` that exercises the action

### CI Testing Strategy

Create `.github/workflows/test-action.yml` to test the action itself:

```yaml
name: Test Action

on:
  push:
    paths:
      - 'action.yml'
      - '.github/workflows/test-action.yml'
  pull_request:
    paths:
      - 'action.yml'

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate action.yml
        run: |
          cargo install action-validator
          action-validator action.yml

  test-dry-run:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Create a minimal test config
      - name: Create test config
        run: |
          echo "- repo:" > .common-repo.yaml
          echo "    url: https://github.com/common-repo/common-repo" >> .common-repo.yaml
          echo "    ref: main" >> .common-repo.yaml
          echo "    with:" >> .common-repo.yaml
          echo "      - include: [README.md]" >> .common-repo.yaml

      # Test the action in dry-run mode
      - name: Test action (dry-run)
        uses: ./
        with:
          dry-run: 'true'
```

### Best Practices

1. **Lint in pre-commit** - Catch issues before commit with `action-validator`
2. **Lint in CI** - Run `action-validator` in CI as backup
3. **Test with dry-run** - Test the action without creating actual PRs
4. **Test on real workflow** - Have a test repo that uses the action for integration testing
5. **Version with tags** - Use semantic versioning tags (v1, v1.0.0) for stable references

## Documentation

### README.md

Add a section after "Checking for updates":

```markdown
### Automated updates with GitHub Actions

Use the common-repo GitHub Action to automatically create PRs when upstream configurations change:

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
```

### docs/src/github-action.md (new file)

Create a dedicated documentation page:

```markdown
# GitHub Action

The common-repo GitHub Action automatically checks for upstream updates and creates pull requests with the changes.

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

If your `.common-repo.yaml` references private repositories, you'll need a Personal Access Token (PAT) with `repo` scope:

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
```

### docs/src/SUMMARY.md

Add to the table of contents:

```markdown
- [GitHub Action](./github-action.md)
```

## Next Steps

1. Add `action-validator` to `.pre-commit-config.yaml`
2. Write the `action.yml` file
3. Create `.github/workflows/test-action.yml`
4. Update `README.md` with action usage
5. Create `docs/src/github-action.md`
6. Update `docs/src/SUMMARY.md`
7. Test with a sample downstream repo

## Notes

- `install.sh` uses `VERSION` env var for pinning (not a CLI flag) - already supported
- `action-validator` is Rust-based and can be installed via `cargo install`
