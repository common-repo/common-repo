# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project with automated tooling for code quality, conventional commits, and semantic versioning. The project is configured for modern development practices with comprehensive CI/CD automation.

## Quick Setup

This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern:

```bash
./script/setup    # First-time setup (installs deps, configures environment)
./script/test     # Run test suite (uses cargo-nextest)
./script/cibuild  # Run CI checks locally
```

Other scripts: `./script/bootstrap` (install deps), `./script/update` (after pulling changes)

## Agent Session Protocol

Each session starts with no memory of previous work. Follow this protocol:

1. **Verify clean state**: Run `git status`, `git stash list`, check for unpushed commits. Ask user if any pending changes exist.
2. **Create feature branch**: Checkout main, pull latest changes on main branch, create branch `<type>/<description>-<session-id>`
3. **Start baseline tests**: Run `./script/test` with `run_in_background: true` (skip for docs/context-only changes)
4. **Find current task**: Read `context/current-task.json` for active work
5. **Review recent history**: Run `git log --oneline -5`
6. **Execute**: Find first task where `status=pending` and `blocked_by=null`, complete it, update status

**Using background tests**: After making code changes, use `BashOutput` to check results. Start a new background test run after edits to verify changes.

**Key context files:**
- `context/current-task.json` - Active task and plan file
- `context/traceability-map.md` - Component to documentation mapping

**Archived files** (in `context/completed/`): feature-status.json, implementation plans, testing guides

**Reference docs:** `context/cli-design.md`, `docs/purpose.md`, `docs/design.md`, `README.md`

### Task Stash Stack

When interrupted by higher-priority work:

**Push** (preserve current, start new):
1. Rename `current-task.json` → `current-task-stash{N}.json` (skip if current task is null/empty)
2. Create new `current-task.json` with new task

**Pop** (resume after completing current):
1. Delete/clear `current-task.json`
2. Rename highest-numbered stash back to `current-task.json`

Stack order: highest number = oldest task.

### Archiving Completed Plans

When all tasks in a plan are complete:
1. `git mv context/<plan>.json context/completed/`
2. Update `current-task.json` to next plan or clear
3. Commit: `chore(context): archive completed <plan-name>`

## Agent Effectiveness Guidelines

