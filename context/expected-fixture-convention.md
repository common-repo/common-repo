# Convention: `<name>.expected/` byte-exact apply fixtures

A test convention for verifying that `common-repo apply` produces an exact
filesystem result, comparable file-by-file. Use this instead of capturing
output into Rust assertions — those drift away from the spec because the
"expected" values get whatever the implementation happens to produce on first
run.

## Layout

A fixture root contains zero or more sibling directories whose names end in
`.expected`. Each is a self-contained mini-fixture:

- `<name>.expected/.common-repo.yaml` — the input config.
- `<name>.expected/<rel-path>` — every file the apply must produce, byte-exact.

Other siblings of the fixture root (typically the upstream definitions being
inherited) are referenced from the input config via the `__FIXTURE__`
placeholder, which the runner substitutes with the absolute path of the
fixture root before running apply.

Example layout:

```
tests/testdata/some-fixture/
  upstream-base/                      ← upstream definition (consumed via repo:)
    .common-repo.yaml
    merge.yaml
  upstream-base.expected/             ← consumer fixture #1
    .common-repo.yaml                 ← `- repo: { url: __FIXTURE__/upstream-base }`
    merge.yaml                        ← byte-exact expected output
  consumer.expected/                  ← consumer fixture #2
    .common-repo.yaml
    merge.yaml
    .github/workflows/ci.yaml         ← any path under .expected/ is checked
```

`upstream-base/` here exists only as a consumable upstream — apply is never
run there. If a tier needs to be tested both as a definition and as a
consumer of itself, add a separate `<tier>.expected/` whose config does the
self-application.

## Runner

`tests/common/expected_fixture.rs` provides:

```rust
pub fn run_expected_fixtures(fixture_root: &Path);
```

The runner discovers every `*.expected/` directly under `fixture_root` and,
for each:

1. Creates a fresh tempdir.
2. Reads `<name>.expected/.common-repo.yaml`, replaces every literal
   `__FIXTURE__` with the canonicalized absolute path of `fixture_root`,
   writes the result into the tempdir.
3. Runs `common-repo apply` in the tempdir. Failure of apply fails the test.
4. Walks `<name>.expected/` and asserts every file is byte-identical to the
   corresponding tempdir file. Missing files fail with a clear message.
5. Walks the tempdir and fails if any file outside the expected set was
   produced. Catches over-creation.

`.git/` and `.common-repo-cache/` are ignored on both sides. The input
`.common-repo.yaml` is excluded from byte comparison because the runner
templates it.

## Using the runner

A test file is two lines of body:

```rust
mod common;
use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn some_fixture_apply_matches_expected() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("some-fixture");
    run_expected_fixtures(&fixture);
}
```

## Failure output

Mismatches panic with one or more of:

- `missing: <rel> — expected file was not produced by apply`
- `unexpected: <rel> — apply produced a file not listed in the expected fixture`
- `content mismatch: <rel>` followed by byte counts and quoted snippets of
  both sides.

The messages are designed to be eyeballable. For larger diffs, run
`diff -ru` against the tempdir directly while debugging.

## When to use which fixture style

| Style | Use when |
|-------|----------|
| `<name>.expected/` byte-exact | The output is deterministic and the spec defines exactly what files should exist with what content. Best for integration-style tests of merge, rename, include semantics. |
| Captured Rust assertions | Avoid for new work. Treat existing ones as legacy and migrate when convenient. |
| Property-based / shape assertions | When only a property is specified (e.g., "merge.yaml exists and parses as YAML with key X") rather than the full byte sequence. |

## Self-test

`tests/cli_e2e_expected_runner.rs` runs the runner against
`tests/testdata/expected-runner-selftest/`, a minimal two-directory fixture
that consumes a local source via `repo: __FIXTURE__/source`. Confirms both
the happy path and (during initial development) the negative path.

## Extension axes (not yet implemented)

If a tier needs different expected outputs for self-block apply vs.
source-block consumption, the convention can extend to `<name>.exposed/` for
the source-block view. Not needed today — flag it here so we don't reinvent.

If allowlisting non-checked files is needed (e.g., expected metadata that
varies by run), add a `<name>.expected/.expected-allow-extra` file with one
glob per line. Not implemented yet.
