# Feature Spec: Selective Source Updates with `--filter`

**Date:** 2026-01-07
**Status:** Draft

## Summary

Add a `--filter <GLOB>` option to `common-repo update` that allows updating only sources matching a glob pattern. The pattern matches against the combined `url/path` string for each repo operation.

## Motivation

When a config has multiple repo sources, users often want to:
- Update CI templates without touching stable linter config
- Test updates incrementally to isolate issues
- Maintain different update cadences for different sources

Currently, `update` operates on all repos at once with no way to be selective.

## Design

### Command Interface

```bash
# Update only repos matching the glob pattern
common-repo update --filter "github.com/org/ci-*"

# Multiple filters (OR logic)
common-repo update --filter "*/*/ci-*" --filter "*/*/linter-*"

# Combine with existing flags
common-repo update --filter "github.com/org/*" --latest --dry-run
```

### Match Target

Each `repo` operation has:
- `url` (required): e.g., `https://github.com/org/templates`
- `path` (optional): e.g., `ci/` for a subdirectory

The filter matches against a normalized string with the URL scheme stripped:
```
{url_without_scheme}/{path}
```

The scheme (`https://`, `http://`, `git://`, etc.) is stripped by removing everything up to and including `://`. If `path` is not specified, the match target is just the schemeless URL.

**Examples:**

| Config | Match Target |
|--------|--------------|
| `url: https://github.com/org/ci-templates` | `github.com/org/ci-templates` |
| `url: https://github.com/org/monorepo`, `path: configs/eslint` | `github.com/org/monorepo/configs/eslint` |
| `url: git://gitlab.com/org/repo` | `gitlab.com/org/repo` |

### Glob Semantics

Use the same `glob::Pattern` from `src/path.rs` that powers `include`/`exclude` operations:

- `*` matches any sequence except `/`
- `**` matches any sequence including `/`
- `?` matches any single character
- `[abc]` matches any character in the set

**Pattern Examples:**

```bash
# Match any repo under github.com/myorg
--filter "github.com/myorg/*"

# Match repos with "ci" in the name
--filter "*/*ci*"

# Match specific subpath in a monorepo
--filter "github.com/org/monorepo/configs/eslint"

# Match all GitHub repos (vs GitLab, etc.)
--filter "github.com/**"

# Match any org's ci-templates repo
--filter "*/*/ci-templates"
```

### Behavior

1. If `--filter` is not provided, behavior is unchanged (update all repos)
2. If `--filter` is provided, only matching repos are checked/updated
3. Multiple `--filter` flags use OR logic (match any)
4. Filter applies before `--compatible`/`--latest` filtering
5. If no repos match the filter, print a message and exit cleanly

### Output

When filter is active, indicate it in the output:

```
Loading configuration from: .common-repo.yaml
Filtering sources matching: */org/ci-*
Checking for repository updates...

📦 Available Updates:
1 repository can be updated (2 filtered out)

🔄 https://github.com/org/ci-templates (current: v1.2.0)
   Latest: v1.3.0
   ✅ Compatible update
```

## Implementation

### Files to Modify

1. **`src/commands/update.rs`**
   - Add `filter: Vec<String>` to `UpdateArgs`
   - Add filtering logic after loading config, before checking updates
   - Update output messages to show filter status

2. **`src/path.rs`** (optional)
   - May add helper if needed, but `glob_match` already exists

3. **Tests**
   - Unit tests for filter matching logic
   - Integration test with multi-repo config

### Code Sketch

```rust
// In UpdateArgs
/// Filter sources by glob pattern (matches against url/path, scheme stripped)
#[arg(long, value_name = "GLOB")]
pub filter: Vec<String>,

// In execute() or src/path.rs
fn strip_url_scheme(url: &str) -> &str {
    url.find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url)
}

fn build_match_target(repo: &RepoOp) -> String {
    let base = strip_url_scheme(&repo.url);
    match &repo.path {
        Some(path) => format!("{}/{}", base, path.trim_matches('/')),
        None => base.to_string(),
    }
}

// Filter repos before update check
let repos_to_check: Vec<_> = if args.filter.is_empty() {
    all_repos
} else {
    all_repos
        .into_iter()
        .filter(|repo| {
            let target = build_match_target(repo);
            args.filter.iter().any(|pattern| {
                glob_match(pattern, &target).unwrap_or(false)
            })
        })
        .collect()
};
```

## Acceptance Criteria

- [ ] `--filter` flag added to `update` command
- [ ] Filter matches against `url/path` string
- [ ] Multiple `--filter` flags work with OR logic
- [ ] Filtered-out repos are not checked for updates
- [ ] Output shows filter status and count of filtered repos
- [ ] `--help` documents the flag with examples
- [ ] Unit tests cover pattern matching
- [ ] E2E test verifies filtered update

## Future Considerations

- Could add `--filter` to other commands (`check`, `diff`) for consistency
- Could support `--exclude-filter` for inverse matching
- Could add named sources (`name: ci-templates`) for friendlier filtering

## References

- Existing glob implementation: `src/path.rs:glob_match()`
- Current update command: `src/commands/update.rs`
- Competitive analysis: `context/research/competitive-analysis.md`
