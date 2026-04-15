# Changelog

## [0.34.3](https://github.com/common-repo/common-repo/compare/v0.34.2...v0.34.3) (2026-04-15)


### Bug Fixes

* unify source and self: pipelines to use the same sequential code path ([2c4d1e9](https://github.com/common-repo/common-repo/commit/2c4d1e9459aebf5aaf38d6a58fb92ccf19c636f3)), closes [#305](https://github.com/common-repo/common-repo/issues/305)

## [0.34.2](https://github.com/common-repo/common-repo/compare/v0.34.1...v0.34.2) (2026-04-14)


### Bug Fixes

* auto-merge with defer:true in sequential (self:) pipelines ([b172b2a](https://github.com/common-repo/common-repo/commit/b172b2a03943ef882e13b0e35fb2347155ad40ab))
* update e2e tests to not hardcode upstream minor version ([f0a575b](https://github.com/common-repo/common-repo/commit/f0a575b52e136969376a007cb9e00c9e48dae7c9))

## [0.34.1](https://github.com/common-repo/common-repo/compare/v0.34.0...v0.34.1) (2026-04-14)


### Bug Fixes

* resolve XML merge parity gaps and update stale spec annotations ([e5d6982](https://github.com/common-repo/common-repo/commit/e5d6982a75e7307bb7cf6a33f97a32218e490eb4))

## [0.34.0](https://github.com/common-repo/common-repo/compare/v0.33.0...v0.34.0) (2026-04-14)


### ⚠ BREAKING CHANGES

* Local dotfiles are no longer excluded from the merge phase. Projects with dotfiles in the working directory will now see them loaded into the pipeline. This aligns with the spec and with common-repo's core purpose of managing dotfiles.

### Features

* add integrate_sub_composite for incremental composite construction ([9afc2c9](https://github.com/common-repo/common-repo/commit/9afc2c9a626890988b773bea9385fe674bc463cc))
* add resolve_repo_inline for on-demand sub-repo processing ([bfd2be8](https://github.com/common-repo/common-repo/commit/bfd2be8c196e81f6fabd721066fb7693a6eeb239))
* add sequential-context hints to merge error messages ([ba657df](https://github.com/common-repo/common-repo/commit/ba657df720c4f7e1dc576ecfae069d3fb082c189))
* wire sequential pipeline for self: blocks with inline repo resolution ([9304e91](https://github.com/common-repo/common-repo/commit/9304e913ee0bfe02405b9a9d44bce625688caa18))


### Bug Fixes

* address code review findings across phases module ([15e556b](https://github.com/common-repo/common-repo/commit/15e556bc064145e446b651e2fd56b886c4c7e017))
* chain rename mappings per RenameEquivalence spec invariant ([97a7e5b](https://github.com/common-repo/common-repo/commit/97a7e5bad4c65f44108729abe62c3a9e30d34aca))
* load dotfiles in local_merge phase instead of blanket-skipping ([5152685](https://github.com/common-repo/common-repo/commit/51526859c82aad85df04d1a5d3b4a225edc49a83))
* resolve nested repo: tree children in inline resolution ([d68a4e2](https://github.com/common-repo/common-repo/commit/d68a4e2211d42bef099651cbad6b0642508f9bf6))

## [0.33.0](https://github.com/common-repo/common-repo/compare/v0.32.2...v0.33.0) (2026-04-14)


### ⚠ BREAKING CHANGES

* Consumer operations now execute in declaration order. Configs that relied on merges always running before filters (regardless of declaration order) must be reordered so merges appear before the filters that would remove their source files.

### Features

* sequential declaration-order execution for Phase 5 consumer operations ([dc227ab](https://github.com/common-repo/common-repo/commit/dc227abd10500ba527689798fa02bc02748559d4))

## [0.32.2](https://github.com/common-repo/common-repo/compare/v0.32.1...v0.32.2) (2026-04-07)


### Bug Fixes

* **ci:** don't fail ci due to race conditions on main ([e61f033](https://github.com/common-repo/common-repo/commit/e61f033e33beda5df1843493bb9e7ef31a3eca44))

## [0.32.1](https://github.com/common-repo/common-repo/compare/v0.32.0...v0.32.1) (2026-04-04)


### Bug Fixes

* **#284:** use AppendUnique for inter-repo collision and filter deferred ops ([dbf4a26](https://github.com/common-repo/common-repo/commit/dbf4a26681f1af72b73d5e6f97545d6ebe4080c2))
* merge auto-merge files across chained repos instead of overwriting ([#284](https://github.com/common-repo/common-repo/issues/284)) ([5958869](https://github.com/common-repo/common-repo/commit/59588690b64abb071a336d8429235cf289b7300a))
* remove duplicate test functions ([12e3892](https://github.com/common-repo/common-repo/commit/12e38929f8656ac49e630857da9faea400b15970))
* **test:** fix config parse error and remove hardcoded version ([c57b7db](https://github.com/common-repo/common-repo/commit/c57b7dbdb919603ef3c16cc8b26439823a2ddc55))
* **test:** fix false-positive vars: assertion (template-vars: contains vars:) ([91a4253](https://github.com/common-repo/common-repo/commit/91a42534d9137646c7e96f382fa39f013ce98601))
* **update:** preserve YAML structure and update all ref occurrences ([62ad17b](https://github.com/common-repo/common-repo/commit/62ad17b37a1ec8385755e218f817503ec157d22d))

## [0.32.0](https://github.com/common-repo/common-repo/compare/v0.31.1...v0.32.0) (2026-03-30)


### Features

* **merge:** add XML merge operator with spec compliance fixes ([0224858](https://github.com/common-repo/common-repo/commit/02248585ccff21bd838f3a7f906960f9bb21eddc))


### Bug Fixes

* collect only deferred merge operations per spec ([6325de7](https://github.com/common-repo/common-repo/commit/6325de71d613ddda968351c58ae43135d7b137c7)), closes [#268](https://github.com/common-repo/common-repo/issues/268)

## [0.31.1](https://github.com/common-repo/common-repo/compare/v0.31.0...v0.31.1) (2026-03-29)


### Bug Fixes

* use init_git_repo helper in alternate config test ([384b6d9](https://github.com/common-repo/common-repo/commit/384b6d9e75dd2838a255ca653a26eba0f230a469)), closes [#257](https://github.com/common-repo/common-repo/issues/257)

## [0.31.0](https://github.com/common-repo/common-repo/compare/v0.30.0...v0.31.0) (2026-03-29)


### ⚠ BREAKING CHANGES

* Template variables now use __COMMON_REPO__VAR__ syntax. ${VAR} and ${VAR:-default} are no longer interpreted. Environment variable fallback removed — all variables must be declared in template-vars.
* **config:** YAML configs using `append: true/false` on yaml/json/toml merge operations must migrate to `array_mode: append` or `array_mode: replace`.

### Features

* add migration warning for legacy ${VAR} template syntax ([7c72d32](https://github.com/common-repo/common-repo/commit/7c72d32a2f3610cbec6c27a445563d8e73d3976f))
* migrate template syntax to __COMMON_REPO__VAR__ ([5a4e6e6](https://github.com/common-repo/common-repo/commit/5a4e6e652afe877108b3bcf289c9d33df5a1ba33))
* **spec:** add CompositePrecedence invariant and MergeLocalFiles rule ([13cc685](https://github.com/common-repo/common-repo/commit/13cc6857584761ef6d0d6387948358e695e87ce1)), closes [#265](https://github.com/common-repo/common-repo/issues/265)
* validate template-vars names against identifier pattern ([71d4a91](https://github.com/common-repo/common-repo/commit/71d4a91963482d77af352feb6e3ff9a36ee40ab0))


### Bug Fixes

* **local_merge:** composite wins, deferred merges, enforce operation ordering ([d826c0d](https://github.com/common-repo/common-repo/commit/d826c0d57a9af2c47d2843fc5eba360fa75599da))
* replace legacy ${VAR} literals in template-vars values and benchmarks ([4c6d096](https://github.com/common-repo/common-repo/commit/4c6d096f814f1193eb017ca9943ced587ae0ddcf))
* update repo_tests template strings to new __COMMON_REPO__ syntax ([87d05f4](https://github.com/common-repo/common-repo/commit/87d05f468cb3ac110119f74182e0aec16ef5dab3))


### Code Refactoring

* **config:** remove deprecated append field from yaml/json/toml merge ops ([1b0759f](https://github.com/common-repo/common-repo/commit/1b0759f74956f5f3688d6b88f86c288db4925db0))

## [0.30.0](https://github.com/common-repo/common-repo/compare/v0.29.0...v0.30.0) (2026-03-24)


### Features

* add Rust TDD guide and make TDD the default development approach ([985f4c2](https://github.com/common-repo/common-repo/commit/985f4c21f433ca776d7cccf89fd978f64142d860))
* **commands:** handle Self_ in update and check commands ([f769c44](https://github.com/common-repo/common-repo/commit/f769c44d0f9150962a77c6018b2cedcebcea7407))
* **config:** add InsertPosition enum for typed position handling ([a3696a7](https://github.com/common-repo/common-repo/commit/a3696a7df5dae93a8d86ca8d91fa7ce2ed10d31d))
* **config:** add Self_ variant and SelfOp struct for self: operator ([9e90974](https://github.com/common-repo/common-repo/commit/9e9097426938fd4f8cf38d0048c00f8197a78a9d))
* **config:** add self: to original format parser ([075fc3b](https://github.com/common-repo/common-repo/commit/075fc3bc2cbea858a656721228e33a4c5605e5c1))
* **config:** validate self: blocks — reject nesting and empty blocks ([0286121](https://github.com/common-repo/common-repo/commit/0286121a0d5e11fd3cd9ff5b7a7071d09181c259))
* handle Self_ in version checking and info command ([b2fc462](https://github.com/common-repo/common-repo/commit/b2fc4620ae95491050f4a3df3e2c3e71d6afb69b))
* **merge:** add position support to TOML merge operations ([7c61e1a](https://github.com/common-repo/common-repo/commit/7c61e1a0deb7e7c069928e9bf138168326986cae))
* **merge:** add position support to YAML merge operations ([dbce41a](https://github.com/common-repo/common-repo/commit/dbce41a30b35525b75ba0d8f1145038f44d4933f))
* **merge:** migrate JSON merge to ArrayMergeMode + InsertPosition ([c5088ed](https://github.com/common-repo/common-repo/commit/c5088ed18a9d7694128f8fabab17f724df330af6))
* **orchestrator:** execute self: blocks as isolated pipelines ([9511a04](https://github.com/common-repo/common-repo/commit/9511a041775ae36a693ae15e708283d35ab205ef)), closes [#253](https://github.com/common-repo/common-repo/issues/253)


### Bug Fixes

* **spec:** resolve syntax errors and align spec with implementation ([b9e8e13](https://github.com/common-repo/common-repo/commit/b9e8e13f2ef2343c8f14d44ee1f4af09236d2fc9))

## [0.29.0](https://github.com/common-repo/common-repo/compare/v0.28.5...v0.29.0) (2026-03-19)


### Features

* **06-01:** update CLI help and output to use upstream terminology ([0f78454](https://github.com/common-repo/common-repo/commit/0f78454c4b2db36b3413b33cd2fe137d9cbaec72))


### Bug Fixes

* **06:** revise plan based on checker feedback ([cdefcd1](https://github.com/common-repo/common-repo/commit/cdefcd16fcb8fa5d9a9242b10b05a42d4856f4bf))
* **07-02:** update test assertions from source to upstream terminology ([d4ac3ed](https://github.com/common-repo/common-repo/commit/d4ac3ed417ef4f6cb17226249a0dcb8daa438122))
* **08-01:** fix prose lint violation in action design spec ([fc5d7b3](https://github.com/common-repo/common-repo/commit/fc5d7b38cd69505d50f3476cdbe03c6830ec4c02))
* **08-02:** rename remaining source repo references to upstream ([a1058cd](https://github.com/common-repo/common-repo/commit/a1058cd8a730ff51737f52f162e92f3efdbad8aa))

## [0.28.5](https://github.com/common-repo/common-repo/compare/v0.28.4...v0.28.5) (2026-03-15)


### Bug Fixes

* **processing:** use operations-aware keys so consumer template-vars overrides are not dropped ([5dca64e](https://github.com/common-repo/common-repo/commit/5dca64e3590e7ecaec95e032bec2919cdcea61d5)), closes [#249](https://github.com/common-repo/common-repo/issues/249)

## [0.28.4](https://github.com/common-repo/common-repo/compare/v0.28.3...v0.28.4) (2026-03-15)


### Bug Fixes

* **ls:** correct empty-state message to include modified files ([39d8cb8](https://github.com/common-repo/common-repo/commit/39d8cb839a45cccc8047f2252381fceb75b64873))

## [0.28.3](https://github.com/common-repo/common-repo/compare/v0.28.2...v0.28.3) (2026-03-15)


### Bug Fixes

* **template:** skip ${{ }} GitHub Actions expressions in variable substitution ([90f351f](https://github.com/common-repo/common-repo/commit/90f351fe096053525a34da8e1b9a1cb4094b186d)), closes [#239](https://github.com/common-repo/common-repo/issues/239)

## [0.28.2](https://github.com/common-repo/common-repo/compare/v0.28.1...v0.28.2) (2026-03-15)


### Bug Fixes

* **discovery:** pass template and template-vars from source repos to consumers ([ba88086](https://github.com/common-repo/common-repo/commit/ba880864c9d441323dd5940e79d1845656c3b3c4)), closes [#235](https://github.com/common-repo/common-repo/issues/235)

## [0.28.1](https://github.com/common-repo/common-repo/compare/v0.28.0...v0.28.1) (2026-03-14)


### Bug Fixes

* **ls:** respect exclude filters in ls output ([#226](https://github.com/common-repo/common-repo/issues/226)) ([701b1b7](https://github.com/common-repo/common-repo/commit/701b1b7e59e3ff86ddcde1161d01261e57d924a9))
* **ls:** restore --working-dir flag and remove duplicate orchestrator ([2987020](https://github.com/common-repo/common-repo/commit/29870207019cf330653b328b448b2c2bd1468050))

## [0.28.0](https://github.com/common-repo/common-repo/compare/v0.27.0...v0.28.0) (2026-01-21)


### Features

* add GitHub Action for automated upstream sync ([2b2f5bf](https://github.com/common-repo/common-repo/commit/2b2f5bfd1f48025e8cadb6a1ee423335298fc6a9))


### Bug Fixes

* **bootstrap:** add action-validator installation ([d7d8a3e](https://github.com/common-repo/common-repo/commit/d7d8a3e16e7f2a44e47dfb1be1d30941e0be6fe2))
* **ci:** install action-validator for pre-commit ([b9dac0b](https://github.com/common-repo/common-repo/commit/b9dac0b03e3966577c740bc9eec19ca86dc28fbf))
* **phases:** respect source repo's filtering operations and auto-exclude config files ([c404843](https://github.com/common-repo/common-repo/commit/c404843a6147507a170890cc88e1dc89d761fda1))
* **schema:** remove patterns subkey from include/exclude/template ops ([fb4e27e](https://github.com/common-repo/common-repo/commit/fb4e27ed5e2590a19cb0de97f05f5919b5229142))
* **testdata:** update fixture configs to new schema format ([832ab4f](https://github.com/common-repo/common-repo/commit/832ab4f71309f02b3c9db9fc6aa239a7aabaa66a))

## [0.27.0](https://github.com/common-repo/common-repo/compare/v0.26.1...v0.27.0) (2026-01-08)


### Features

* **xtask:** add context flag and improve prose pattern detection ([306841c](https://github.com/common-repo/common-repo/commit/306841c4892c2968a929068a5c913f3633013e6d))


### Bug Fixes

* **xtask:** exclude ai-writing-patterns.md and LICENSE.md from prose check ([dd03785](https://github.com/common-repo/common-repo/commit/dd03785106f64ef667430e9e8e2bc482b0ea303c))

## [0.26.1](https://github.com/common-repo/common-repo/compare/v0.26.0...v0.26.1) (2026-01-08)


### Bug Fixes

* **test:** handle conditional pre-commit CLI prompt in interactive test ([da79040](https://github.com/common-repo/common-repo/commit/da79040a63b1147b29dbe8e912abe04d04f821a4))


### Performance Improvements

* add criterion benchmarks for core operations ([9ea851a](https://github.com/common-repo/common-repo/commit/9ea851af889adb2bfd9c3f4965e1300e74c4dce5))
* add minimal-size build profile ([550b136](https://github.com/common-repo/common-repo/commit/550b1366a0d99636b12cf203935769a092c7b54b))

## [0.26.0](https://github.com/common-repo/common-repo/compare/v0.25.0...v0.26.0) (2026-01-08)


### Features

* **update:** add --filter flag for selective source updates ([d512a66](https://github.com/common-repo/common-repo/commit/d512a6634f898da2651c28f0c607b6cfeb9a942f))


### Performance Improvements

* add release profile optimizations ([a693afc](https://github.com/common-repo/common-repo/commit/a693afcf39c549782d5051a4dc35b498dfe13fac))

## [0.25.0](https://github.com/common-repo/common-repo/compare/v0.24.0...v0.25.0) (2026-01-07)


### Features

* **ci:** integrate prose linter into CI checks ([c415ad2](https://github.com/common-repo/common-repo/commit/c415ad2135b5d620dc30922fb982c857ad2b5bca))
* **xtask:** add check-prose command structure ([d85bf5b](https://github.com/common-repo/common-repo/commit/d85bf5baa867a0b8d10c64b9d96693722e0a3e1b))
* **xtask:** add pattern data structures for prose linter ([b58fbec](https://github.com/common-repo/common-repo/commit/b58fbec8bc31d6ff9826ffb5c0b44ec48f4e9511))
* **xtask:** implement file scanning and pattern matching for prose linter ([4793b1a](https://github.com/common-repo/common-repo/commit/4793b1a1b76b883b321e3074ead7fc24ecbc41ca))


### Bug Fixes

* **ci:** auto-merge workflow was waiting for itself to complete ([e8b2833](https://github.com/common-repo/common-repo/commit/e8b2833ef00eeb7988db76b21e25b5428ebb2ff5))

## [0.24.0](https://github.com/common-repo/common-repo/compare/v0.23.0...v0.24.0) (2026-01-03)


### Features

* add --verbose and --quiet global flags ([b569cf9](https://github.com/common-repo/common-repo/commit/b569cf918685a30d51eaa9acb591b0a6e9b1d4b1))
* add helpful suggestions to error messages ([1bde6a6](https://github.com/common-repo/common-repo/commit/1bde6a63e3fac1811eced3f74e838e10b5468058))
* audit and standardize CLI exit codes ([d5121ad](https://github.com/common-repo/common-repo/commit/d5121ad7598449a8f692c0defaa1aabd0638826d))
* **output:** add TTY-aware output with NO_COLOR support ([1b4a5a8](https://github.com/common-repo/common-repo/commit/1b4a5a885a98ff63d05b88e517eb247ea048f77c))
* **workflow:** wait for in-progress checks before auto-merge ([84a3d0b](https://github.com/common-repo/common-repo/commit/84a3d0b36c2a92ebd74da7d8e841ba1e6fa5acc8))


### Bug Fixes

* centralize cache_root default logic using shared defaults module ([d40e5d7](https://github.com/common-repo/common-repo/commit/d40e5d770ad22adfe9d1e2b10c41929a88dfd32a))
* **ci:** also exclude self from failed checks count in auto-merge ([c5dd0a7](https://github.com/common-repo/common-repo/commit/c5dd0a7b631faf887f61c0ac7c395bcc3d0318c3))
* **ci:** exclude self from pending checks in auto-merge workflow ([4144d7f](https://github.com/common-repo/common-repo/commit/4144d7f8914d2436a410ae64665a9d879623729f))
* resolve verbose/quiet flag conflicts between global and local ([20b32bb](https://github.com/common-repo/common-repo/commit/20b32bb38fac090c32ee9f9ca13d174bdb7824e5))
* **test:** add --color=always to tree E2E tests for CI compatibility ([ec9f80b](https://github.com/common-repo/common-repo/commit/ec9f80b0a9bfdb421c73bc0939979087fe279fde))

## [0.23.0](https://github.com/common-repo/common-repo/compare/v0.22.0...v0.23.0) (2026-01-01)


### Features

* **cli:** add shell completions command ([8d58636](https://github.com/common-repo/common-repo/commit/8d58636d8f2c3d116521df06e0003490464b03a7))
* **error:** add helpful hints to error messages ([dad6c06](https://github.com/common-repo/common-repo/commit/dad6c06c2fb74da61cb902a74726d765ecc5e551))
* **workflow:** add check status verification before enabling auto-merge ([c0fa040](https://github.com/common-repo/common-repo/commit/c0fa040cc1bca2d24268b28e0ed7ce6124a16ec1))
* **xtask:** add cargo xtask automation with coverage and release-prep ([30e8b5e](https://github.com/common-repo/common-repo/commit/30e8b5eb06b71a0623bee334ad6b69cdb95cff2f))


### Bug Fixes

* **workflow:** use --auto flag for auto-merge to respect branch protection ([ce4501a](https://github.com/common-repo/common-repo/commit/ce4501ab97db3acbd4fd0e08eb88dc49b7ad4492))
* **workflow:** use rebase instead of squash for auto-merge ([c02c1ec](https://github.com/common-repo/common-repo/commit/c02c1ec6e72291d11e2b8ed74a80444f49048407))

## [0.22.0](https://github.com/common-repo/common-repo/compare/v0.21.1...v0.22.0) (2026-01-01)


### Features

* **add:** implement add command for quick repo addition ([08a6724](https://github.com/common-repo/common-repo/commit/08a6724d6045a9938ae47f3d4fad5170f680be41))
* **config:** add defer and auto-merge fields to merge operators ([9e9a3b5](https://github.com/common-repo/common-repo/commit/9e9a3b507f71443c2e26451e14315ac9dfb10870))
* **config:** add fluent builder pattern for merge operations ([a1396e5](https://github.com/common-repo/common-repo/commit/a1396e57ee2d177309ffefb490122d731c1003c5))
* **context:** add source-declared merge implementation plan ([dc6b87c](https://github.com/common-repo/common-repo/commit/dc6b87cf5c58b00e0b7ff92598572e060745e0c9))
* **deps:** add dialoguer for interactive CLI prompts ([c07787f](https://github.com/common-repo/common-repo/commit/c07787f3c1781f791191bea8c8d650fba53eece1))
* **design:** add auto-merge shorthand for source-declared merge ([ff05d23](https://github.com/common-repo/common-repo/commit/ff05d23e4a00ee8d1f0903f1272855b1802c1b9f))
* **init:** add pre-commit hook setup to interactive wizard ([15866d7](https://github.com/common-repo/common-repo/commit/15866d7aff5588201cf09301d2712e2ba71d57f4))
* **init:** add URI positional argument for quick initialization ([d18c37c](https://github.com/common-repo/common-repo/commit/d18c37c7e469024b9ad04f2c5d5c361b72d65854))
* **init:** implement interactive wizard with dialoguer ([77604ff](https://github.com/common-repo/common-repo/commit/77604ffb6b766551f1397279b4cc56a9082e8c46))
* **install:** add cr short alias for common-repo ([4e0bbe3](https://github.com/common-repo/common-repo/commit/4e0bbe33f8d97e2ebe3095b62fed4a24f4cd3211))
* **install:** add optional prek installation to install script ([40b6251](https://github.com/common-repo/common-repo/commit/40b62510057cf4dda455d467d91897920ee41a39))
* **phases:** apply deferred operations from source repos ([f397dd5](https://github.com/common-repo/common-repo/commit/f397dd5151bb861a2417efb9d4c29a14a6f1f44d))
* **scripts:** add script/ci for local CI checks ([5930b98](https://github.com/common-repo/common-repo/commit/5930b981346fb9035abac16602c03f9b860e3137))


### Bug Fixes

* **ci:** ignore unmaintained transitive dependencies in security audits ([f4248b4](https://github.com/common-repo/common-repo/commit/f4248b458fcc406823f0ebd5a9602bffc6c4f5dd))
* **ci:** simplify deny.toml and remove unused ring clarification ([0bc069e](https://github.com/common-repo/common-repo/commit/0bc069e62818390f113cfb64d81142805f001954))
* **ci:** update MSRV to 1.85.0 for edition2024 support ([2d8b886](https://github.com/common-repo/common-repo/commit/2d8b8862c0104742717baed3a1ad17446e9c971a))
* **ci:** use version tags instead of SHA hashes for actions ([f24559f](https://github.com/common-repo/common-repo/commit/f24559f9ec1e0b31579742816b72d219832c9e23))
* **docs:** update merge module doc examples for private modules ([b9cabd0](https://github.com/common-repo/common-repo/commit/b9cabd0abac1f3fb10393cc589008144fb5ea176))
* **merge:** add auto-merge validation and make TOML path optional ([488610d](https://github.com/common-repo/common-repo/commit/488610d44d69e3d5fe5cdc616fa798e83bec7fa9))
* **test:** skip interactive mode E2E test that requires TTY ([f0cbefe](https://github.com/common-repo/common-repo/commit/f0cbefeaaba2c1b15d01ba9ecb3e8e8110b636ef))
* **test:** update interactive E2E tests for pre-commit prompt ([9afaf1a](https://github.com/common-repo/common-repo/commit/9afaf1a849ff7f2ec961928cc1a6e70d0456032c))

## [0.21.1](https://github.com/common-repo/common-repo/compare/v0.21.0...v0.21.1) (2025-12-08)


### Bug Fixes

* remove hooks ([d4a9fbd](https://github.com/common-repo/common-repo/commit/d4a9fbd0e67db386d8cb7846e56e09694f63daa8))

## [0.21.0](https://github.com/common-repo/common-repo/compare/v0.20.0...v0.21.0) (2025-12-03)


### Features

* add install.sh for easy one-liner installation ([452aa42](https://github.com/common-repo/common-repo/commit/452aa4254e76cbb201a0aaf8d4582fefd09d1d12))

## [0.20.0](https://github.com/common-repo/common-repo/compare/v0.19.0...v0.20.0) (2025-12-03)


### Features

* **ci:** add precompiled binary publishing to releases ([f02461d](https://github.com/common-repo/common-repo/commit/f02461da73115ae7e7db04896ea72283039aea63))


### Performance Improvements

* speed up tarpaulin coverage from ~25min to ~2min ([77666c5](https://github.com/common-repo/common-repo/commit/77666c5320fd8fd1382cbc47477288c2c1379d3c))

## [0.19.0](https://github.com/common-repo/common-repo/compare/v0.18.0...v0.19.0) (2025-12-02)


### Features

* add pre-commit hook for common-repo update ([bd8aaf8](https://github.com/common-repo/common-repo/commit/bd8aaf8a63449f223af294949e073a8dd9271ea3))
* add sync-check and hooks commands for dependency sync ([7bd2d3b](https://github.com/common-repo/common-repo/commit/7bd2d3bf886ea11b9d266c108dd94067751f846f))
* **phase1:** implement parallel cloning using rayon ([7862833](https://github.com/common-repo/common-repo/commit/78628332f7e9fecdbd4cff83fa4d4373f5f750d7))


### Bug Fixes

* **context:** add required E2E tests for parallel cloning ([d1d034f](https://github.com/common-repo/common-repo/commit/d1d034f56890fb10c1c0f11212040670efad673a))
* **context:** make parallel cloning the default behavior in plan ([9d34646](https://github.com/common-repo/common-repo/commit/9d346465ecbb3b51177bde6851862d4afc09e1e2))
* **test:** update integration test for archived file ([748147d](https://github.com/common-repo/common-repo/commit/748147d4db53041f440069ade82a4837ff699903))

## [0.18.0](https://github.com/common-repo/common-repo/compare/v0.17.0...v0.18.0) (2025-12-01)


### Features

* **tests:** add INI merge operation tests ([1b83f69](https://github.com/common-repo/common-repo/commit/1b83f69c01670aa53e61d74417e6aaeeb8fa01a6))
* **tests:** add Markdown merge operation tests ([5738bb7](https://github.com/common-repo/common-repo/commit/5738bb768ee98a65b8b7f4940afbe12b3933c3b9))
* **tests:** add merge configuration parsing tests ([7fb82c9](https://github.com/common-repo/common-repo/commit/7fb82c951b4f9a583fcbf1a029a308c69493a877))
* **tests:** add TOML merge operation tests ([45479d1](https://github.com/common-repo/common-repo/commit/45479d11559e87d4c97f4ad80418cdca5abae94a))


### Bug Fixes

* **test:** update integration test for archived file ([705f2b4](https://github.com/common-repo/common-repo/commit/705f2b4af4408e06256dfa6c55c0f5cb192f00ad))

## [0.17.0](https://github.com/common-repo/common-repo/compare/v0.16.0...v0.17.0) (2025-12-01)


### Features

* **tests:** add JSON merge operation tests ([4a6217d](https://github.com/common-repo/common-repo/commit/4a6217d0a7706d6d4f92407d66a195af5118c9f2))
* **tests:** add Phase 1 discovery error handling tests ([3ac4727](https://github.com/common-repo/common-repo/commit/3ac4727727bcad4127b82bb145ff271ef05054ce))
* **tests:** add Phase 2 processing tests ([0743782](https://github.com/common-repo/common-repo/commit/074378233560bc9e847666d794d2a77d0b414916))
* **tests:** add YAML merge operation tests ([a2784fd](https://github.com/common-repo/common-repo/commit/a2784fd9a32ff3f5a03c6eb93d5a2a0f7c0f0d6d))

## [0.16.0](https://github.com/common-repo/common-repo/compare/v0.15.0...v0.16.0) (2025-12-01)


### Features

* **tests:** add INI merge integration tests ([eca2239](https://github.com/common-repo/common-repo/commit/eca2239e41a01d1455472e5399418f541bf752f3))
* **tests:** add integration test infrastructure for merge operators ([aa77aae](https://github.com/common-repo/common-repo/commit/aa77aae7640dda574b98020adcdacf1eb24fcd49))
* **tests:** add JSON merge integration tests ([1d65afc](https://github.com/common-repo/common-repo/commit/1d65afca9b53955a50fb1023a77c4b2e43991b1a))
* **tests:** add Markdown merge integration tests ([319b7af](https://github.com/common-repo/common-repo/commit/319b7af90bda8c35e97a52586dbcd68005c87dd4))
* **tests:** add TOML merge integration tests ([7d94c5f](https://github.com/common-repo/common-repo/commit/7d94c5fb5d55405825e39d3e8dcf0ae2bd879742))
* **tests:** add YAML merge integration tests ([aba5e52](https://github.com/common-repo/common-repo/commit/aba5e527b835b7c4f0d294091c662ea7c0a45e17))

## [0.15.0](https://github.com/common-repo/common-repo/compare/v0.14.0...v0.15.0) (2025-11-30)


### Features

* **context:** add task stash stack system ([a2e8f43](https://github.com/common-repo/common-repo/commit/a2e8f439cefc523df0ef39f7abc710fd52a281af))
* **scripts:** add QUICK and SKIP_UPDATE env vars for faster test runs ([13b89b9](https://github.com/common-repo/common-repo/commit/13b89b9c2e6f2dd26dd4c5cd2e946a0335c8eebe))
* **scripts:** use install-cargo-binstall in bootstrap ([40e898a](https://github.com/common-repo/common-repo/commit/40e898a6fa4d4be73cf840825d6d4d3d4ab49ea1))
* **scripts:** use prek binary installer for faster bootstrap ([720d2fa](https://github.com/common-repo/common-repo/commit/720d2fafffeec859db26baf2036a92e03f12f173))

## [0.14.0](https://github.com/common-repo/common-repo/compare/v0.13.0...v0.14.0) (2025-11-30)


### Features

* **scripts:** add install-cargo-binstall script ([8b1a279](https://github.com/common-repo/common-repo/commit/8b1a2790364ca6e4917e1b1eaaf44436d95b990d))

## [0.13.0](https://github.com/common-repo/common-repo/compare/v0.12.0...v0.13.0) (2025-11-30)


### Features

* **cli:** add diff command to preview changes before applying ([df339d5](https://github.com/common-repo/common-repo/commit/df339d5288bc82af6a12c2fec58d62a67feeb68e))
* **cli:** implement ls command to list files from configuration ([a998cd7](https://github.com/common-repo/common-repo/commit/a998cd72e20fd4d001d1f546acca2bfb5c58b2de))


### Bug Fixes

* **tests:** mark tree test as integration test requiring network ([bf07183](https://github.com/common-repo/common-repo/commit/bf07183411eabf446085f98c72a59e0de5b0a503))
* **tests:** resolve flaky test issues with serial execution ([b2bbe4f](https://github.com/common-repo/common-repo/commit/b2bbe4f69259a53a7dbeb2b049ca43e5b7654677))
* **tests:** use cargo_bin_cmd macro instead of deprecated Command::cargo_bin ([3c723ec](https://github.com/common-repo/common-repo/commit/3c723ecc5cd40b37ddbd303e282076d136d75a00))

## [0.12.0](https://github.com/common-repo/common-repo/compare/v0.11.0...v0.12.0) (2025-11-21)


### Features

* **cli:** implement tree command with hierarchical display ([67f6e68](https://github.com/common-repo/common-repo/commit/67f6e68db9032eaac2ae613c9c1a46134e8a4c02))

## [0.11.0](https://github.com/common-repo/common-repo/compare/v0.10.0...v0.11.0) (2025-11-21)


### Features

* **cli:** implement info command with full tests ([d988629](https://github.com/common-repo/common-repo/commit/d988629bf4c96e7fe4517c12383fee386b96c9e7))


### Bug Fixes

* **tests:** use semver comparison in update test ([fd127ba](https://github.com/common-repo/common-repo/commit/fd127ba7551a03791bed8788bc01e42cb07a83bb))

## [0.10.0](https://github.com/common-repo/common-repo/compare/v0.9.0...v0.10.0) (2025-11-21)


### Features

* **cache:** implement cache list and clean commands ([881bf25](https://github.com/common-repo/common-repo/commit/881bf253b801005662eccb178cfaa944572d8bab))


### Bug Fixes

* **tests:** update test expectations for v0.9.0 and context/ directory ([1fad196](https://github.com/common-repo/common-repo/commit/1fad1960489afa75647605fc4d5511f4f8555566))
* **tests:** use runtime version comparison in update test ([4abd79f](https://github.com/common-repo/common-repo/commit/4abd79f7cb8277d9a322c4ab113e37f808cf6883))

## [0.9.0](https://github.com/common-repo/common-repo/compare/v0.8.0...v0.9.0) (2025-11-21)


### Features

* **cli,toml:** add init command and fix TOML path escape handling ([f3be762](https://github.com/common-repo/common-repo/commit/f3be7620949a9308e383d1cb6b5aeb9c3363201b))
* **cli,toml:** add logging infrastructure and enhance TOML path handling ([98e0bc6](https://github.com/common-repo/common-repo/commit/98e0bc6ccf628faaad2025ace0f5e6c68491bbcd))
* **cursor:** spreading rules ([6b6135d](https://github.com/common-repo/common-repo/commit/6b6135dd74ec1b21ec7e0d40eaff74408c86b19f))
* **gemini:** more context ([c425929](https://github.com/common-repo/common-repo/commit/c4259290bdea8a5eb047aabeff0d72963b71568b))
* **ini:** make section optional and enhance merge capabilities ([b5d2b84](https://github.com/common-repo/common-repo/commit/b5d2b8442bd167beaf1f60dfdef82f3848a5ee82))


### Bug Fixes

* **merge:** prevent duplicate merge operations and update test expectations ([26f3ad2](https://github.com/common-repo/common-repo/commit/26f3ad2b2747b6660566cdfa01dff3691692bf72))

## [0.8.0](https://github.com/common-repo/common-repo/compare/v0.7.1...v0.8.0) (2025-11-21)


### Features

* **cli:** stub out long description ([ceabdcb](https://github.com/common-repo/common-repo/commit/ceabdcbb3bbf53e800ca8f4b87cb2b13db2ae2d6))
* **validate:** add validate command with full tests ([205b4e5](https://github.com/common-repo/common-repo/commit/205b4e5b8bf54097484f3d4e986c89e70f0c62b2))

## [0.7.1](https://github.com/common-repo/common-repo/compare/v0.7.0...v0.7.1) (2025-11-16)


### Bug Fixes

* add doc comment to yaml_merge_integration tests ([b995ed4](https://github.com/common-repo/common-repo/commit/b995ed4f810d20b779f0f64b9d10e4645f9cb0eb))
* enable YAML merge operations in Phase 2 processing ([cd627f0](https://github.com/common-repo/common-repo/commit/cd627f0d2eeab78b99eee418bf7770acb58801f3))
* execute local YAML merge operations in Phase 5 after loading local files ([dd9ffd4](https://github.com/common-repo/common-repo/commit/dd9ffd418505bc18fbca57ca0f3c1badb403b7c1))
* remove unused predicates import from CLI e2e tests ([3c4bea8](https://github.com/common-repo/common-repo/commit/3c4bea871f9af7f5baa95710815eaa4b29855e81))

## [0.7.0](https://github.com/common-repo/common-repo/compare/v0.6.0...v0.7.0) (2025-11-16)


### Features

* implement Phase 2.4 YAML/TOML merge enhancements ([4afb087](https://github.com/common-repo/common-repo/commit/4afb087aaf67665a8ae87c195209b78b1f6f2208))

## [0.6.0](https://github.com/common-repo/common-repo/compare/v0.5.0...v0.6.0) (2025-11-16)


### Features

* add optional path field and escaping support for YAML merge operator ([7915b95](https://github.com/common-repo/common-repo/commit/7915b95fe379eacf4d2dcc098822a5c3f7f80f6c))
* **dev:** add prek as recommended pre-commit tool ([982c091](https://github.com/common-repo/common-repo/commit/982c091b742bb4dca48006c3117b1c2fe0b76105))
* enable merge operators in Phase 4 composition ([896d31b](https://github.com/common-repo/common-repo/commit/896d31bcd23171386964e317b21014313bd900b2))


### Bug Fixes

* correct integration test API usage for MemoryFS and serde_yaml ([9908eb3](https://github.com/common-repo/common-repo/commit/9908eb32c6a3132aba4c65e676810afc985ad568))
* correct test assertions and remove clippy warnings ([609b6c9](https://github.com/common-repo/common-repo/commit/609b6c950a484d7d3042bce815390f3a33f16891))
* correct test module imports for visibility ([f6b4678](https://github.com/common-repo/common-repo/commit/f6b4678afee0205ca69c8d660846248946506d8e))
* correct test module imports to use crate::phases::phase5 path ([42be728](https://github.com/common-repo/common-repo/commit/42be728f6183902a18d29b5d6f73483c87bc706c))
* **dev:** install prek from git to get latest version ([8883295](https://github.com/common-repo/common-repo/commit/88832952c365a52210f83b586cbbc85f84fcfcf7))
* make test helper functions pub(crate) for test visibility ([a3605fa](https://github.com/common-repo/common-repo/commit/a3605faca118d0f60482046313981ead268053c5))
* use super::super:: for nested test module imports ([0ef3ee1](https://github.com/common-repo/common-repo/commit/0ef3ee1367026a15eabdb46faa6a3075ade23b83))

## [0.5.0](https://github.com/common-repo/common-repo/compare/v0.4.0...v0.5.0) (2025-11-16)


### Features

* **operators:** complete with: clause support for include, template, and tools ([debdb8c](https://github.com/common-repo/common-repo/commit/debdb8c142d01ae4a3b7bb51859f12ff495be6f2))

## [0.4.0](https://github.com/common-repo/common-repo/compare/v0.3.0...v0.4.0) (2025-11-16)


### ⚠ BREAKING CHANGES

* **ci:** Release tags will now use vX.Y.Z format instead of common-repo-vX.Y.Z. This aligns with standard semantic versioning conventions and improves compatibility with version detection tools.

### Features

* add backward-compatible yaml parsing for original schema format ([def57d3](https://github.com/common-repo/common-repo/commit/def57d3ee31794f7f079fe0a9348935ac6e06956))
* add CLI implementation with apply command ([49556ac](https://github.com/common-repo/common-repo/commit/49556ac6276018250e010ef4f30caa665ed2d8e8))
* add CommonRepo v2 schema with operator-based design ([dc70afe](https://github.com/common-repo/common-repo/commit/dc70afe213a7d256c60b51dd4919258d037cd52f))
* add Lint job to CI workflow ([c0f137f](https://github.com/common-repo/common-repo/commit/c0f137f93249740b35f1033d9f50c40be9a728b6))
* add sub-path support to repository operations ([d7ff119](https://github.com/common-repo/common-repo/commit/d7ff1195d1e5acc160587f09599dcfcafc7ef573))
* added sponsorship button ([14f0776](https://github.com/common-repo/common-repo/commit/14f0776fcd7ed4c089b5c04e4e31019b61bdda2e))
* **ci:** use vX.Y.Z tag format for releases ([e4800ff](https://github.com/common-repo/common-repo/commit/e4800ff093fbb7c78e31e951583acc1c0af8e89f))
* complete inheritance pipeline with Phases 1-5 ([63a6782](https://github.com/common-repo/common-repo/commit/63a6782a3cca090e82b5bab1b7c6146e439393e4))
* complete layer 0 foundation components ([8bf3b54](https://github.com/common-repo/common-repo/commit/8bf3b54e05db2b0732097c63ca8092ddca28331e))
* complete merge operators and phase pipeline ([9106fa5](https://github.com/common-repo/common-repo/commit/9106fa59e6c51b6cfa728f04aaca9eb3781b1923))
* complete mvp end-to-end cli pipeline implementation ([a5ea6bc](https://github.com/common-repo/common-repo/commit/a5ea6bc51c810bec11fbd541d42b48db94bdd0f9))
* enhance phase 1 with recursive repository discovery ([a964d3c](https://github.com/common-repo/common-repo/commit/a964d3cac9166265ae9228b7503d77b048024a1c))
* implement layer 2.1 repo operator with full testing ([50f1183](https://github.com/common-repo/common-repo/commit/50f11837e641cdbcb331be331575463806ac8629))
* implement phase 6 writing to disk and apply code review fixes ([d54b708](https://github.com/common-repo/common-repo/commit/d54b7085c996593c3680cc9c0d95be377b659571))
* implement repository manager with trait-based design ([50fc6a5](https://github.com/common-repo/common-repo/commit/50fc6a58eaa9cb2cba7228ef2939b1fc2915c834))
* implement template and YAML/JSON merge operators ([f935c47](https://github.com/common-repo/common-repo/commit/f935c47722d67254b2368c2683a0811ee9a7482c))
* implement tool validation and version detection ([c52f64b](https://github.com/common-repo/common-repo/commit/c52f64bff3e43ca8ef84f9a3258a09be4ca8ad2d))
* initialize Rust project with modern tooling ([ac3b9be](https://github.com/common-repo/common-repo/commit/ac3b9be9f0bf279c2a85ab851a6023411617d374))
* remove placeholder binary, clarify library-only usage ([15d1329](https://github.com/common-repo/common-repo/commit/15d132978f29a3a9c7e43d220cfcd80b51609034))
* **repo-sub-path:** implement repository sub-path filtering ([d714322](https://github.com/common-repo/common-repo/commit/d7143220ff36558e48d61212a6b73c27bc56e0dd))
* starting work on schema definitions ([47d79e8](https://github.com/common-repo/common-repo/commit/47d79e840f2eb925b7da095bfa718bb16a777234))
* use nextest ([378ec8f](https://github.com/common-repo/common-repo/commit/378ec8f13567713199916cc5e0f7d9824ebf6464))


### Bug Fixes

* address implementation inconsistencies and missing error types ([c1dd7f8](https://github.com/common-repo/common-repo/commit/c1dd7f865780d4d461cf665122ec38110a652ccb))
* correct Rust edition and update test counts in documentation ([d69953c](https://github.com/common-repo/common-repo/commit/d69953c756e57b906ca8fdc84e4a38c42d181996))
* correct variable names in execute_pull call ([72f4be1](https://github.com/common-repo/common-repo/commit/72f4be1c0dd3ea99b670ddd7a706c98f85a54b5a))
* improve Phase 1 discovery and fix code review issues ([74ff794](https://github.com/common-repo/common-repo/commit/74ff79455e8eec04f88578e6f0ae4b2006b9873c))
* **test:** make test_update_dry_run more reliable and add ignored fixture test ([015150a](https://github.com/common-repo/common-repo/commit/015150a0b7944060df9862b591bcb97e08bb359c))

## [0.2.1](https://github.com/common-repo/common-repo/compare/common-repo-v0.2.0...common-repo-v0.2.1) (2025-11-16)


### Bug Fixes

* **test:** make test_update_dry_run more reliable and add ignored fixture test ([015150a](https://github.com/common-repo/common-repo/commit/015150a0b7944060df9862b591bcb97e08bb359c))

## [0.2.0](https://github.com/common-repo/common-repo/compare/common-repo-v0.1.0...common-repo-v0.2.0) (2025-11-16)


### Features

* add backward-compatible yaml parsing for original schema format ([def57d3](https://github.com/common-repo/common-repo/commit/def57d3ee31794f7f079fe0a9348935ac6e06956))
* add CLI implementation with apply command ([49556ac](https://github.com/common-repo/common-repo/commit/49556ac6276018250e010ef4f30caa665ed2d8e8))
* add CommonRepo v2 schema with operator-based design ([dc70afe](https://github.com/common-repo/common-repo/commit/dc70afe213a7d256c60b51dd4919258d037cd52f))
* add Lint job to CI workflow ([c0f137f](https://github.com/common-repo/common-repo/commit/c0f137f93249740b35f1033d9f50c40be9a728b6))
* add sub-path support to repository operations ([d7ff119](https://github.com/common-repo/common-repo/commit/d7ff1195d1e5acc160587f09599dcfcafc7ef573))
* added sponsorship button ([14f0776](https://github.com/common-repo/common-repo/commit/14f0776fcd7ed4c089b5c04e4e31019b61bdda2e))
* complete inheritance pipeline with Phases 1-5 ([63a6782](https://github.com/common-repo/common-repo/commit/63a6782a3cca090e82b5bab1b7c6146e439393e4))
* complete layer 0 foundation components ([8bf3b54](https://github.com/common-repo/common-repo/commit/8bf3b54e05db2b0732097c63ca8092ddca28331e))
* complete merge operators and phase pipeline ([9106fa5](https://github.com/common-repo/common-repo/commit/9106fa59e6c51b6cfa728f04aaca9eb3781b1923))
* complete mvp end-to-end cli pipeline implementation ([a5ea6bc](https://github.com/common-repo/common-repo/commit/a5ea6bc51c810bec11fbd541d42b48db94bdd0f9))
* enhance phase 1 with recursive repository discovery ([a964d3c](https://github.com/common-repo/common-repo/commit/a964d3cac9166265ae9228b7503d77b048024a1c))
* implement layer 2.1 repo operator with full testing ([50f1183](https://github.com/common-repo/common-repo/commit/50f11837e641cdbcb331be331575463806ac8629))
* implement phase 6 writing to disk and apply code review fixes ([d54b708](https://github.com/common-repo/common-repo/commit/d54b7085c996593c3680cc9c0d95be377b659571))
* implement repository manager with trait-based design ([50fc6a5](https://github.com/common-repo/common-repo/commit/50fc6a58eaa9cb2cba7228ef2939b1fc2915c834))
* implement template and YAML/JSON merge operators ([f935c47](https://github.com/common-repo/common-repo/commit/f935c47722d67254b2368c2683a0811ee9a7482c))
* implement tool validation and version detection ([c52f64b](https://github.com/common-repo/common-repo/commit/c52f64bff3e43ca8ef84f9a3258a09be4ca8ad2d))
* initialize Rust project with modern tooling ([ac3b9be](https://github.com/common-repo/common-repo/commit/ac3b9be9f0bf279c2a85ab851a6023411617d374))
* remove placeholder binary, clarify library-only usage ([15d1329](https://github.com/common-repo/common-repo/commit/15d132978f29a3a9c7e43d220cfcd80b51609034))
* **repo-sub-path:** implement repository sub-path filtering ([d714322](https://github.com/common-repo/common-repo/commit/d7143220ff36558e48d61212a6b73c27bc56e0dd))
* starting work on schema definitions ([47d79e8](https://github.com/common-repo/common-repo/commit/47d79e840f2eb925b7da095bfa718bb16a777234))
* use nextest ([378ec8f](https://github.com/common-repo/common-repo/commit/378ec8f13567713199916cc5e0f7d9824ebf6464))


### Bug Fixes

* address implementation inconsistencies and missing error types ([c1dd7f8](https://github.com/common-repo/common-repo/commit/c1dd7f865780d4d461cf665122ec38110a652ccb))
* correct Rust edition and update test counts in documentation ([d69953c](https://github.com/common-repo/common-repo/commit/d69953c756e57b906ca8fdc84e4a38c42d181996))
* correct variable names in execute_pull call ([72f4be1](https://github.com/common-repo/common-repo/commit/72f4be1c0dd3ea99b670ddd7a706c98f85a54b5a))
* improve Phase 1 discovery and fix code review issues ([74ff794](https://github.com/common-repo/common-repo/commit/74ff79455e8eec04f88578e6f0ae4b2006b9873c))
