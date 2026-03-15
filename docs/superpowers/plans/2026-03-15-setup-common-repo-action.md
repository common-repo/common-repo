# setup-common-repo GitHub Action Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a composite GitHub Action at `common-repo/setup-common-repo` that installs the common-repo binary, with full CI/CD including semantic release with floating major version tags.

**Architecture:** Composite (shell-based) GitHub Action wrapping the existing `install.sh` script. CI uses pre-commit for linting, self-consumption for testing, and semantic-release for publishing.

**Tech Stack:** GitHub Actions (composite), shell, semantic-release, pre-commit, commitlint

**Target repo:** `/Users/shakefu/git/common-repo/setup-common-repo` (already cloned, empty)

**Spec:** `docs/superpowers/specs/2026-03-15-setup-common-repo-action-design.md`

---

## File Map

| File | Responsibility |
|------|---------------|
| `action.yml` | Composite action: install binary, set PATH, expose outputs |
| `.pre-commit-config.yaml` | Pre-commit hooks for YAML, JSON, markdown, whitespace |
| `commitlint.config.js` | Conventional commit config |
| `.releaserc.yaml` | Semantic release config with floating major tags |
| `CODEOWNERS` | Code ownership for required reviews |
| `.github/workflows/ci.yml` | Lint job via pre-commit |
| `.github/workflows/test-action.yml` | Self-consume action, verify binaries |
| `.github/workflows/commitlint.yml` | PR commit message linting |
| `.github/workflows/release.yaml` | Semantic release with common-repo-bot |
| `LICENSE` | AGPL-3.0-or-later (matches common-repo) |
| `README.md` | Usage documentation |

---

## Chunk 1: Core Action and Config Files

### Task 1: Initialize repo with action.yml

**Files:**
- Create: `action.yml`

- [ ] **Step 1: Create action.yml**

```yaml
name: 'setup-common-repo'
description: 'Install the common-repo binary'
author: 'common-repo'

inputs:
  version:
    description: 'Version to install (e.g., v0.28.1) or latest'
    required: false
    default: 'latest'

outputs:
  version:
    description: 'The installed version of common-repo'
    value: ${{ steps.info.outputs.version }}
  path:
    description: 'Path to the installed binary'
    value: ${{ steps.info.outputs.path }}

runs:
  using: composite
  steps:
    - name: Install common-repo
      shell: bash
      env:
        VERSION: ${{ inputs.version != 'latest' && inputs.version || '' }}
        GITHUB_TOKEN: ${{ github.token }}
      run: |
        curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
        echo "$HOME/.local/bin" >> $GITHUB_PATH

    - name: Capture install info
      id: info
      shell: bash
      run: |
        bin_path="$HOME/.local/bin/common-repo"
        version=$("$bin_path" --version | awk '{print $2}')
        echo "version=$version" >> $GITHUB_OUTPUT
        echo "path=$bin_path" >> $GITHUB_OUTPUT
```

Note: `$GITHUB_PATH` changes don't take effect until the next step in the *calling* workflow, not within the composite action itself. We use the absolute path `$bin_path` to work around this.

- [ ] **Step 2: Commit**

```bash
cd /Users/shakefu/git/common-repo/setup-common-repo
git add action.yml
git commit -m "feat: add composite action for installing common-repo"
```

### Task 2: Add pre-commit config

**Files:**
- Create: `.pre-commit-config.yaml`

- [ ] **Step 1: Create .pre-commit-config.yaml**

```yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
        args: [--allow-multiple-documents]
      - id: check-json
      - id: check-merge-conflict
      - id: check-added-large-files
  - repo: https://github.com/DavidAnson/markdownlint-cli2
    rev: v0.17.2
    hooks:
      - id: markdownlint-cli2
```

- [ ] **Step 2: Commit**

```bash
git add .pre-commit-config.yaml
git commit -m "chore: add pre-commit config for linting"
```

### Task 3: Add commitlint config

**Files:**
- Create: `commitlint.config.js`

- [ ] **Step 1: Create commitlint.config.js**

```javascript
module.exports = {
  extends: ["@commitlint/config-conventional"],
};
```

- [ ] **Step 2: Commit**

```bash
git add commitlint.config.js
git commit -m "chore: add commitlint config for conventional commits"
```

### Task 4: Add semantic release config

**Files:**
- Create: `.releaserc.yaml`

- [ ] **Step 1: Create .releaserc.yaml**

Copy the exact config from cr-upstream-repo (which comes from cr-semantic-release):

