# Agent Documentation Improvement Plan

## Overview

This plan implements recommendations from Anthropic's "Effective Harnesses for Long-Running Agents" to improve how AI agents work with this codebase. The goal is to reduce token consumption, provide clearer task prioritization, and establish session startup protocols.

Reference: https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents

## Session Startup Instructions

Before beginning any task in this plan:

1. Run `git status` to check current branch and working state
2. Run `./script/test` to verify baseline functionality
3. Read this file to find your current task
4. Check `context/feature-status.json` (once it exists) for overall progress
5. Review recent commits: `git log --oneline -5`

## Tasks

Each task is independent and can be completed in a single session. Complete tasks in order unless blocked.

---

### Task 1: Create feature-status.json

**Status**: pending

**Description**: Create the structured JSON feature tracking file that agents will use to understand project status at a glance.

**Steps**:
1. Read `context/implementation-progress.md` to extract current feature status
2. Create `context/feature-status.json` with the schema below
3. Populate with all major features from Layers 0-4
4. Mark each feature with correct status (complete/in_progress/pending)
5. Verify the JSON is valid: `python3 -c "import json; json.load(open('context/feature-status.json'))"`

**Schema**:
```json
{
  "last_updated": "YYYY-MM-DD",
  "features": [
    {
      "id": "unique-kebab-case-id",
      "name": "Human readable name",
      "status": "complete|in_progress|pending",
      "layer": "0|1|2|3|3.5|4",
      "priority": 1,
      "steps": [
        "Step 1 description",
        "Step 2 description"
      ],
      "tests": ["test_file.rs"],
      "blocked_by": null
    }
  ]
}
```

**Acceptance Criteria**:
- [ ] File exists at `context/feature-status.json`
- [ ] JSON is valid and parseable
- [ ] All Layer 0-4 features from implementation-progress.md are represented
- [ ] Status matches current implementation state

---

### Task 2: Create next-priority.md

**Status**: pending

**Description**: Create a single-focus file that tells agents exactly what to work on next.

**Steps**:
1. Identify the highest priority incomplete feature from implementation-progress.md
2. Create `context/next-priority.md` with the template below
3. Include specific acceptance criteria
4. Link to relevant design/plan sections

**Template**:
```markdown
# Current Priority

**Feature**: [Feature name]
**ID**: [matches feature-status.json id]
**Layer**: [0-4]

## Description

[1-2 sentence description of what needs to be done]

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Tests pass: `cargo test [test_name]`
- [ ] Clippy clean

## References

- Plan: [link to implementation-plan.md section]
- Design: [link to design.md section]

## Notes

[Any context the next agent needs to know]
```

**Acceptance Criteria**:
- [ ] File exists at `context/next-priority.md`
- [ ] Contains exactly one priority item
- [ ] Has testable acceptance criteria
- [ ] References are valid links

---

### Task 3: Add session startup protocol to CLAUDE.md

**Status**: pending

**Description**: Add a clear "Agent Session Startup" section to CLAUDE.md that agents should follow at the beginning of each session.

**Steps**:
1. Read current CLAUDE.md structure
2. Add new section after "## LLM Context Files" titled "## Agent Session Startup"
3. Include the 5-step startup protocol
4. Reference the new context files (feature-status.json, next-priority.md)

**Content to Add**:
```markdown
## Agent Session Startup

Before beginning work on any task, follow this protocol:

1. **Check state**: Run `git status` to verify branch and working tree
2. **Verify baseline**: Run `./script/test` to ensure tests pass
3. **Read priority**: Check `context/next-priority.md` for current focus
4. **Review progress**: Check `context/feature-status.json` for overall status
5. **Check history**: Run `git log --oneline -5` to see recent work

This protocol catches regressions early and ensures continuity across sessions.
```

**Acceptance Criteria**:
- [ ] Section exists in CLAUDE.md
- [ ] Follows the 5-step protocol from Anthropic article
- [ ] References correct file paths
- [ ] Placed logically in document structure

---

### Task 4: Create traceability-map.md

**Status**: pending

**Description**: Extract all traceability links from implementation-progress.md into a separate reference file to reduce token consumption.

**Steps**:
1. Read implementation-progress.md and identify all "**Traceability**" sections
2. Create `context/traceability-map.md` with a table mapping features to plan/design sections
3. Format as a reference table, not prose

**Format**:
```markdown
# Traceability Map

Quick reference linking implementation components to design and plan documents.

| Component | Plan Section | Design Section |
|-----------|--------------|----------------|
| Layer 0: Config Schema | [0.1 Configuration](implementation-plan.md#01-configuration-schema--parsing) | [Execution Model](../docs/design.md#phase-2-processing-individual-repos) |
| Layer 1: Git Operations | [1.1 Git Operations](implementation-plan.md#11-git-operations) | [Phase 1](../docs/design.md#phase-1-discovery-and-cloning) |
```

