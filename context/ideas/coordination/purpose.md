# Parallel Session Coordination Service

## Problem Statement

Claude Code Web sessions are isolated by design. Each session:
- Can only push to its assigned `claude/*-{session-suffix}` branch
- Cannot access GitHub API write operations (no auth exposed to subprocesses)
- Cannot push to arbitrary refs or branches

The current workflow uses `context/current-task.json` committed to the repository to track which task is active. This creates a bottleneck for parallel work:

1. Session A claims a task by committing to its branch
2. Session A must create a PR and merge to main
3. Only then can Session B see the claim and pick a different task

This forces a **sequential, single-session workflow** when we want **parallel, multi-session execution**.

## Goal

Enable multiple Claude Code Web sessions to work on different tasks in parallel without requiring commits to main for coordination.

## Requirements

1. **Ephemeral coordination** - Claims should not pollute git history or require cleanup before PR merge
2. **Cross-session visibility** - All sessions can see what tasks are claimed
3. **No auth from Web sessions** - Must work with Claude Code Web's network restrictions
4. **Race condition handling** - Graceful handling when two sessions try to claim the same task
5. **Failure recovery** - Stale claims from crashed sessions should not block tasks permanently

## Constraints Discovered

Through investigation of Claude Code Web's environment:

| Capability | Status |
|------------|--------|
| Read GitHub API (public endpoints) | Works |
| Write GitHub API (issues, gists, etc.) | Blocked (401 - no auth) |
| Push to assigned `claude/*` branch | Works |
| Push to other branches/refs | Blocked (403) |
| Access OAuth tokens from subprocess | Not exposed |
| Custom domain allowlisting | Available via environment settings |
| curl/Python requests | Available |

## Solution Approach

Deploy a lightweight external coordination service that:
- Sessions can reach via allowlisted domain
- Requires no authentication (session IDs are unguessable UUIDs)
- Provides simple claim/release API
- Auto-expires stale claims

This sidesteps all the Git/GitHub limitations by using HTTP to an external service.
