<project_overview>
Rust project. Binary: `common-repo`. Automated code quality, conventional commits, semantic versioning, CI/CD.
</project_overview>

<quick_setup>
Follows [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern:

```bash
./script/setup    # First-time setup (installs deps, configures environment)
./script/test     # Run test suite (uses cargo-nextest)
./script/ci       # Run CI checks locally (no tests)
./script/cibuild  # Run full CI locally (checks + tests)
```

Other scripts: `./script/bootstrap` (install deps), `./script/update` (after pulling changes)

When to use each script:
- `./script/ci` - quick validation before committing (fmt, clippy, pre-commit, security audits)
- `./script/test` - tests only
- `./script/cibuild` - full CI validation (deps update + checks + tests)
</quick_setup>

<agent_session_protocol>
No memory of previous work. Protocol:

1. Verify clean state: `git status`, `git stash list`, check unpushed commits. Ask user about pending changes.
2. Create feature branch: checkout main, pull latest on main, create `<type>/<description>-<session-id>`
3. Start baseline tests: `./script/test` with `run_in_background: true` (skip for docs/context-only changes)
4. Find current task: read `context/current-task.json`
5. Review history: `git log --oneline -5`
6. Execute: first task with `status=pending` and `blocked_by=null`, complete, update status

Background tests: after code changes, check `BashOutput` results. Start new background test after edits.

<context_files>
Key context files:
- `context/current-task.json` - active task/plan
- `context/traceability-map.md` - component-to-docs mapping

Archived (in `context/completed/`): feature-status.json, implementation plans, testing guides

Refs: `context/cli-design.md`, `context/purpose.md`, `context/design.md`, `README.md`

User docs: `docs/src/` mdBook guides at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/)
</context_files>

<task_stash_stack>
On higher-priority interruption:

<stash_push>
Push (preserve current, start new):
1. Rename `current-task.json` to `current-task-stash{N}.json` (skip if null/empty)
2. Create new `current-task.json` with new task
3. Commit and push
4. <critical>Do not start new task without user confirmation - user typically wants fresh sessions</critical>
</stash_push>

<stash_pop>
Pop (resume after completing current):
1. Delete/clear `current-task.json`
2. Rename highest-numbered stash back to `current-task.json`
</stash_pop>

Stack order: highest number = oldest task.
</task_stash_stack>

<archiving_completed_plans>
When all tasks in a plan are complete:
1. `git mv context/<plan>.json context/completed/`
2. Update `current-task.json` to next plan or clear
3. Commit: `chore(context): archive completed <plan-name>`
</archiving_completed_plans>
</agent_session_protocol>

<agent_effectiveness_guidelines>
Per [Anthropic's long-running agent guide](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents):

<rules>
<rule>Follow recommendations precisely. Read entire sources before proposing; no unjustified paraphrasing</rule>
<rule>If corrected, acknowledge and fix. Don't defend substitutions contradicting source</rule>
<rule>One task at a time. No scope creep or doing too much at once</rule>
<rule>Documentation is part of implementation. When adding functionality, update all related docs (module, function, user-facing) in same change. Never defer</rule>
</rules>

<platform_limitations>
AskUserQuestion unavailable on Claude Code iOS. Proceed with reasonable defaults or mention alternatives.
</platform_limitations>

<structured_tracking>
<rules>
<rule>Use JSON for task lists and progress tracking, not YAML/markdown</rule>
<rule>Never substitute formats. If a reference says "use JSON", use JSON</rule>
<rule>Include explicit status fields: `"status": "pending|in_progress|complete"`</rule>
<rule>Add verifiable step-by-step descriptions</rule>
</rules>

<example name="task_plan_structure">
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
    {"id": "dependent-task", "status": "pending", "blocked_by": "task-id-kebab-case", "...": "..."}
  ],
  "notes": [
    "Tasks with blocked_by=null can run in parallel",
    "Update status to complete and last_updated when done"
  ]
}
```
</example>
</structured_tracking>

<plan_to_plan_pattern>
When a task requires reading excessive files/lines that would consume too much context, break into sub-plans.

How it works:
1. Parent plan includes task: `"Create sub-plan for X"` → complete when sub-plan file created
2. Next parent task: `"Complete sub-plan-X.json"` → stays `in_progress`
3. Update `current-task.json` to point to sub-plan
4. Work through sub-plan tasks
5. Sub-plan done: archive it, update `current-task.json` back to parent, mark parent task complete

<example name="parent_plan_tasks">
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
</example>

<rules>
<rule>Sub-plans are functionally separate (no parent references needed)</rule>
<rule>Archive sub-plans separately when complete</rule>
<rule>Nesting depth is unlimited, but confirm with user at 3+ levels</rule>
</rules>
</plan_to_plan_pattern>

<feature_completion_criteria>
<critical>Do not mark features complete prematurely.</critical>

<rules>
<rule>E2E tests exist and pass. Unit tests alone insufficient</rule>
<rule>All acceptance criteria met. Check each explicitly</rule>
<rule>Tests run and pass via `./script/test`. Don't assume</rule>
<rule>Documentation updated for new commands/features</rule>
</rules>
</feature_completion_criteria>
</agent_effectiveness_guidelines>

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run             # Run application
```

