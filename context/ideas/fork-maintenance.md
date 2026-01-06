# Automated Fork Maintenance

## Problem

When inheriting from upstream repos (e.g., `common-repo/rust-cli`), users depend on external infrastructure they don't control. While most users commit the output files (reducing runtime dependency), some organizations want full control over their dependency chain.

## Proposal

Provide tooling to automate fork maintenance:

### 1. Fork Sync Actions

GitHub Actions (or equivalent) that automatically:
- Watch upstream repos for new tags/releases
- Create PRs to update fork with upstream changes
- Run validation before merging

### 2. URI Rewriting

A command or config option to rewrite `repo:` URLs:

```yaml
# .common-repo.yaml or separate config
uri_rewrites:
  - from: https://github.com/common-repo/
    to: https://github.com/my-org/common-repo-fork-
```

This would transform:
- `common-repo/rust-cli` → `my-org/common-repo-fork-rust-cli`
- `common-repo/base` → `my-org/common-repo-fork-base`

### 3. Bulk Fork Setup

A command to fork an entire dependency tree:

```bash
common-repo fork --org my-org --prefix common-repo-fork-
```

This would:
1. Resolve full dependency tree
2. Fork each repo to the target org
3. Generate a URI rewrite config
4. Optionally set up sync actions on each fork

## Benefits

- Full control over dependency chain
- Audit capability for all inherited content
- Resilience against upstream disappearance
- Ability to patch/customize upstream before inheritance

## Considerations

- Fork maintenance is ongoing work
- Sync actions need care to avoid breaking changes
- May want selective forking (some repos, not others)