**Acceptance Criteria**:
- [ ] File exists at `context/traceability-map.md`
- [ ] All traceability links from implementation-progress.md are captured
- [ ] Links are valid and point to correct sections
- [ ] Format is a scannable table, not prose

---

### Task 5: Slim down implementation-progress.md

**Status**: pending
**Blocked by**: Task 4 (traceability-map.md must exist first)

**Description**: Remove verbose traceability sections and old change history from implementation-progress.md to reduce its size by ~40%.

**Steps**:
1. Verify `context/traceability-map.md` exists and is complete
2. Remove all "**Traceability**" subsections from implementation-progress.md
3. Add a single reference at the top: "For traceability links, see [traceability-map.md](traceability-map.md)"
4. Remove "Recent Changes Summary" entries older than 2 weeks (they're in git history)
5. Keep only: Current Status, Layer completion status, Next Steps, Recent changes (last 2 weeks)

**Acceptance Criteria**:
- [ ] File size reduced by at least 30%
- [ ] No traceability sections remain inline
- [ ] Reference to traceability-map.md exists at top
- [ ] Recent changes limited to last 2 weeks
- [ ] All current status information preserved

---

### Task 6: Add completion checklist to CLAUDE.md

**Status**: pending

**Description**: Add a feature completion checklist that agents must verify before marking any feature complete.

**Steps**:
1. Read current CLAUDE.md "Pre-Commit Checklist" section
2. Add new section "## Feature Completion Checklist" after it
3. Include E2E testing requirement (key point from Anthropic article)

**Content to Add**:
```markdown
## Feature Completion Checklist

Before marking any feature as complete in `context/feature-status.json`:

1. **Unit tests exist and pass**: `cargo test [feature_tests]`
2. **E2E tests exist and pass**: Features must have end-to-end tests, not just unit tests
3. **Code quality**: `cargo clippy --all-targets --all-features -- -D warnings`
4. **Formatting**: `cargo fmt --check`
5. **Documentation**: Update implementation-progress.md with completion status
6. **Feature status**: Update `context/feature-status.json` status to "complete"

**Important**: Do not mark features complete based on unit tests alone. The Anthropic research shows agents tend to declare completion prematurely. Always verify with E2E tests.
```

**Acceptance Criteria**:
- [ ] Section exists in CLAUDE.md
- [ ] Emphasizes E2E testing requirement
- [ ] References feature-status.json
- [ ] Placed after Pre-Commit Checklist

---

### Task 7: Update LLM Context Files section in CLAUDE.md

**Status**: pending
**Blocked by**: Tasks 1-6 (all new files must exist)

**Description**: Update the "LLM Context Files" section to reference all new context files.

**Steps**:
1. Read current "LLM Context Files" section in CLAUDE.md
2. Add entries for new files:
   - `context/feature-status.json` - Structured feature tracking for agents
   - `context/next-priority.md` - Current priority task for agents
   - `context/traceability-map.md` - Links between implementation and design docs
3. Update description to emphasize agent workflow

**Acceptance Criteria**:
- [ ] All new context files are listed
- [ ] Descriptions are accurate and concise
- [ ] Section reflects agent-first workflow

---

### Task 8: Validate and test the new workflow

**Status**: pending
**Blocked by**: Task 7

**Description**: Verify that the new documentation structure works by simulating an agent session startup.

**Steps**:
1. Follow the new "Agent Session Startup" protocol exactly
2. Verify all referenced files exist and are readable
3. Verify feature-status.json is valid JSON
4. Verify next-priority.md has actionable content
5. Verify traceability-map.md links are valid
6. Document any issues found

**Acceptance Criteria**:
- [ ] All 5 startup protocol steps complete without errors
- [ ] feature-status.json parses correctly
- [ ] All file references resolve
- [ ] No broken links in traceability-map.md

---

## Progress Tracking

Update this section as tasks are completed:

| Task | Status | Completed By | Date |
|------|--------|--------------|------|
| 1. feature-status.json | pending | | |
| 2. next-priority.md | pending | | |
| 3. Session startup in CLAUDE.md | pending | | |
| 4. traceability-map.md | pending | | |
| 5. Slim implementation-progress.md | pending | | |
| 6. Completion checklist | pending | | |
| 7. Update LLM Context Files | pending | | |
| 8. Validate workflow | pending | | |

## Notes for Future Sessions

- Tasks 1-4 and 6 can be done in parallel (no dependencies)
- Task 5 requires Task 4 to be complete
- Task 7 requires Tasks 1-6 to be complete
- Task 8 is the final validation step
- Each task should take 10-20 minutes
- Commit after each task with message format: `docs(agent): [task description]`
