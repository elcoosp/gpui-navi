# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-14

### Bug Fixes
- scanner ancestor chain bug resolved and fixed blocking route freeze by @elcoosp ([31ab98c](https://github.com/elcoosp/navi/commit/31ab98cb7f980b0b6afcc22a08e988eab0ab30ad))
- add non_snake_case allow only for modules with special characters by @elcoosp ([c0cbecd](https://github.com/elcoosp/navi/commit/c0cbecda0f3a961c4fd7fe21f8e86ce1269be506))
- generator now produces snake_case module names and avoids unused imports by @elcoosp ([1382584](https://github.com/elcoosp/navi/commit/13825842f011ecd7a76db1eacd6803f8ecfe53e4))
- simplify 404 registration, clean up route file imports by @elcoosp ([40ac726](https://github.com/elcoosp/navi/commit/40ac726365b8cd35d788a895b1868918ab7c5982))
- remove use navi_router::RouteDef; imports by @elcoosp ([4e629cd](https://github.com/elcoosp/navi/commit/4e629cd273ca1ccc2c5b5f399d7142982a15a516))
- run clippy by @elcoosp ([caed1eb](https://github.com/elcoosp/navi/commit/caed1ebb7d94de15ef9edb7eec1bcf5c79d4a0b5))
- correct get_loader_state using rs-query API, clean up warnings by @elcoosp ([65b1b78](https://github.com/elcoosp/navi/commit/65b1b78885e7cf830a72886b0e7e0fe52d8154ed))
- assign parents purely in-memory by @elcoosp ([2a23940](https://github.com/elcoosp/navi/commit/2a23940550462d773c8398a8964bd8b05cdf4a94))
- separate module ident from url segment for $ files by @elcoosp ([cdb2f3f](https://github.com/elcoosp/navi/commit/cdb2f3fdd9048c0ebd072cc97299a6c73b3b3197))
- resolve borrow checker, import, and macro API issues in Outlet, RouterState, and example by @elcoosp ([c3c7312](https://github.com/elcoosp/navi/commit/c3c7312b2f4c1e305345fb4d1b1b8492fad09193))
- correctly parse dollar filenames, parent hierarchy, and emit compilable route stubs by @elcoosp ([be6b6fa](https://github.com/elcoosp/navi/commit/be6b6facfc02b6479d07669aeeffdc2591add47c))
- use edition = 2024 by @elcoosp ([b69aa48](https://github.com/elcoosp/navi/commit/b69aa48163b83b94a1f1141ada036cdc7ca3be2f))

### Documentation
- add Vercel documentation link to Cargo.toml by @elcoosp ([725838f](https://github.com/elcoosp/navi/commit/725838fe987aff7380e56051b17bba8c89f7882b))
- enforce SKILL.md rules (jsDelivr logos, absolute links, remove license sections) by @elcoosp ([3c6e69e](https://github.com/elcoosp/navi/commit/3c6e69ecec649d85954a2d527b44e42a569458e9))
- rewrite for framework-agnostic quoin architecture by @elcoosp ([2c1f239](https://github.com/elcoosp/navi/commit/2c1f2394f5d546eca166fc34b48f96a6e0455902))
- rewrite performance section highlighting allocation-free loops by @elcoosp ([eb6ed84](https://github.com/elcoosp/navi/commit/eb6ed84075c902ecbb2ef74289fc39d0e6a02e10))
- update dependency stack documentation for gpui-kit migration by @elcoosp ([9d61a86](https://github.com/elcoosp/navi/commit/9d61a86ed18daac9dfbdb2ab9f15709caabdc5cb))
- update README with new features, development workflow and quality gates by @elcoosp ([5530ac7](https://github.com/elcoosp/navi/commit/5530ac7e57450fe1736c2e0c47b4721f30b71224))

### Features
- introduce RouteKind enum, cache regexes and add scanner test suite by @elcoosp ([58edc38](https://github.com/elcoosp/navi/commit/58edc38ba9d76ebb5cfbe5c1b558bb09689dd210))
- make codegen idempotent and add comprehensive tests by @elcoosp ([b633319](https://github.com/elcoosp/navi/commit/b633319afd40c36adb482be65476ecb888a27719))
- add NaviConfig::discover, serde defaults and comprehensive tests by @elcoosp ([6cb0c19](https://github.com/elcoosp/navi/commit/6cb0c1973b63e38791043f2e1e5e9df89394090f))
- chunk 3 parity by @elcoosp ([33e6e49](https://github.com/elcoosp/navi/commit/33e6e49a78f20ffea5c36faf1e6abe1739ba636c))
- nexum integration poc by @elcoosp ([f3a5b61](https://github.com/elcoosp/navi/commit/f3a5b61ad29381cd4e1ab0d9830eff1e8c079694))
- see prev commit by @elcoosp ([3f7f3a3](https://github.com/elcoosp/navi/commit/3f7f3a330ba03f7df22648ff0e2be9aa1e8afc5c))
- emit module tree and register_routes by @elcoosp ([c5883a5](https://github.com/elcoosp/navi/commit/c5883a5a68c7887e3bdb146f4a3b306f5e8eaf8f))
- working version with generated route tree by @elcoosp ([6621dc0](https://github.com/elcoosp/navi/commit/6621dc0d472efab295ed828b7bf0f0c0ba605696))
- add logo to readme by @elcoosp ([75db2b5](https://github.com/elcoosp/navi/commit/75db2b549e55f0c44ec2f181d9db7eb0adf52ec3))
- example with loaders by @elcoosp ([f9fea6d](https://github.com/elcoosp/navi/commit/f9fea6da2e2f411e80d9bc54e2b469de427d444f))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))
- init by @elcoosp ([e7488fb](https://github.com/elcoosp/navi/commit/e7488fb8d9bafba7bbfce6e5807cda5152ae3417))

### Miscellaneous
- inject missing metadata and point to root README by @elcoosp ([4fe4624](https://github.com/elcoosp/navi/commit/4fe4624455fc2b6f2476f3173374c27df1541314))
- inject precisely curated keywords for crates.io by @elcoosp ([ea31ad8](https://github.com/elcoosp/navi/commit/ea31ad8756433fd1b01ba3c2c2b5a099187828da))
- logo by @elcoosp ([4bfc15e](https://github.com/elcoosp/navi/commit/4bfc15ebbeaf218119b0257ccab8a751e0e936a5))
- readme by @elcoosp ([48a8988](https://github.com/elcoosp/navi/commit/48a89889c2cc23ec6188bdf8b8ea0fa090e2b709))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([2e5ce01](https://github.com/elcoosp/navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- final fixes, docs, and cleanup for decoupling release by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([d34fa5c](https://github.com/elcoosp/navi/commit/d34fa5c45ae26f1c51dab77fba5a61bff9092948))
- run fix by @elcoosp ([02a760a](https://github.com/elcoosp/navi/commit/02a760a645d4fe52d944eaea24274cca92b370e3))
- up readme by @elcoosp ([2549ff1](https://github.com/elcoosp/navi/commit/2549ff1c625f915fd12df0fbe44d75e092348b4c))

### Refactor
- rename gpui-navi → navi across repo and docs by @elcoosp ([e7938aa](https://github.com/elcoosp/navi/commit/e7938aa5d34ff270e199ec65c4bc514f1c66b8f6))
- remove global app parameters from route register code-gen by @elcoosp ([e206987](https://github.com/elcoosp/navi/commit/e2069877943f6b79f319585ad11d7c54b7a94093))
- update codegen for core/adapter split by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([aafec04](https://github.com/elcoosp/navi/commit/aafec041ac9d91689d54439a046b24e805c5bfae))
- replace regex with syn AST parsing by @elcoosp ([0a7481c](https://github.com/elcoosp/navi/commit/0a7481ca19c407e04d14df92349cf268f04d67ae))

### Testing
- add unit tests for History, Blocker, Validation; remove broken integration test by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([7843bc1](https://github.com/elcoosp/navi/commit/7843bc1f3231eb7ab66fb75d1ce2b125fd64cca9))
- fix optional segment skipped test assertions by @elcoosp ([f8ce755](https://github.com/elcoosp/navi/commit/f8ce7555df911fd9459d1cd95a5cbbf0daf7311a))

### build
- remove unused deps and inherit workspace lints in navi-codegen by @elcoosp ([71108ee](https://github.com/elcoosp/navi/commit/71108eef65f7c11d16a0465e850255066c497043))
**Full Changelog**: https://github.com/elcoosp/navi/releases/tag/v0.2.0
