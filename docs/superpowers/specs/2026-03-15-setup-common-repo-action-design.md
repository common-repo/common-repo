# Design: setup-common-repo GitHub Action

## Problem

Workflows that need the common-repo binary must inline the install logic:

```yaml
- name: Install common-repo
  shell: bash
  run: |
    curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
    echo "$HOME/.local/bin" >> $GITHUB_PATH
```

This is duplicated across every consuming workflow. A setup action reduces it to:

```yaml
- uses: common-repo/setup-common-repo@v1
- run: common-repo validate
```

## Solution

A new repository `common-repo/setup-common-repo` containing a composite GitHub Action that wraps the existing `install.sh` script.

## Repository Structure

```
setup-common-repo/
├── action.yml                # Composite action definition
├── .releaserc.yaml           # Semantic release config with floating major tags
├── .pre-commit-config.yaml   # YAML, JSON, markdown, whitespace hooks
├── commitlint.config.js      # Conventional commit enforcement
├── CODEOWNERS                # Code ownership for PR reviews
├── .github/
│   └── workflows/
│       ├── ci.yml            # Lint job: pre-commit checks
│       ├── test-action.yml   # Test job: self-consume action, verify binaries
│       ├── commitlint.yml    # Conventional commit linting on PRs
│       └── release.yaml      # Semantic release with floating major version tags
├── LICENSE                   # AGPL-3.0-or-later
└── README.md                 # Usage documentation
```

## Action Interface

### Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `version` | No | `latest` | Version to install (e.g., `v0.28.1`) or `latest` |

### Outputs

| Output | Description |
|--------|-------------|
| `version` | The installed version string (e.g., `0.28.1`) |
| `path` | Absolute path to the installed binary |

### Behavior

1. Run `install.sh` via curl, passing:
   - `VERSION` env var from the `version` input (empty string for `latest`)
   - `GITHUB_TOKEN` from the runner environment (transparent, avoids API rate limits)
   - No need to set `INSTALL_PREK` or `SKIP_ALIAS` — prek prompt is skipped automatically in non-interactive environments, and the `cr` alias is created by default
2. Add `~/.local/bin` to `$GITHUB_PATH`
3. Capture version by parsing `common-repo --version` output (strip binary name prefix, e.g., `common-repo 0.28.1` -> `0.28.1`)
4. Capture binary path via `which common-repo` as the `path` output

## CI/CD

All workflows should use least-privilege `permissions:` blocks and `concurrency` groups to cancel in-progress runs on the same branch.

### Lint Job (`ci.yml`)

- Triggered on: push to main, pull requests, workflow_call
- Permissions: `contents: read`
- Job name: `Pre-commit Checks` (must match branch protection status check)
- Runs `pre-commit/action@v3.0.1` with `actions/setup-python@v5`
- Pre-commit hooks:
  - `pre-commit-hooks`: trailing-whitespace, end-of-file-fixer, check-yaml, check-merge-conflict, check-added-large-files, check-json
  - `markdownlint-cli2`: markdown linting

### Test Job (`test-action.yml`)

- Triggered on: push to main, pull requests
- Permissions: `contents: read`
- Job name: `Test Action` (must match branch protection status check)
- Matrix: `ubuntu-latest` (Linux x86_64 only — macOS runners are expensive and the install.sh platform detection is already tested upstream)
- Steps:
  1. Checkout the repo
  2. Self-consume the action with `uses: ./`
  3. Assert `common-repo --version` produces output matching a semver pattern
  4. Assert `cr --version` produces matching output (alias works)
  5. Verify the `version` and `path` outputs are populated
- Version matrix: `[{version: "latest"}, {version: "v0.28.1"}]` to test both latest resolution and pinned version

### Commitlint (`commitlint.yml`)

- Triggered on: pull requests to main
- Uses `wagoid/commitlint-github-action` with `@commitlint/config-conventional`

### Release (`release.yaml`)

- Triggered on: push to main, workflow_dispatch
- Calls CI workflow via `workflow_call` and uses `needs:` so that manual dispatch also requires CI to pass
- Uses `common-repo-bot` GitHub App token for authentication
- Runs semantic-release with plugins:
  - `@semantic-release/commit-analyzer` (conventionalcommits preset)
  - `@semantic-release/release-notes-generator` (conventionalcommits preset)
  - `@semantic-release/changelog`
  - `@semantic-release/git`
  - `@semantic-release/github` (with noisy comments disabled)
  - `semantic-release-major-tag` (creates/updates floating `v1`, `v2` tags)

## Branch Protection (main)

Matches common-repo org standard:

- Required PR reviews: 1 approval
- Code owner review required
- PR bypass: `common-repo-bot` app
- Required status checks (strict): `Pre-commit Checks`, `Test Action`
- Linear history required
- No force pushes or deletions

## Consumers (follow-up work)

After the action is published, update these to use it:

- `common-repo/cr-upstream-repo` CI and distributed `src/.github/workflows/ci.yaml`
- `common-repo/common-repo/action.yml` install step
- Any other repos using the curl install pattern

## Decisions

- **No caching**: Binary download is fast; install.sh may gain additional setup responsibilities later, so running it each time is preferred.
- **No Node.js**: Composite (shell-based) action keeps it simple and dependency-free.
- **Transparent GITHUB_TOKEN**: Passed from the runner environment to install.sh to avoid GitHub API rate limits when resolving `latest`. No explicit input needed.
- **Self-contained repo**: No `.common-repo.yaml` consumption for now. When upstream sources like cr-semantic-release are ready, adding a config will merge smoothly.
