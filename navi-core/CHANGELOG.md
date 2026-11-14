# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-14

### Bug Fixes
- run clippy by @elcoosp ([caed1eb](https://github.com/elcoosp/navi/commit/caed1ebb7d94de15ef9edb7eec1bcf5c79d4a0b5))
- use edition = 2024 by @elcoosp ([b69aa48](https://github.com/elcoosp/navi/commit/b69aa48163b83b94a1f1141ada036cdc7ca3be2f))

### Documentation
- add Vercel documentation link to Cargo.toml by @elcoosp ([725838f](https://github.com/elcoosp/navi/commit/725838fe987aff7380e56051b17bba8c89f7882b))
- enforce SKILL.md rules (jsDelivr logos, absolute links, remove license sections) by @elcoosp ([3c6e69e](https://github.com/elcoosp/navi/commit/3c6e69ecec649d85954a2d527b44e42a569458e9))
- rewrite for framework-agnostic quoin architecture by @elcoosp ([2c1f239](https://github.com/elcoosp/navi/commit/2c1f2394f5d546eca166fc34b48f96a6e0455902))
- rewrite performance section highlighting allocation-free loops by @elcoosp ([eb6ed84](https://github.com/elcoosp/navi/commit/eb6ed84075c902ecbb2ef74289fc39d0e6a02e10))
- update dependency stack documentation for gpui-kit migration by @elcoosp ([9d61a86](https://github.com/elcoosp/navi/commit/9d61a86ed18daac9dfbdb2ab9f15709caabdc5cb))
- update README with new features, development workflow and quality gates by @elcoosp ([5530ac7](https://github.com/elcoosp/navi/commit/5530ac7e57450fe1736c2e0c47b4721f30b71224))

### Features
- introduce WindowKey opaque window identifier replacement by @elcoosp ([200ee8e](https://github.com/elcoosp/navi/commit/200ee8ee9681f82d275c39d12a86b3e486fabd65))
- nexum integration poc by @elcoosp ([f3a5b61](https://github.com/elcoosp/navi/commit/f3a5b61ad29381cd4e1ab0d9830eff1e8c079694))
- add logo to readme by @elcoosp ([75db2b5](https://github.com/elcoosp/navi/commit/75db2b549e55f0c44ec2f181d9db7eb0adf52ec3))
- icons working but not text by @elcoosp ([287184d](https://github.com/elcoosp/navi/commit/287184ded63c6d1d108e9c145fa5e185689780e3))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))
- init by @elcoosp ([e7488fb](https://github.com/elcoosp/navi/commit/e7488fb8d9bafba7bbfce6e5807cda5152ae3417))

### Miscellaneous
- inject missing metadata and point to root README by @elcoosp ([4fe4624](https://github.com/elcoosp/navi/commit/4fe4624455fc2b6f2476f3173374c27df1541314))
- inject precisely curated keywords for crates.io by @elcoosp ([ea31ad8](https://github.com/elcoosp/navi/commit/ea31ad8756433fd1b01ba3c2c2b5a099187828da))
- drop explicit gpui dependency requirement from core crate by @elcoosp ([9b038ec](https://github.com/elcoosp/navi/commit/9b038ecb46bac1e0b747ef168bae1a52857db717))
- logo by @elcoosp ([4bfc15e](https://github.com/elcoosp/navi/commit/4bfc15ebbeaf218119b0257ccab8a751e0e936a5))
- readme by @elcoosp ([48a8988](https://github.com/elcoosp/navi/commit/48a89889c2cc23ec6188bdf8b8ea0fa090e2b709))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([2e5ce01](https://github.com/elcoosp/navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- up readme by @elcoosp ([2549ff1](https://github.com/elcoosp/navi/commit/2549ff1c625f915fd12df0fbe44d75e092348b4c))

### Refactor
- rename gpui-navi → navi across repo and docs by @elcoosp ([e7938aa](https://github.com/elcoosp/navi/commit/e7938aa5d34ff270e199ec65c4bc514f1c66b8f6))

### Testing
- add unit tests for SuspenseState and SuspenseConfig by @elcoosp ([d5fa429](https://github.com/elcoosp/navi/commit/d5fa42981badfb4d31d6dd6eb0a137d039b52084))
- add unit and gpui integration tests for context system by @elcoosp ([50357a5](https://github.com/elcoosp/navi/commit/50357a5e9eef2448829bea83775078738402042a))
- fix optional segment skipped test assertions by @elcoosp ([f8ce755](https://github.com/elcoosp/navi/commit/f8ce7555df911fd9459d1cd95a5cbbf0daf7311a))

### build
- remove unused deps and inherit workspace lints in navi-core by @elcoosp ([72e8448](https://github.com/elcoosp/navi/commit/72e84483e999d6c5d038617c8358496a3126b5c7))
**Full Changelog**: https://github.com/elcoosp/navi/releases/tag/v0.2.0
