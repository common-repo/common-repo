---
phase: 07
slug: test-updates
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 07 — Validation Strategy

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

- **After every task commit:** Run `cargo nextest run` (compilation + test pass)
- **After every plan wave:** Run `./script/test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | TEST-01 | compile + grep | `cargo test --test cli_e2e_upstream_ops --no-run` | ✅ | ⬜ pending |
| 07-01-02 | 01 | 1 | TEST-02 | compile + run | `cargo nextest run` | ✅ | ⬜ pending |
| 07-02-01 | 02 | 1 | TEST-02 | compile + run | `cargo nextest run --test cli_e2e_update --test cli_e2e_defer` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
