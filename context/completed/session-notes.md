# Session Notes

Captured during session on 2025-12-20.
Plans created: 2025-12-29.

## Status

All items below have been converted to plan JSON files:
- `context/init-redesign.json` - Init command redesign
- `context/add-command.json` - New add command
- `context/source-declared-merge.json` - Source-declared merge (stub for exploration)
- `context/cr-alias.json` - Short alias `cr`
- `context/configuration-docs.json` - Configuration.md improvements
- `context/source-authoring-docs.json` - Source repository authoring docs

## Notes

### docs/configuration.md improvements

1. **Add table of contents** - Document is 480+ lines, needs navigation
2. **Operator listing at top needs descriptions** - The YAML block (lines 9-22) showing all operators should include brief comments explaining what each operator does
   - Alternative: Include descriptions in the ToC instead of inline comments
   - Current listing is just syntax, no context for what each operator does

### `common-repo init` command redesign

Current behavior is not useful - just dumps an example YAML file into the repo.

**Needs to be redesigned to:**
- Walk user through an interactive setup process
- Guide adding the common-repo pre-commit hook
- The pre-commit hook config itself should be a source repository (dogfooding)
- Potentially offer choices for common base templates/configurations
- Accept a URI argument for a parent repo as starting point (e.g. `common-repo init https://github.com/org/base-config`)
- When given a URI, automatically detect the latest semver release tag for the `ref` field

### New `common-repo add` command

Add a new source repository to the configuration file.

**Behavior:**
- `common-repo add <URI>` - adds a new `repo:` entry to `.common-repo.yaml`
- If no config file exists, create it
- Should also auto-detect latest semver release for the `ref` field
- Essentially a quick way to append repos without editing YAML manually

### Merge operators: Source-declared merge behavior

**Problem:** Currently merge operators must be explicitly called in the consuming repo's config. This is cumbersome when the source repo knows how its files should be merged.

**Desired behavior:** Source repos can declare merge behavior internally, so consumers just do:
```yaml
- repo: org/my-claude-rules
  ref: v1.0.0
```
...and the source repo's CLAUDE.md automatically merges into the consumer's CLAUDE.md without explicit `markdown:` operator in consumer config.

**Use case:** A common repo with CLAUDE.md rules that should merge into many other repos simply.

**Open questions:**
- How does source repo declare "this file should merge, not overwrite"?
- What's the syntax? Metadata in the file? Separate manifest? Convention-based (e.g. `*.merge.md`)?
- What if consumer wants to override the merge behavior?
- Priority: Start with Markdown merge, then extend to other formats

### Short command alias: `cr`

Add `cr` as a short alias for `common-repo` (the existing `cr` command for newline conversion is obscure and rarely installed).

**Implementation options:**
- Install script adds `alias cr=common-repo` to user's shell profile
- Symlink `cr` -> `common-repo` in the bin directory where it installs
- Change the actual binary name to `cr` (most direct)
- Combination: binary named `cr`, with `common-repo` as symlink for discoverability

### Documentation gap: Source repository authoring

Current docs are focused on *consuming* existing common-repos.

**Missing documentation for source repository authors:**
- Best practices for creating a source repository
- What to include/exclude
- How to structure for reusability
- Template variable conventions
- Versioning and release strategies
- How to design for composability (being inherited alongside other repos)
- Testing your source repo
