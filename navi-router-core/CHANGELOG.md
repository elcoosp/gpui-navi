# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-11

### Features
- add garde integration, DRY validation helpers and per-feature tests by @elcoosp ([e7f5565](https://github.com/elcoosp/gpui-navi/commit/e7f5565fa91e38c724fc0410bca1140d3c5fe5d2))
- add AnyData::new/downcast_ref/get_cloned helpers and tests by @elcoosp ([8552c5d](https://github.com/elcoosp/gpui-navi/commit/8552c5d65913fa537915fad28644ffa4024522fd))
- rewrite matching trie for determinism, add Arc nodes and RouteTree::validate by @elcoosp ([ccd31bc](https://github.com/elcoosp/gpui-navi/commit/ccd31bc1693f9b243a407b3d19b618ad40a6f0f0))
- add percent-decoding, query arrays, nested-search and comprehensive tests by @elcoosp ([0b4f1fb](https://github.com/elcoosp/gpui-navi/commit/0b4f1fbcd78a43e5c9d14db860883324a5cda2b8))
- add len/is_empty, fix notify order and add comprehensive tests by @elcoosp ([bb95817](https://github.com/elcoosp/gpui-navi/commit/bb9581753d56f33850e23b29e6e50da6447e8901))
- add NavOp transactions, single-match navigation and traversal re-matching by @elcoosp ([fbc8bb6](https://github.com/elcoosp/gpui-navi/commit/fbc8bb6132188166782740b37344159d85340474))
- readd validator by @elcoosp in [#3](https://github.com/elcoosp/gpui-navi/pull/3) ([12576c1](https://github.com/elcoosp/gpui-navi/commit/12576c15dfc29acf0d337ffbceacc8daf27dcefb))

### Miscellaneous
- wire criterion harness configurations into the manifest by @elcoosp ([42f9fca](https://github.com/elcoosp/gpui-navi/commit/42f9fca4c57e115d12547a890534deb17a15f545))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/gpui-navi/pull/3) ([2e5ce01](https://github.com/elcoosp/gpui-navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/gpui-navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- final fixes, docs, and cleanup for decoupling release by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([d34fa5c](https://github.com/elcoosp/gpui-navi/commit/d34fa5c45ae26f1c51dab77fba5a61bff9092948))

### Performance
- craft zero-copy radix trie matching with stack bindings by @elcoosp ([439af85](https://github.com/elcoosp/gpui-navi/commit/439af8505dce31d4a5c987e167c8f1335f344af0))
- optimize scalar parsing queries coercion rules by @elcoosp ([0524e6d](https://github.com/elcoosp/gpui-navi/commit/0524e6dd749aefe0f89e55d2dc0759396dfadc64))
- author route trie matching and queries coercion benchmarks by @elcoosp ([69bd50d](https://github.com/elcoosp/gpui-navi/commit/69bd50de4828607d5407188d308cf95d099d9959))

### Refactor
- purge redundant mutex lock inside browser history stack by @elcoosp ([6e8e586](https://github.com/elcoosp/gpui-navi/commit/6e8e5864764118c9a8da493831f1c6d6d155561c))
- accept shared route tree reference inside initializers by @elcoosp ([7e13548](https://github.com/elcoosp/gpui-navi/commit/7e135487e193b48f61eea0a18d8b656aabe9960e))
- update re-exports for NavOp, MatchResult, RouteTreeError by @elcoosp ([23806a2](https://github.com/elcoosp/gpui-navi/commit/23806a25ff9e7b51b805fea2c16db5c0a722f1be))
- extract ShouldBlockFn type alias and add must_use annotations by @elcoosp ([a544962](https://github.com/elcoosp/gpui-navi/commit/a54496245229d9e3493c6d47ac0acf3e929cd0c9))
- update codegen for core/adapter split by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([aafec04](https://github.com/elcoosp/gpui-navi/commit/aafec041ac9d91689d54439a046b24e805c5bfae))

### Testing
- add Debug derive and unit tests for Redirect and NotFound by @elcoosp ([f692575](https://github.com/elcoosp/gpui-navi/commit/f692575adddf8eecce901070e6e57e425b9f6890))
- add unit tests for History, Blocker, Validation; remove broken integration test by @elcoosp in [#3](https://github.com/elcoosp/gpui-navi/pull/3) ([7843bc1](https://github.com/elcoosp/gpui-navi/commit/7843bc1f3231eb7ab66fb75d1ce2b125fd64cca9))

### build
- update deps, add nested-search feature, inherit workspace lints by @elcoosp ([19bfb0a](https://github.com/elcoosp/gpui-navi/commit/19bfb0af05847e8e7cff18b2f6bdbb118068cd6a))
**Full Changelog**: https://github.com/elcoosp/gpui-navi/releases/tag/v0.2.0
