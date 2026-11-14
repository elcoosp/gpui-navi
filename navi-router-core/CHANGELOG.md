# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-14

### Bug Fixes
- restore NodeElement wrappers and suppress dead_code warnings by @elcoosp ([11e819b](https://github.com/elcoosp/navi/commit/11e819b9a34aea1e005ac931e88e0dd085753e05))

### Documentation
- add Vercel documentation link to Cargo.toml by @elcoosp ([725838f](https://github.com/elcoosp/navi/commit/725838fe987aff7380e56051b17bba8c89f7882b))
- enforce SKILL.md rules (jsDelivr logos, absolute links, remove license sections) by @elcoosp ([3c6e69e](https://github.com/elcoosp/navi/commit/3c6e69ecec649d85954a2d527b44e42a569458e9))
- rewrite for framework-agnostic quoin architecture by @elcoosp ([2c1f239](https://github.com/elcoosp/navi/commit/2c1f2394f5d546eca166fc34b48f96a6e0455902))
- rewrite performance section highlighting allocation-free loops by @elcoosp ([eb6ed84](https://github.com/elcoosp/navi/commit/eb6ed84075c902ecbb2ef74289fc39d0e6a02e10))
- update dependency stack documentation for gpui-kit migration by @elcoosp ([9d61a86](https://github.com/elcoosp/navi/commit/9d61a86ed18daac9dfbdb2ab9f15709caabdc5cb))
- update README with new features, development workflow and quality gates by @elcoosp ([5530ac7](https://github.com/elcoosp/navi/commit/5530ac7e57450fe1736c2e0c47b4721f30b71224))

### Features
- add garde integration, DRY validation helpers and per-feature tests by @elcoosp ([e7f5565](https://github.com/elcoosp/navi/commit/e7f5565fa91e38c724fc0410bca1140d3c5fe5d2))
- add AnyData::new/downcast_ref/get_cloned helpers and tests by @elcoosp ([8552c5d](https://github.com/elcoosp/navi/commit/8552c5d65913fa537915fad28644ffa4024522fd))
- rewrite matching trie for determinism, add Arc nodes and RouteTree::validate by @elcoosp ([ccd31bc](https://github.com/elcoosp/navi/commit/ccd31bc1693f9b243a407b3d19b618ad40a6f0f0))
- add percent-decoding, query arrays, nested-search and comprehensive tests by @elcoosp ([0b4f1fb](https://github.com/elcoosp/navi/commit/0b4f1fbcd78a43e5c9d14db860883324a5cda2b8))
- add len/is_empty, fix notify order and add comprehensive tests by @elcoosp ([bb95817](https://github.com/elcoosp/navi/commit/bb9581753d56f33850e23b29e6e50da6447e8901))
- add NavOp transactions, single-match navigation and traversal re-matching by @elcoosp ([fbc8bb6](https://github.com/elcoosp/navi/commit/fbc8bb6132188166782740b37344159d85340474))
- readd validator by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([12576c1](https://github.com/elcoosp/navi/commit/12576c15dfc29acf0d337ffbceacc8daf27dcefb))
- nexum integration poc by @elcoosp ([f3a5b61](https://github.com/elcoosp/navi/commit/f3a5b61ad29381cd4e1ab0d9830eff1e8c079694))
- add logo to readme by @elcoosp ([75db2b5](https://github.com/elcoosp/navi/commit/75db2b549e55f0c44ec2f181d9db7eb0adf52ec3))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))

### Miscellaneous
- inject missing metadata and point to root README by @elcoosp ([4fe4624](https://github.com/elcoosp/navi/commit/4fe4624455fc2b6f2476f3173374c27df1541314))
- inject precisely curated keywords for crates.io by @elcoosp ([ea31ad8](https://github.com/elcoosp/navi/commit/ea31ad8756433fd1b01ba3c2c2b5a099187828da))
- wire criterion harness configurations into the manifest by @elcoosp ([42f9fca](https://github.com/elcoosp/navi/commit/42f9fca4c57e115d12547a890534deb17a15f545))
- logo by @elcoosp ([4bfc15e](https://github.com/elcoosp/navi/commit/4bfc15ebbeaf218119b0257ccab8a751e0e936a5))
- readme by @elcoosp ([48a8988](https://github.com/elcoosp/navi/commit/48a89889c2cc23ec6188bdf8b8ea0fa090e2b709))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([2e5ce01](https://github.com/elcoosp/navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- final fixes, docs, and cleanup for decoupling release by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([d34fa5c](https://github.com/elcoosp/navi/commit/d34fa5c45ae26f1c51dab77fba5a61bff9092948))
- up readme by @elcoosp ([2549ff1](https://github.com/elcoosp/navi/commit/2549ff1c625f915fd12df0fbe44d75e092348b4c))

### Performance
- craft zero-copy radix trie matching with stack bindings by @elcoosp ([439af85](https://github.com/elcoosp/navi/commit/439af8505dce31d4a5c987e167c8f1335f344af0))
- optimize scalar parsing queries coercion rules by @elcoosp ([0524e6d](https://github.com/elcoosp/navi/commit/0524e6dd749aefe0f89e55d2dc0759396dfadc64))
- author route trie matching and queries coercion benchmarks by @elcoosp ([69bd50d](https://github.com/elcoosp/navi/commit/69bd50de4828607d5407188d308cf95d099d9959))

### Refactor
- rename gpui-navi → navi across repo and docs by @elcoosp ([e7938aa](https://github.com/elcoosp/navi/commit/e7938aa5d34ff270e199ec65c4bc514f1c66b8f6))
- purge redundant mutex lock inside browser history stack by @elcoosp ([6e8e586](https://github.com/elcoosp/navi/commit/6e8e5864764118c9a8da493831f1c6d6d155561c))
- accept shared route tree reference inside initializers by @elcoosp ([7e13548](https://github.com/elcoosp/navi/commit/7e135487e193b48f61eea0a18d8b656aabe9960e))
- update re-exports for NavOp, MatchResult, RouteTreeError by @elcoosp ([23806a2](https://github.com/elcoosp/navi/commit/23806a25ff9e7b51b805fea2c16db5c0a722f1be))
- extract ShouldBlockFn type alias and add must_use annotations by @elcoosp ([a544962](https://github.com/elcoosp/navi/commit/a54496245229d9e3493c6d47ac0acf3e929cd0c9))
- update codegen for core/adapter split by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([aafec04](https://github.com/elcoosp/navi/commit/aafec041ac9d91689d54439a046b24e805c5bfae))

### Testing
- add Debug derive and unit tests for Redirect and NotFound by @elcoosp ([f692575](https://github.com/elcoosp/navi/commit/f692575adddf8eecce901070e6e57e425b9f6890))
- add unit tests for History, Blocker, Validation; remove broken integration test by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([7843bc1](https://github.com/elcoosp/navi/commit/7843bc1f3231eb7ab66fb75d1ce2b125fd64cca9))
- fix optional segment skipped test assertions by @elcoosp ([f8ce755](https://github.com/elcoosp/navi/commit/f8ce7555df911fd9459d1cd95a5cbbf0daf7311a))

### build
- update deps, add nested-search feature, inherit workspace lints by @elcoosp ([19bfb0a](https://github.com/elcoosp/navi/commit/19bfb0af05847e8e7cff18b2f6bdbb118068cd6a))
**Full Changelog**: https://github.com/elcoosp/navi/releases/tag/v0.2.0