Based on [Anthropic's "Effective Harnesses for Long-Running Agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents):

- **Follow recommendations precisely** - Read entire sources before proposing solutions; don't paraphrase without justification
- **If corrected, acknowledge and fix** - Don't defend substitutions that contradict the source
- **Work on one task at a time** - Avoid scope creep and doing too much at once

### Use JSON for Structured Tracking

- **Use JSON, not YAML or markdown** for task lists and progress tracking
- **Do not substitute formats** - If a reference says "use JSON", use JSON
- **Include explicit status fields**: `"status": "pending|in_progress|complete"`
- **Add step-by-step descriptions** that can be verified

Example structure:
```json
{
  "plan_name": "Feature or Project Name",
  "last_updated": "YYYY-MM-DD",
  "session_startup": [
    "Run git status to check branch",
    "Run ./script/test to verify baseline",
    "Read this file to find current task",
    "Find first task where status=pending and blocked_by=null"
  ],
  "tasks": [
    {
      "id": "task-id-kebab-case",
      "name": "Human readable task name",
      "status": "pending",
      "priority": 1,
      "blocked_by": null,
      "output_file": "path/to/output.rs",
      "steps": [
        "Concrete step 1 with specific action",
        "Concrete step 2 with verification command"
      ],
      "acceptance_criteria": [
        "File exists at expected path",
        "Tests pass: cargo test test_name",
        "No clippy warnings"
      ]
    },
    {
      "id": "dependent-task",
      "name": "Task that depends on first",
      "status": "pending",
      "priority": 2,
      "blocked_by": "task-id-kebab-case",
      "steps": ["..."],
      "acceptance_criteria": ["..."]
    }
  ],
  "notes": [
    "Tasks with blocked_by=null can run in parallel",
    "Update status to complete and last_updated when done"
  ]
}
```

### Plan-to-Plan Pattern

When a task requires reading excessive files/lines that would consume too much context, break it into sub-plans.

**How it works:**
1. Parent plan includes task: `"Create sub-plan for X"` → complete when sub-plan file created
2. Next parent task: `"Complete sub-plan-X.json"` → stays `in_progress`
3. Update `current-task.json` to point to sub-plan
4. Work through sub-plan tasks
5. When sub-plan is done: archive it, update `current-task.json` back to parent, mark parent task complete

**Example parent plan tasks:**
```json
[
  {
    "id": "create-coverage-subplans",
    "name": "Create sub-plans for each module's test coverage",
    "status": "complete",
    "steps": ["Create context/coverage-config-module.json", "Create context/coverage-git-module.json"]
  },
  {
    "id": "complete-config-coverage",
    "name": "Complete coverage-config-module.json",
    "status": "in_progress",
    "sub_plan": "context/coverage-config-module.json"
  }
]
```

**Rules:**
- Sub-plans are functionally separate (no parent references needed)
- Archive sub-plans separately when complete
- Nesting depth is unlimited, but confirm with the user at 3+ levels deep

### Feature Completion Criteria

Do not mark features complete prematurely:

1. **E2E tests exist and pass** - Unit tests alone are insufficient
2. **All acceptance criteria met** - Check each explicitly
3. **Tests actually run and pass** - Run `./script/test`, don't assume
4. **Documentation updated** - Add new commands/features to relevant docs

## Development Commands

### Building
```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run             # Run application
```

### Testing

Uses cargo-nextest for faster execution (falls back to `cargo test` if unavailable).

```bash
# Unit tests (fast, no network)
cargo nextest run
cargo nextest run test_name              # Specific test
cargo nextest run --profile ci           # Identify slow tests

# Integration tests (requires network)
cargo nextest run --features integration-tests
SKIP_NETWORK_TESTS=1 cargo nextest run --features integration-tests

# Quick runs (skip dependency update)
QUICK=1 ./script/test  # Also: SKIP_UPDATE=1 or CI=1
```

**Important:**
- Use unit tests during development (fast, no network)
- Run integration tests before major changes
- Integration tests are disabled by default (feature-gated)
- Datatest tests for schema parsing auto-discover from YAML files
- All tests must pass for CI/CD to succeed
- Slow test config: `.config/nextest.toml`

### Writing E2E CLI Tests

E2E tests are in `tests/cli_e2e_*.rs`. Use `cargo_bin_cmd!` macro (not deprecated `Command::cargo_bin`):
```rust
// CORRECT
use assert_cmd::cargo::cargo_bin_cmd;
let mut cmd = cargo_bin_cmd!("common-repo");
cmd.arg("ls").arg("--help").assert().success();

// WRONG - Command::cargo_bin is deprecated
Command::cargo_bin("common-repo").unwrap()  // Don't use
```

### Test Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html              # HTML report in target/tarpaulin/
cargo tarpaulin --fail-under 80         # Enforce minimum coverage
```

## Code Quality & Pre-commit

**Before every commit**, run:
```bash
prek run --all-files  # Recommended: runs all checks automatically
# Or: ./script/cibuild  # Run full CI checks locally
```

Or individually:
```bash
cargo fmt                                              # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo test                                             # Run tests
```

Pre-commit hooks (configured in `.pre-commit-config.yaml`) automatically run: cargo fmt, cargo check, cargo clippy, conventional commit validation, trailing whitespace/YAML checks.

**Common CI failures:**
- Commit message >100 chars or wrong format
- Code not formatted
- Clippy warnings

## Commit Message Requirements

All commits must follow **conventional commits**:

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

Examples: `feat: add user auth`, `fix: resolve memory leak`, `docs: update install instructions`

Breaking changes: `feat!: description` or `BREAKING CHANGE:` in footer

## Committing Guidelines for Claude Code

1. **NEVER commit/push without explicit user approval**
2. **Avoid hardcoding values that change** - No version numbers, dates, or timestamps in tests. Use dynamic checks.
3. **When fixing tests** - Understand what's being validated, fix the underlying issue, make expectations flexible
4. **Keep summaries brief** - 1-2 sentences, no code samples unless requested

## CI/CD Architecture

**CI Pipeline** (`.github/workflows/ci.yml`): Lint job (pre-commit checks), Test job, Rustfmt job, Clippy job

**Commit Linting** (`.github/workflows/commitlint.yml`): Validates conventional commit format in PRs

## Documentation Style Guide

- Follow [Rustdoc guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), and [std-dev style guide](https://std-dev-guide.rust-lang.org/development/how-to-write-documentation.html)
- Link to files/documentation appropriately
- No emojis or hype language
- No specific numbers that will change (versions, coverage percentages)
- No line number references
- Review for consistency and accuracy when done

## Important Notes

- Clippy is strict: all warnings are errors (`-D warnings`)
- Binary name is `common-repo`
- When reviewing: look for flimsy tests, check for TODOs/stubs
- Before pushing: rebase on main and resolve conflicts