```yaml
---
branches:
  - main

preset: "conventionalcommits"
tagFormat: "v${version}"

plugins:
  - - "@semantic-release/commit-analyzer"
    - preset: conventionalcommits
  - - "@semantic-release/release-notes-generator"
    - preset: conventionalcommits
  - - "@semantic-release/changelog"
  - - "@semantic-release/git"
  - - "@semantic-release/github"
    # Disable noisy issue/PR comments to avoid GH rate limits
    - failComment: false
      failTitle: false
      labels: false
      releasedLabels: false
      successComment: false
  - - "semantic-release-major-tag"

generateNotes:
  - path: "@semantic-release/release-notes-generator"
    writerOpts:
      commitsSort:
        - subject
        - scope
    presetConfig:
      types:
        - type: build
          section: Build System
          hidden: false
        - type: chore
          section: Miscellaneous
          hidden: false
        - type: ci
          section: Continuous Integration
          hidden: false
        - type: docs
          section: Documentation
          hidden: false
        - type: feat
          section: Features
          hidden: false
        - type: fix
          section: Bug Fixes
          hidden: false
        - type: perf
          section: Performance Improvements
          hidden: false
        - type: refactor
          section: Code Refactoring
          hidden: false
        - type: style
          section: Styles
          hidden: false
        - type: test
          section: Tests
          hidden: false
```

- [ ] **Step 2: Commit**

```bash
git add .releaserc.yaml
git commit -m "chore: add semantic release config with floating major tags"
```

### Task 5: Add CODEOWNERS and LICENSE

**Files:**
- Create: `CODEOWNERS`
- Create: `LICENSE`

- [ ] **Step 1: Create CODEOWNERS**

```
* @common-repo/maintainers
```

Note: Verify the team `@common-repo/maintainers` exists. If not, use `@shakefu` or the appropriate owner. Check with: `gh api orgs/common-repo/teams --jq '.[].slug'`

- [ ] **Step 2: Create LICENSE**

Use AGPL-3.0-or-later to match common-repo. Fetch the standard text:

```bash
curl -fsSL https://www.gnu.org/licenses/agpl-3.0.txt > LICENSE
```

- [ ] **Step 3: Commit**

```bash
git add CODEOWNERS LICENSE
git commit -m "chore: add CODEOWNERS and AGPL-3.0 license"
```

---

## Chunk 2: CI/CD Workflows

### Task 6: Add CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create .github/workflows/ci.yml**

```yaml
name: CI

on:
  pull_request:
  push:
    branches:
      - main
  workflow_call:

concurrency:
  group: ${{ github.workflow }}-${{ github.head_ref || github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  pre-commit:
    name: Pre-commit Checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: "3.x"

      - uses: pre-commit/action@v3.0.1
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add lint workflow with pre-commit checks"
```

### Task 7: Add test workflow

**Files:**
- Create: `.github/workflows/test-action.yml`

- [ ] **Step 1: Create .github/workflows/test-action.yml**

```yaml
name: Test

on:
  pull_request:
  push:
    branches:
      - main

concurrency:
  group: ${{ github.workflow }}-${{ github.head_ref || github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  test:
    name: Test Action
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Test 1: latest version
      - name: Setup common-repo (latest)
        id: setup-latest
        uses: ./

      - name: Verify latest install
        run: |
          # Verify common-repo command works and outputs a version
          version_output=$(common-repo --version)
          echo "common-repo --version: $version_output"
          if ! echo "$version_output" | grep -qE '^common-repo [0-9]+\.[0-9]+\.[0-9]+'; then
            echo "::error::common-repo --version did not match expected pattern"
            exit 1
          fi

          # Verify cr alias produces same output
          if [ "$(common-repo --version)" != "$(cr --version)" ]; then
            echo "::error::cr alias output does not match common-repo output"
            exit 1
          fi
          echo "cr alias verified: $(cr --version)"

          # Verify outputs
          echo "version output: ${{ steps.setup-latest.outputs.version }}"
          echo "path output: ${{ steps.setup-latest.outputs.path }}"
          if ! echo "${{ steps.setup-latest.outputs.version }}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+'; then
            echo "::error::version output does not match semver pattern"
            exit 1
          fi
          if [ ! -x "${{ steps.setup-latest.outputs.path }}" ]; then
            echo "::error::path output does not point to an executable"
            exit 1
          fi

      # Test 2: pinned version
      - name: Setup common-repo (pinned)
        id: setup-pinned
        uses: ./
        with:
          version: v0.28.1

      - name: Verify pinned install
        run: |
          expected="0.28.1"
          actual="${{ steps.setup-pinned.outputs.version }}"
          if [ "$actual" != "$expected" ]; then
            echo "::error::Expected version $expected but got $actual"
            exit 1
          fi
          echo "Pinned version verified: $actual"
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/test-action.yml
git commit -m "ci: add test workflow that self-consumes the action"
```

### Task 8: Add commitlint workflow

