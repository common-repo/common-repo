<project_overview>
This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

This is a Rust project with automated tooling for code quality, conventional commits, and semantic versioning. The project is configured for modern development practices with full CI/CD automation.
</project_overview>

<quick_setup>
This project follows the [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all) pattern:

```bash
./script/setup    # First-time setup (installs deps, configures environment)
./script/test     # Run test suite (uses cargo-nextest)
./script/ci       # Run CI checks locally (no tests)
./script/cibuild  # Run full CI locally (checks + tests)
```

Other scripts: `./script/bootstrap` (install deps), `./script/update` (after pulling changes)

<when context="quick_validation_before_commit">`./script/ci` - fmt, clippy, pre-commit, security audits</when>
<when context="run_tests_only">`./script/test`</when>
<when context="full_ci_validation">`./script/cibuild` - update deps + checks + tests</when>
</quick_setup>

<agent_session_protocol>
Each session starts with no memory of previous work. Follow this protocol:

1. Run `git status`, `git stash list`, check for unpushed commits. Ask user if any pending changes exist.
2. Checkout main, pull latest changes on main branch, create branch `<type>/<description>-<session-id>`
3. Run `./script/test` with `run_in_background: true` (skip for docs/context-only changes)
4. Read `context/current-task.json` for active work
5. Run `git log --oneline -5`
6. Find first task where `status=pending` and `blocked_by=null`, complete it, update status

After making code changes, use `BashOutput` to check results. Start a new background test run after edits to verify changes.

<context_files>
- `context/current-task.json` - Active task and plan file
- `context/traceability-map.md` - Component to documentation mapping
- Archived files in `context/completed/`: feature-status.json, implementation plans, testing guides
- Reference docs: `context/cli-design.md`, `context/purpose.md`, `context/design.md`, `README.md`
- User documentation: `docs/src/` contains mdBook-formatted user guides, served at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/)
</context_files>

<task_stash_stack>
When interrupted by higher-priority work:

<stash_push>
1. Rename `current-task.json` to `current-task-stash{N}.json` (skip if current task is null/empty)
2. Create new `current-task.json` with new task
3. Commit and push the change
4. <critical>Do not start the new task without confirmation - user typically wants fresh sessions</critical>
</stash_push>

