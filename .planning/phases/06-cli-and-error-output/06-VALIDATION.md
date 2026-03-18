---
phase: 06
slug: cli-and-error-output
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 06 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo nextest / cargo test |
| **Config file** | .config/nextest.toml |
| **Quick run command** | `cargo nextest run` |
| **Full suite command** | `./script/test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build`
- **After every plan wave:** Run `cargo nextest run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | CLI-01, CLI-02 | build + grep | `cargo build 2>&1 \| tail -5` | ✅ | ⬜ pending |
| 06-01-02 | 01 | 1 | CLI-01, CLI-02, CLI-03 | grep audit | `grep -rn 'source.repo' src/commands/ src/cli.rs src/main.rs src/error.rs` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `common-repo update --help` shows "upstream" | CLI-01 | CLI output check | Run `cargo run -- update --help` and verify "Filter upstreams" appears |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
