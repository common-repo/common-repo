# Changelog

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
* implement layer 2.1 repo operator with comprehensive testing ([50f1183](https://github.com/common-repo/common-repo/commit/50f11837e641cdbcb331be331575463806ac8629))
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
* **test:** make test_update_dry_run more robust and add ignored fixture test ([015150a](https://github.com/common-repo/common-repo/commit/015150a0b7944060df9862b591bcb97e08bb359c))

## [0.2.1](https://github.com/common-repo/common-repo/compare/common-repo-v0.2.0...common-repo-v0.2.1) (2025-11-16)


### Bug Fixes

* **test:** make test_update_dry_run more robust and add ignored fixture test ([015150a](https://github.com/common-repo/common-repo/commit/015150a0b7944060df9862b591bcb97e08bb359c))

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
* implement layer 2.1 repo operator with comprehensive testing ([50f1183](https://github.com/common-repo/common-repo/commit/50f11837e641cdbcb331be331575463806ac8629))
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
