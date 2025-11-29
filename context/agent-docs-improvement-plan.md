# Agent Documentation Improvement Plan

Implements recommendations from [Anthropic's "Effective Harnesses for Long-Running Agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents).

## For Agents

**All task tracking is in JSON**: [`agent-docs-improvement-tasks.json`](agent-docs-improvement-tasks.json)

### Session Startup

1. Run `git status`
2. Run `./script/test`
3. Read `agent-docs-improvement-tasks.json`
4. Find first task where `status: "pending"` and `blocked_by: null`
5. Complete it, update status to `"complete"`, update `last_updated`
6. Commit: `docs(agent): [task name]`

### Task Dependencies

```
Tasks 1-4, 6 (parallel, no deps)
       │
       ▼
    Task 5 (blocked by 4)
       │
       ▼
    Task 7 (blocked by 1-6)
       │
       ▼
    Task 8 (validation)
       │
       ▼
    Task 9 (cleanup)
```

## For Humans

This plan improves how AI agents work with this codebase by:

1. Adding `feature-status.json` - structured feature tracking
2. Adding `next-priority.md` - single focused task file
3. Adding session startup protocol to CLAUDE.md
4. Extracting traceability links to reduce token consumption
5. Adding completion checklist emphasizing E2E tests

Once complete, these temporary planning files will be deleted.
