# Unified Ref Resolution Flag

## Problem

When the same repo appears multiple times in the dependency tree with different refs:

```
local
├── A@v1.0.0
│   └── C@v1.0.0
└── B@v2.0.0
    └── C@v1.2.0
```

Currently, both `C@v1.0.0` and `C@v1.2.0` are processed. The compositing order determines which "wins" for conflicting files, but this is implicit and potentially confusing.

## Proposal

Add an optional flag to unify duplicate repos to a single ref:

```bash
common-repo sync --unify-refs=latest
```

### Behavior

When enabled, if the same repo URL appears with different refs:
1. Compare all refs semantically (semver if possible, otherwise lexicographic)
2. Select the "latest" ref
3. Use that ref for ALL occurrences of that repo in the tree

### Example

With `--unify-refs=latest`, the tree above becomes:

```
local
├── A@v1.0.0
│   └── C@v1.2.0  (unified)
└── B@v2.0.0
    └── C@v1.2.0
```

### Options

- `--unify-refs=latest` - Use highest version
- `--unify-refs=earliest` - Use lowest version (conservative)
- `--unify-refs=error` - Fail if duplicates have different refs

### Lockfile Interaction

When a lockfile is present, unification happens at resolution time and is recorded in the lockfile. The lockfile would note:

```yaml
- url: https://github.com/common-repo/base
  declared_refs: [v1.0.0, v1.2.0]  # what was declared
  resolved_ref: v1.2.0             # what was used
  unified: true                     # flag indicating unification occurred
```

## Benefits

- Explicit control over diamond dependency resolution
- Avoids subtle bugs from mixed versions
- Clear audit trail in lockfile

## Considerations

- "Latest" isn't always safest (newer might have breaking changes)
- Semver parsing isn't always possible (branches, commit SHAs)
- Some users may want per-repo policies
