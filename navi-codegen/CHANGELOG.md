# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-11

### Bug Fixes
- scanner ancestor chain bug resolved and fixed blocking route freeze by @elcoosp ([31ab98c](https://github.com/elcoosp/gpui-navi/commit/31ab98cb7f980b0b6afcc22a08e988eab0ab30ad))
- add non_snake_case allow only for modules with special characters by @elcoosp ([c0cbecd](https://github.com/elcoosp/gpui-navi/commit/c0cbecda0f3a961c4fd7fe21f8e86ce1269be506))
- generator now produces snake_case module names and avoids unused imports by @elcoosp ([1382584](https://github.com/elcoosp/gpui-navi/commit/13825842f011ecd7a76db1eacd6803f8ecfe53e4))
- simplify 404 registration, clean up route file imports by @elcoosp ([40ac726](https://github.com/elcoosp/gpui-navi/commit/40ac726365b8cd35d788a895b1868918ab7c5982))
- remove use navi_router::RouteDef; imports by @elcoosp ([4e629cd](https://github.com/elcoosp/gpui-navi/commit/4e629cd273ca1ccc2c5b5f399d7142982a15a516))
- run clippy by @elcoosp ([caed1eb](https://github.com/elcoosp/gpui-navi/commit/caed1ebb7d94de15ef9edb7eec1bcf5c79d4a0b5))
- correct get_loader_state using rs-query API, clean up warnings by @elcoosp ([65b1b78](https://github.com/elcoosp/gpui-navi/commit/65b1b78885e7cf830a72886b0e7e0fe52d8154ed))
- assign parents purely in-memory by @elcoosp ([2a23940](https://github.com/elcoosp/gpui-navi/commit/2a23940550462d773c8398a8964bd8b05cdf4a94))
- separate module ident from url segment for $ files by @elcoosp ([cdb2f3f](https://github.com/elcoosp/gpui-navi/commit/cdb2f3fdd9048c0ebd072cc97299a6c73b3b3197))
- resolve borrow checker, import, and macro API issues in Outlet, RouterState, and example by @elcoosp ([c3c7312](https://github.com/elcoosp/gpui-navi/commit/c3c7312b2f4c1e305345fb4d1b1b8492fad09193))
- correctly parse dollar filenames, parent hierarchy, and emit compilable route stubs by @elcoosp ([be6b6fa](https://github.com/elcoosp/gpui-navi/commit/be6b6facfc02b6479d07669aeeffdc2591add47c))
- use edition = 2024 by @elcoosp ([b69aa48](https://github.com/elcoosp/gpui-navi/commit/b69aa48163b83b94a1f1141ada036cdc7ca3be2f))

### Features
- introduce RouteKind enum, cache regexes and add scanner test suite by @elcoosp ([58edc38](https://github.com/elcoosp/gpui-navi/commit/58edc38ba9d76ebb5cfbe5c1b558bb09689dd210))
- make codegen idempotent and add comprehensive tests by @elcoosp ([b633319](https://github.com/elcoosp/gpui-navi/commit/b633319afd40c36adb482be65476ecb888a27719))
- add NaviConfig::discover, serde defaults and comprehensive tests by @elcoosp ([6cb0c19](https://github.com/elcoosp/gpui-navi/commit/6cb0c1973b63e38791043f2e1e5e9df89394090f))
- chunk 3 parity by @elcoosp ([33e6e49](https://github.com/elcoosp/gpui-navi/commit/33e6e49a78f20ffea5c36faf1e6abe1739ba636c))
- see prev commit by @elcoosp ([3f7f3a3](https://github.com/elcoosp/gpui-navi/commit/3f7f3a330ba03f7df22648ff0e2be9aa1e8afc5c))
- emit module tree and register_routes by @elcoosp ([c5883a5](https://github.com/elcoosp/gpui-navi/commit/c5883a5a68c7887e3bdb146f4a3b306f5e8eaf8f))
- working version with generated route tree by @elcoosp ([6621dc0](https://github.com/elcoosp/gpui-navi/commit/6621dc0d472efab295ed828b7bf0f0c0ba605696))
- example with loaders by @elcoosp ([f9fea6d](https://github.com/elcoosp/gpui-navi/commit/f9fea6da2e2f411e80d9bc54e2b469de427d444f))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/gpui-navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))
- init by @elcoosp ([e7488fb](https://github.com/elcoosp/gpui-navi/commit/e7488fb8d9bafba7bbfce6e5807cda5152ae3417))

### Miscellaneous
- fmt by @elcoosp in [#3](https://github.com/elcoosp/gpui-navi/pull/3) ([2e5ce01](https://github.com/elcoosp/gpui-navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/gpui-navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- final fixes, docs, and cleanup for decoupling release by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([d34fa5c](https://github.com/elcoosp/gpui-navi/commit/d34fa5c45ae26f1c51dab77fba5a61bff9092948))
- run fix by @elcoosp ([02a760a](https://github.com/elcoosp/gpui-navi/commit/02a760a645d4fe52d944eaea24274cca92b370e3))

### Refactor
- remove global app parameters from route register code-gen by @elcoosp ([e206987](https://github.com/elcoosp/gpui-navi/commit/e2069877943f6b79f319585ad11d7c54b7a94093))
- update codegen for core/adapter split by @elcoosp in [#2](https://github.com/elcoosp/gpui-navi/pull/2) ([aafec04](https://github.com/elcoosp/gpui-navi/commit/aafec041ac9d91689d54439a046b24e805c5bfae))
- replace regex with syn AST parsing by @elcoosp ([0a7481c](https://github.com/elcoosp/gpui-navi/commit/0a7481ca19c407e04d14df92349cf268f04d67ae))

### Testing
- add unit tests for History, Blocker, Validation; remove broken integration test by @elcoosp in [#3](https://github.com/elcoosp/gpui-navi/pull/3) ([7843bc1](https://github.com/elcoosp/gpui-navi/commit/7843bc1f3231eb7ab66fb75d1ce2b125fd64cca9))

### build
- remove unused deps and inherit workspace lints in navi-codegen by @elcoosp ([71108ee](https://github.com/elcoosp/gpui-navi/commit/71108eef65f7c11d16a0465e850255066c497043))
**Full Changelog**: https://github.com/elcoosp/gpui-navi/releases/tag/v0.2.0
