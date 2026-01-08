# Session Plan

## Status: Planning

## Goal
README/docs cleanup, bug fixes, tech debt, and feature research

## Questions & Answers

**Q1:** Remove crates.io and docs.rs badges from README?
**A1:** Yes

**Q2:** Also remove "From crates.io" installation section (`cargo install common-repo`)?
**A2:** Yes - not valid for this project

**Q3:** Add GitHub Pages documentation link? Where?
**A3:** Yes - both as a badge (shields.io style like others) and a dedicated section near top
**URL:** `https://common-repo.github.io/common-repo/`

**Q4:** README merges section only shows YAML example - add examples for other file types?
**A4:** Yes - show all examples in README (demonstrates power of utility), each linking to full user docs for that operator. Also flesh out examples in user docs themselves.

**Q5:** Compare CLI help output to user docs - any discrepancies?
**A5:** Yes - found 2 issues:
- `--log-level` docs include `off` but CLI doesn't support it
- Cache directory default inconsistent (`~/.common-repo/cache` vs `~/.cache/common-repo`)

**Q6:** Add user docs location to CLAUDE.md?
**A6:** Yes - docs are in `docs/src/` (mdBook format), not referenced in CLAUDE.md

**Q7:** Is cache directory inconsistency a code bug or just docs?
**A7:** **BUG IN CODE** - `apply.rs` uses `~/.common-repo/cache` (hardcoded HOME), all other commands use `dirs::cache_dir()` → `~/.cache/common-repo`. Logic duplicated in 10 places. Need to:
- Create single `default_cache_root()` function
- Fix `apply.rs` to use same logic as others
- Update all 10 command files to use shared function

**Q8:** Are there other duplicated defaults in the codebase?
**A8:** Yes, several patterns found:
- Cache dir fallback (`.common-repo-cache`) - 10 places
- Config file default (`".common-repo.yaml"`) - 7+ command arg definitions
- Hardcoded config paths in add.rs, init.rs
Need full audit and centralization.

**Q9:** User docs contain AI-isms ("The Problem" / "The Solution" headers, etc.)?
**A9:** Yes, confirmed in `introduction.md`. Need to audit all docs and rewrite in natural human language.
Known AI-ism patterns to scan for:
- **Headers**: "The Problem/Solution", "Why This Matters", "Here's the Thing", "Key Takeaways"
- **Filler**: "It's worth noting", "At its core", "Simply put", "In today's world"
- **Buzzwords**: "Seamlessly", "Leverage", "Unlock", "Empower", "Streamline", "Robust", "Game-changer"
- **Metaphors**: "Navigate" (non-literal), "Journey", "Embrace", "Harness", "Delve/Dive into"
- **Openers**: "Whether you're...", "Imagine...", "Think of it as..."

**Q10:** Research similar projects for feature ideas?
**A10:** Yes - go broad, explore what's out there. Investigate:
- copier-org/copier
- cookiecutter
- yeoman
- Other template/config management tools discovered during research
For each: find features people love, features people wish existed.
Goal: discover features we weren't aware of to expand common-repo's capabilities.
Interactive research task - find info, review together, prioritize.

**Q11:** Which cache path should be canonical default?
**A11:** Use `~/.cache/common-repo` (XDG-compliant via `dirs::cache_dir()`). This is the standard location for cache files on Linux. macOS uses `~/Library/Caches/` via the same `dirs` crate. Windows not a priority.

## Tasks

1. Remove crates.io and docs.rs badges from README (lines 5-6)
2. Remove "From crates.io" installation section from README (lines 26-30)
3. Add GitHub Pages documentation link:
   - Badge in badge row (replacing removed badges)
   - Dedicated "Documentation" section near top
4. Expand "Merging files" section in README with examples for each file type:
   - YAML (existing: CI workflows)
   - JSON (e.g., package.json)
   - TOML (e.g., Cargo.toml)
   - INI (e.g., .editorconfig)
   - Markdown (e.g., README sections)
   - Each example links to full operator docs in user documentation
5. Flesh out operator examples in user docs (`docs/src/`):
   - Ensure each operator has complete examples
   - Match the realistic examples added to README
6. Fix CLI docs discrepancies in `docs/src/cli.md`:
   - Remove `off` from `--log-level` options (line 12)
   - Standardize cache directory default across docs
7. Add user docs location to CLAUDE.md (`docs/src/` - mdBook format)
8. **BUG FIX**: Centralize cache directory default logic
   - Create `default_cache_root()` in shared module (e.g., `src/config.rs` or `src/lib.rs`)
   - Fix `apply.rs` to use `dirs::cache_dir()` like other commands
   - Replace duplicated logic in all 10 command files with shared function
   - Update doc comments to reflect correct default (`~/.cache/common-repo` on Linux)
9. **TECH DEBT**: Full defaults audit
   - Scan entire `src/` directory for duplicated default values
   - Known duplications found so far:
     - Cache dir fallback (`.common-repo-cache`) - 10 places
     - Config file default (`".common-repo.yaml"`) - 7+ command arg definitions
     - Hardcoded config paths (`Path::new(".common-repo.yaml")`) - add.rs, init.rs
   - Create constants or functions for each default
   - Replace all duplications with shared references
   - This prevents future bugs from inconsistent defaults
10. **DOCS**: Remove AI-isms from user documentation
    - Scan all files in `docs/src/` for AI-ism patterns (see Q9 list)
    - Known issues: `introduction.md` has "The Problem" / "The Solution" headers
    - Rewrite affected sections in natural, direct language
    - Replace marketing-speak headers with descriptive ones
    - Goal: docs should read like they were written by a developer, not a chatbot
11. **RESEARCH**: Competitive analysis for feature prioritization (interactive)
    - Research similar projects:
      - copier-org/copier
      - cookiecutter
      - yeoman
      - Other config/template management tools
    - For each project:
      - Document key features common-repo lacks
      - Review Issues/Discussions for most-requested features
      - Note community problems
    - Review findings together to prioritize what fits common-repo
