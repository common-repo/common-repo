# Update Workflow

This guide explains how to check for updates and apply them to your inherited repositories.

## Workflow Overview

The update workflow follows a check-review-apply pattern:

1. **Check** for available updates
2. **Review** changes before applying
3. **Apply** with confidence

## Checking for Updates

See if any inherited repositories have newer versions available:

```bash
common-repo check --updates
```

This compares your pinned refs against available tags in each repository. Output shows:
- Current ref for each inherited repo
- Available newer versions
- Whether updates are compatible (minor/patch) or breaking (major)

## Reviewing Changes

Before updating refs, preview what would change in your files:

```bash
# Update refs without applying
common-repo update --dry-run

# See file differences after updating
common-repo diff
```

The `--dry-run` flag shows which refs would change without modifying your config file.

## Applying Updates

### Compatible Updates (Safe)

Update to the latest compatible versions (same major version):

```bash
common-repo update
```

This updates minor and patch versions only. For example, `v1.2.0` might update to `v1.3.5` but not `v2.0.0`.

### Latest Updates (Breaking)

Include breaking changes when updating:

```bash
common-repo update --latest
```

This updates to the newest available version regardless of major version changes.

### Skip Confirmation

For scripting or CI, skip the confirmation prompt:

```bash
common-repo update --yes
```

## Complete Example

A typical update session:

```bash
# Step 1: Check what's available
common-repo check --updates

# Step 2: Preview ref changes
common-repo update --dry-run

# Step 3: Apply ref updates to config
common-repo update

# Step 4: Preview file changes
common-repo diff

# Step 5: Apply file changes with dry-run first
common-repo apply --dry-run

# Step 6: Apply for real
common-repo apply
```

## Semantic Versioning

common-repo understands semantic versioning when comparing refs:

| Update Type | Example | `--compatible` | `--latest` |
|-------------|---------|----------------|------------|
| Patch | v1.2.0 → v1.2.1 | Yes | Yes |
| Minor | v1.2.0 → v1.3.0 | Yes | Yes |
| Major | v1.2.0 → v2.0.0 | No | Yes |

The `--compatible` flag (default) follows semver: patch and minor updates are safe, major updates may contain breaking changes.

## Tips

**Pin to specific versions** for production stability:

```yaml
- repo:
    url: https://github.com/your-org/configs
    ref: v2.1.0  # Pinned version
```

**Use branches** for development or always-latest behavior:

```yaml
- repo:
    url: https://github.com/your-org/configs
    ref: main  # Tracks branch HEAD
```

**Check updates in CI** to catch drift:

```bash
# Fails if updates are available
common-repo check --updates || echo "Updates available"
```
