# Repository scripts

- GitHub “Scripts to Rule Them All” layout.
- `setup`: bootstrap hooks and build; `bootstrap`: install developer tools.
- `update`: bootstrap, fetch dependencies, and build; `test`: nextest or cargo test.
- `ci`: non-test checks; `cibuild`: update, checks, and all-feature tests.
- `QUICK=1`/`SKIP_UPDATE=1` skips test updates; `ci` supports `SKIP_PROSE=1`, `SKIP_SECURITY=1`, `OFFLINE=1`.
