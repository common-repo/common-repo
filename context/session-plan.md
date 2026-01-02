# Session Plan

## Status: Planning

## Goal
Clean up README to remove invalid crate.io references

## Questions & Answers

**Q1:** Remove crates.io and docs.rs badges from README?
**A1:** Yes

**Q2:** Also remove "From crates.io" installation section (`cargo install common-repo`)?
**A2:** Yes - not valid for this project

**Q3:** Add GitHub Pages documentation link? Where?
**A3:** Yes - both as a badge and a dedicated section near top
**URL:** `https://common-repo.github.io/common-repo/`

**Q4:** README merges section only shows YAML example - add examples for other file types?
**A4:** Yes - realistic examples for each (YAML, JSON, TOML, INI, Markdown)

**Q5:** Compare CLI help output to user docs - any discrepancies?
**A5:** Yes - found 2 issues:
- `--log-level` docs include `off` but CLI doesn't support it
- Cache directory default inconsistent (`~/.common-repo/cache` vs `~/.cache/common-repo`)

**Q6:** Add user docs location to CLAUDE.md?
**A6:** Yes - docs are in `docs/src/` (mdBook format), not referenced in CLAUDE.md

## Tasks

1. Remove crates.io and docs.rs badges from README (lines 5-6)
2. Remove "From crates.io" installation section from README (lines 26-30)
3. Add GitHub Pages documentation link:
   - Badge in badge row (replacing removed badges)
   - Dedicated "Documentation" section near top
4. Expand "Merging files" section with realistic examples for each file type:
   - YAML (existing: CI workflows)
   - JSON (e.g., package.json)
   - TOML (e.g., Cargo.toml)
   - INI (e.g., .editorconfig)
   - Markdown (e.g., README sections)
5. Fix CLI docs discrepancies in `docs/src/cli.md`:
   - Remove `off` from `--log-level` options (line 12)
   - Standardize cache directory default across docs
6. Add user docs location to CLAUDE.md (`docs/src/` - mdBook format)