**Files:**
- Create: `.github/workflows/commitlint.yml`

- [ ] **Step 1: Create .github/workflows/commitlint.yml**

```yaml
name: Commitlint

on:
  pull_request:
    branches:
      - main

permissions:
  contents: read
  pull-requests: read

jobs:
  commitlint:
    name: Lint Commits
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: wagoid/commitlint-github-action@v6
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/commitlint.yml
git commit -m "ci: add commitlint workflow for PR validation"
```

### Task 9: Add release workflow

**Files:**
- Create: `.github/workflows/release.yaml`

- [ ] **Step 1: Create .github/workflows/release.yaml**

```yaml
name: Release

on:
  workflow_dispatch:
  push:
    branches:
      - main

permissions:
  contents: write
  issues: write
  pull-requests: write

jobs:
  ci:
    name: CI
    permissions:
      contents: read
    uses: ./.github/workflows/ci.yml

  release:
    name: Release
    needs: ci
    permissions:
      contents: write
    runs-on: ubuntu-latest
    steps:
      - name: Generate app token
        id: app-token
        uses: actions/create-github-app-token@v2
        with:
          app-id: ${{ secrets.COMMON_REPO_BOT_CLIENT_ID }}
          private-key: ${{ secrets.COMMON_REPO_BOT_PRIVATE_KEY }}
          owner: common-repo

      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ steps.app-token.outputs.token }}
          persist-credentials: true

      - uses: actions/setup-node@v5
        with:
          cache: npm
          cache-dependency-path: ".releaserc.yaml"
          node-version: lts/*

      - name: Semantic Release
        id: release
        run: |
          # Semantic Release
          npm install --no-save --no-package-lock \
            "@semantic-release/commit-analyzer" \
            "@semantic-release/release-notes-generator" \
            "@semantic-release/changelog" \
            "@semantic-release/git" \
            "@semantic-release/github" \
            "semantic-release-major-tag" \
            "conventional-changelog-conventionalcommits"
          npx semantic-release > release.log 2>&1; rc=$?
          cat release.log
          if grep -q "There are no relevant changes, so no new version is released." release.log; then
            echo "publish=false" >> "$GITHUB_OUTPUT"
          elif [ "$rc" -ne 0 ]; then
            echo "::error::semantic-release failed with exit code $rc"
            exit 1
          else
            echo "publish=true" >> "$GITHUB_OUTPUT"
          fi
        env:
          GITHUB_TOKEN: ${{ steps.app-token.outputs.token }}
    outputs:
      publish: ${{ steps.release.outputs.publish }}
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yaml
git commit -m "ci: add semantic release workflow with floating major tags"
```

---

## Chunk 3: README and Push

### Task 10: Add README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create README.md**

```markdown
# setup-common-repo

GitHub Action to install the [common-repo](https://github.com/common-repo/common-repo) binary.

## Usage

```yaml
steps:
  - uses: common-repo/setup-common-repo@v1
  - run: common-repo validate
```

### Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `version` | No | `latest` | Version to install (e.g., `v0.28.1`) or `latest` |

### Outputs

| Output | Description |
|--------|-------------|
| `version` | The installed version of common-repo |
| `path` | Path to the installed binary |

### Pin to a specific version

```yaml
steps:
  - uses: common-repo/setup-common-repo@v1
    with:
      version: v0.28.1
```

## License

AGPL-3.0-or-later
```

Note: The triple-backtick fences inside the yaml code blocks above need to be actual fences in the written file. The plan shows them inline for readability.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage instructions"
```

### Task 11: Push to remote and apply branch protection

- [ ] **Step 1: Push all commits**

```bash
cd /Users/shakefu/git/common-repo/setup-common-repo
git push -u origin main
```

- [ ] **Step 2: Wait for initial CI run**

Check that the workflows trigger: `gh run list --repo common-repo/setup-common-repo`

- [ ] **Step 3: Apply branch protection**

```bash
gh api repos/common-repo/setup-common-repo/branches/main/protection \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["Pre-commit Checks", "Test Action"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": true,
    "required_approving_review_count": 1,
    "require_last_push_approval": false,
    "bypass_pull_request_allowances": {
      "apps": ["common-repo-bot"],
      "teams": [],
      "users": []
    }
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "lock_branch": false,
  "allow_fork_syncing": false
}
EOF
```

- [ ] **Step 4: Verify branch protection**

```bash
gh api repos/common-repo/setup-common-repo/branches/main/protection \
  --jq '{enforce_admins: .enforce_admins.enabled, required_linear_history: .required_linear_history.enabled, status_checks: .required_status_checks.contexts}'
```

Expected: `{"enforce_admins":false,"required_linear_history":true,"status_checks":["Pre-commit Checks","Test Action"]}`