<testing>
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

<rules>
<rule>Unit tests during development (fast, no network)</rule>
<rule>Integration tests before major changes</rule>
<rule>Integration tests disabled by default (feature-gated)</rule>
<rule>All tests must pass for CI/CD</rule>
</rules>

Datatest tests for schema parsing auto-discover from YAML files.
Slow test config: `.config/nextest.toml`.
</testing>

<e2e_cli_tests>
E2E tests in `tests/cli_e2e_*.rs`. Use `cargo_bin_cmd!` macro, not deprecated `Command::cargo_bin`:

<example name="correct_e2e_test">
```rust
// CORRECT
use assert_cmd::cargo::cargo_bin_cmd;
let mut cmd = cargo_bin_cmd!("common-repo");
cmd.arg("ls").arg("--help").assert().success();

// WRONG - Command::cargo_bin is deprecated
Command::cargo_bin("common-repo").unwrap()  // Don't use
```
</example>
</e2e_cli_tests>

```bash
cargo xtask coverage                 # HTML report (default)
cargo xtask coverage --format json   # JSON report
cargo xtask coverage --fail-under 80 # Fail if coverage < 80%
cargo xtask coverage --open          # Open report in browser
```

<code_quality>
Pre-commit validation tools:

```bash
./script/ci           # Recommended: all CI checks (no tests)
prek run --all-files  # Alternative: pre-commit hooks only
```

`./script/ci` env vars:
- `SKIP_SECURITY=1` - skip cargo audit/deny
- `SKIP_PROSE=1` - skip prose style check (AI writing patterns)
- `OFFLINE=1` - skip network-dependent checks

Or individually:
```bash
cargo fmt                                              # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo xtask check-prose .                              # Check for AI patterns
cargo test                                             # Run tests
```

Pre-commit hooks (`.pre-commit-config.yaml`): cargo fmt, cargo check, cargo clippy, conventional commit validation, trailing whitespace/YAML checks.

Clippy is strict: all warnings are errors (`-D warnings`).

<common_ci_failures>
- Commit message >100 chars or wrong format
- Code not formatted
- Clippy warnings
- AI writing patterns in documentation (`cargo xtask check-prose .` to check)
</common_ci_failures>
</code_quality>

<commit_message_requirements>
All commits follow conventional commits:

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

<example name="commit_format">
`feat: add user auth`, `fix: resolve memory leak`, `docs: update install instructions`
</example>

Breaking changes: `feat!: description` or `BREAKING CHANGE:` in footer
</commit_message_requirements>

<committing_guidelines>
<rules>
<rule>Run `./script/ci` before every commit. Catches formatting, linting, and prose issues</rule>
<critical>NEVER commit/push without explicit user approval</critical>
<rule>No hardcoded values that change (versions, dates, timestamps) in tests. Use runtime checks</rule>
<rule>When fixing tests: understand what's validated, fix underlying issue, flexible expectations</rule>
<rule>Summaries: 1-2 sentences, no code samples unless requested</rule>
<rule>When reviewing: look for flimsy tests, check for TODOs/stubs</rule>
<rule>Before pushing: rebase on main and resolve conflicts</rule>
</rules>
</committing_guidelines>

<ci_cd_architecture>
CI Pipeline (`.github/workflows/ci.yml`): Lint (pre-commit checks), Test, Rustfmt, Clippy jobs

Commit Linting (`.github/workflows/commitlint.yml`): conventional commit validation in PRs
</ci_cd_architecture>

<documentation_style_guide>
<rules>
<rule>Follow [Rustdoc guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), and [std-dev style guide](https://std-dev-guide.rust-lang.org/development/how-to-write-documentation.html)</rule>
<rule>Avoid AI writing patterns per `context/ai-writing-patterns.md` (phrases to avoid). Scan: `cargo xtask check-prose .`</rule>
<rule>Link files/docs appropriately</rule>
<rule>No emojis or hype language</rule>
<rule>No volatile numbers (versions, coverage %)</rule>
<rule>No line number references</rule>
<rule>Review for consistency and accuracy</rule>
</rules>

<prose_linter>
`check-prose` scans markdown and Rust doc comments for AI patterns:

```bash
cargo xtask check-prose .                    # Check entire project
cargo xtask check-prose docs/src README.md   # Check specific paths
cargo xtask check-prose . --format json      # JSON output for tooling
cargo xtask check-prose . --verbose          # Show files being checked
```

Exits 1 on match (CI-compatible). Full pattern list: `context/ai-writing-patterns.md`.
</prose_linter>
</documentation_style_guide>