<stash_pop>
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
Based on [Anthropic's guide to long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents):

<rules>
<rule>Follow recommendations precisely - Read entire sources before proposing solutions; don't paraphrase without justification</rule>
<rule>If corrected, acknowledge and fix - Don't defend substitutions that contradict the source</rule>
<rule>Work on one task at a time - Avoid scope creep and doing too much at once</rule>
<rule>Documentation is part of implementation - When adding functionality, update all related docs (module docs, function docs, user-facing docs) in the same change. Don't defer documentation to later.</rule>
</rules>

<platform_limitations>
<rule>AskUserQuestion tool does not work on Claude Code iOS. Do not use this tool; instead proceed with reasonable defaults or mention alternatives in your response.</rule>
</platform_limitations>

<structured_tracking>
<rules>
<rule>Use JSON, not YAML or markdown for task lists and progress tracking</rule>
<rule>Do not substitute formats - If a reference says "use JSON", use JSON</rule>
<rule>Include explicit status fields: `"status": "pending|in_progress|complete"`</rule>
<rule>Add step-by-step descriptions that can be verified</rule>
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
</example>
</structured_tracking>

<plan_to_plan_pattern>
When a task requires reading excessive files/lines that would consume too much context, break it into sub-plans.

How it works:
1. Parent plan includes task: `"Create sub-plan for X"` → complete when sub-plan file created
2. Next parent task: `"Complete sub-plan-X.json"` → stays `in_progress`
3. Update `current-task.json` to point to sub-plan
4. Work through sub-plan tasks
5. When sub-plan is done: archive it, update `current-task.json` back to parent, mark parent task complete

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
<rule>Nesting depth is unlimited, but confirm with the user at 3+ levels deep</rule>
</rules>
</plan_to_plan_pattern>

<feature_completion_criteria>
<critical>Do not mark features complete prematurely.</critical>

<rules>
<rule>E2E tests exist and pass - Unit tests alone are insufficient</rule>
<rule>All acceptance criteria met - Check each explicitly</rule>
<rule>Tests actually run and pass - Run `./script/test`, don't assume</rule>
<rule>Documentation updated - Add new commands/features to relevant docs</rule>
</rules>
</feature_completion_criteria>
</agent_effectiveness_guidelines>

<development_commands>

<building>
```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run             # Run application
```
</building>

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
<rule>Use unit tests during development (fast, no network)</rule>
<rule>Run integration tests before major changes</rule>
<rule>Integration tests are disabled by default (feature-gated)</rule>
<rule>Datatest tests for schema parsing auto-discover from YAML files</rule>
<rule>All tests must pass for CI/CD to succeed</rule>
<rule>Slow test config: `.config/nextest.toml`</rule>
</rules>
</testing>

<e2e_cli_tests>
E2E tests are in `tests/cli_e2e_*.rs`. Use `cargo_bin_cmd!` macro (not deprecated `Command::cargo_bin`):

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

<test_coverage>
```bash
cargo xtask coverage                 # HTML report (default)
cargo xtask coverage --format json   # JSON report
cargo xtask coverage --fail-under 80 # Fail if coverage < 80%
cargo xtask coverage --open          # Open report in browser
```
</test_coverage>

</development_commands>

<code_quality>
<critical>Before every commit, run:</critical>

```bash
./script/ci           # Recommended: runs all CI checks (no tests)
prek run --all-files  # Alternative: runs pre-commit hooks only
```

Environment variables for `./script/ci`:
- `SKIP_SECURITY=1` - Skip cargo audit and cargo deny checks
- `SKIP_PROSE=1` - Skip prose style check (AI writing patterns)
- `OFFLINE=1` - Skip network-dependent checks

Or individually:
```bash
cargo fmt                                              # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo xtask check-prose .                              # Check for AI patterns
cargo test                                             # Run tests
```

Pre-commit hooks (configured in `.pre-commit-config.yaml`) automatically run: cargo fmt, cargo check, cargo clippy, conventional commit validation, trailing whitespace/YAML checks.

<common_ci_failures>
- Commit message >100 chars or wrong format
- Code not formatted
- Clippy warnings
- AI writing patterns in documentation (run `cargo xtask check-prose .` to check)
</common_ci_failures>
</code_quality>

<commit_message_requirements>
All commits must follow conventional commits:

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
<rule>Run `./script/ci` before every commit - Catches formatting, linting, and prose issues</rule>
<critical>NEVER commit/push without explicit user approval</critical>
<rule>Avoid hardcoding values that change - No version numbers, dates, or timestamps in tests. Use runtime checks.</rule>
<rule>When fixing tests - Understand what's being validated, fix the underlying issue, make expectations flexible</rule>
<rule>Keep summaries brief - 1-2 sentences, no code samples unless requested</rule>
</rules>
</committing_guidelines>

<ci_cd_architecture>
CI Pipeline (`.github/workflows/ci.yml`): Lint job (pre-commit checks), Test job, Rustfmt job, Clippy job

Commit Linting (`.github/workflows/commitlint.yml`): Validates conventional commit format in PRs
</ci_cd_architecture>

<documentation_style_guide>
<rules>
<rule>Follow [Rustdoc guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), and [std-dev style guide](https://std-dev-guide.rust-lang.org/development/how-to-write-documentation.html)</rule>
<rule>Avoid AI writing patterns - See `context/ai-writing-patterns.md` for the list of phrases to avoid. Run `cargo xtask check-prose .` to scan for violations</rule>
<rule>Link to files/documentation appropriately</rule>
<rule>No emojis or hype language</rule>
<rule>No specific numbers that will change (versions, coverage percentages)</rule>
<rule>No line number references</rule>
<rule>Review for consistency and accuracy when done</rule>
</rules>

<prose_linter>
The `check-prose` tool scans markdown files and Rust doc comments for AI writing patterns:

```bash
cargo xtask check-prose .                    # Check entire project
cargo xtask check-prose docs/src README.md   # Check specific paths
cargo xtask check-prose . --format json      # JSON output for tooling
cargo xtask check-prose . --verbose          # Show files being checked
```

The tool exits with code 1 if any patterns are found, making it suitable for CI. See `context/ai-writing-patterns.md` for the full pattern list.
</prose_linter>
</documentation_style_guide>

<important_notes>
<rules>
<rule>Clippy is strict: all warnings are errors (`-D warnings`)</rule>
<rule>Binary name is `common-repo`</rule>
<rule>When reviewing: look for flimsy tests, check for TODOs/stubs</rule>
<rule>Before pushing: rebase on main and resolve conflicts</rule>
</rules>
</important_notes>
