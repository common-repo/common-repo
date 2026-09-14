# GitHub automation

- `workflows/ci.yaml` — Rust tests, fmt, clippy, MSRV, security, coverage; reusable by release CI.
- `workflows/release.yaml` — inherited Cocogitto release pipeline; `release-binaries.yaml` builds and attaches platform archives after release publication.
- `workflows/pre-commit.yaml`, `conventional-commits.yaml` — inherited hook and commit checks. Trace local customizations to root `.common-repo.yaml` and `.common-repo/` fragments.
- `workflows/test-action.yml` — tests root `action.yml`; `benchmark.yml` — Criterion; `docs.yml` — mdBook publication.
- `workflows/auto-merge.yml` — rebase merge triggered by the `ready to merge` label.
