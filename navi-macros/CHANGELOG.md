# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-14

### Bug Fixes
- fix define_router! build_router signature to match RouterState::new by @elcoosp ([cf6fbed](https://github.com/elcoosp/navi/commit/cf6fbedfc0dd9316df926cca808d8061decab5a6))
- rm backups by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([66bd0d6](https://github.com/elcoosp/navi/commit/66bd0d6404a56da2840e8fbfbfed180722a77358))
- awaited loader by @elcoosp ([720c8cd](https://github.com/elcoosp/navi/commit/720c8cde87eed7b11a6b3f988ff9e6c78443ee0c))
- parallel loading working with demo in example dashboard by @elcoosp ([8963001](https://github.com/elcoosp/navi/commit/8963001beca53faf0507af7100f26bdd61992e26))
- fully qualified trait methods and () params handling by @elcoosp ([33eb389](https://github.com/elcoosp/navi/commit/33eb389010f35d6c449da1a002b00c57923942fb))
- remove use navi_router::RouteDef; imports by @elcoosp ([4e629cd](https://github.com/elcoosp/navi/commit/4e629cd273ca1ccc2c5b5f399d7142982a15a516))
- correct get_loader_state using rs-query API, clean up warnings by @elcoosp ([65b1b78](https://github.com/elcoosp/navi/commit/65b1b78885e7cf830a72886b0e7e0fe52d8154ed))
- correct use_can_go_back, remove loader side-effect, add use_match, use_loader_state by @elcoosp ([2ae4bf0](https://github.com/elcoosp/navi/commit/2ae4bf051422673277689d5c238f8f2bb92a7e41))
- resolve borrow checker, import, and macro API issues in Outlet, RouterState, and example by @elcoosp ([c3c7312](https://github.com/elcoosp/navi/commit/c3c7312b2f4c1e305345fb4d1b1b8492fad09193))
- use_loader_data retriggered while rendering & already loading by @elcoosp ([e233a07](https://github.com/elcoosp/navi/commit/e233a07fdb73f19492cfe905f8a2ba646a2e72ca))
- working loader declarative without suspense boundary by @elcoosp ([c1081e9](https://github.com/elcoosp/navi/commit/c1081e90ff82119ce737393d4b9f178c7e964b3f))
- working define_route and use_params by @elcoosp ([ba13847](https://github.com/elcoosp/navi/commit/ba13847d0983740c6db08d6161ddb37fac43d8e7))
- use edition = 2024 by @elcoosp ([b69aa48](https://github.com/elcoosp/navi/commit/b69aa48163b83b94a1f1141ada036cdc7ca3be2f))

### Documentation
- add Vercel documentation link to Cargo.toml by @elcoosp ([725838f](https://github.com/elcoosp/navi/commit/725838fe987aff7380e56051b17bba8c89f7882b))
- enforce SKILL.md rules (jsDelivr logos, absolute links, remove license sections) by @elcoosp ([3c6e69e](https://github.com/elcoosp/navi/commit/3c6e69ecec649d85954a2d527b44e42a569458e9))
- rewrite for framework-agnostic quoin architecture by @elcoosp ([2c1f239](https://github.com/elcoosp/navi/commit/2c1f2394f5d546eca166fc34b48f96a6e0455902))
- rewrite performance section highlighting allocation-free loops by @elcoosp ([eb6ed84](https://github.com/elcoosp/navi/commit/eb6ed84075c902ecbb2ef74289fc39d0e6a02e10))
- update dependency stack documentation for gpui-kit migration by @elcoosp ([9d61a86](https://github.com/elcoosp/navi/commit/9d61a86ed18daac9dfbdb2ab9f15709caabdc5cb))
- update README with new features, development workflow and quality gates by @elcoosp ([5530ac7](https://github.com/elcoosp/navi/commit/5530ac7e57450fe1736c2e0c47b4721f30b71224))

### Features
- chunk 4 parity by @elcoosp ([6cfdee6](https://github.com/elcoosp/navi/commit/6cfdee6c2734271af6d1edec5b2b517d741913fa))
- chunk 3 parity by @elcoosp ([33e6e49](https://github.com/elcoosp/navi/commit/33e6e49a78f20ffea5c36faf1e6abe1739ba636c))
- chunk 1 parity by @elcoosp ([c005d39](https://github.com/elcoosp/navi/commit/c005d39ff38a739a4bdd602a6dde828f71c6300c))
- nexum integration poc by @elcoosp ([f3a5b61](https://github.com/elcoosp/navi/commit/f3a5b61ad29381cd4e1ab0d9830eff1e8c079694))
- see prev commit by @elcoosp ([3f7f3a3](https://github.com/elcoosp/navi/commit/3f7f3a330ba03f7df22648ff0e2be9aa1e8afc5c))
- generate register_routes function by @elcoosp ([5468176](https://github.com/elcoosp/navi/commit/54681762980aa84e073cbcebe00e5e0e0fa51bca))
- integrate darling for span-accurate errors by @elcoosp ([dc22e06](https://github.com/elcoosp/navi/commit/dc22e0654fd92ef4136ef81c3eb32a430e027d26))
- working rs-query integration with loader by @elcoosp ([82a1057](https://github.com/elcoosp/navi/commit/82a10575b4e688731dbf63c5fd67d782f977f805))
- add logo to readme by @elcoosp ([75db2b5](https://github.com/elcoosp/navi/commit/75db2b549e55f0c44ec2f181d9db7eb0adf52ec3))
- wire component auto-registration, add parent key, fix loader side-effects by @elcoosp ([83f49a9](https://github.com/elcoosp/navi/commit/83f49a91d007cdb28d1dad0ee7f803c267f0fe95))
- correct validation search params by @elcoosp ([be285b1](https://github.com/elcoosp/navi/commit/be285b1ca5442446466201fd2b2914c17b5ddafd))
- example working with list sort from query params by @elcoosp ([8c7a02a](https://github.com/elcoosp/navi/commit/8c7a02a999774d4f425e30341c5b82c22d9d094d))
- compiling but freezing loader feature by @elcoosp ([3f81c25](https://github.com/elcoosp/navi/commit/3f81c25d2d95016f3ecfd408b3264b138cc1a4d9))
- try declarative loader but can't use it in example by @elcoosp ([47000fe](https://github.com/elcoosp/navi/commit/47000febebf901fd72baa2e8e6145dce583a429b))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))
- init by @elcoosp ([e7488fb](https://github.com/elcoosp/navi/commit/e7488fb8d9bafba7bbfce6e5807cda5152ae3417))

### Miscellaneous
- inject missing metadata and point to root README by @elcoosp ([4fe4624](https://github.com/elcoosp/navi/commit/4fe4624455fc2b6f2476f3173374c27df1541314))
- inject precisely curated keywords for crates.io by @elcoosp ([ea31ad8](https://github.com/elcoosp/navi/commit/ea31ad8756433fd1b01ba3c2c2b5a099187828da))
- update router.rs by @elcoosp ([31cac76](https://github.com/elcoosp/navi/commit/31cac768f2617fb72c14587a7ed744c44f8e9e03))
- update route.rs by @elcoosp ([52a31e6](https://github.com/elcoosp/navi/commit/52a31e604dcf8830029231051202c52eb4aedf06))
- update hooks.rs by @elcoosp ([ccae344](https://github.com/elcoosp/navi/commit/ccae344f098d967e52424f2cf642516e197eb033))
- logo by @elcoosp ([4bfc15e](https://github.com/elcoosp/navi/commit/4bfc15ebbeaf218119b0257ccab8a751e0e936a5))
- readme by @elcoosp ([48a8988](https://github.com/elcoosp/navi/commit/48a89889c2cc23ec6188bdf8b8ea0fa090e2b709))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([2e5ce01](https://github.com/elcoosp/navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- up readme by @elcoosp ([2549ff1](https://github.com/elcoosp/navi/commit/2549ff1c625f915fd12df0fbe44d75e092348b4c))

### Refactor
- rename gpui-navi → navi across repo and docs by @elcoosp ([e7938aa](https://github.com/elcoosp/navi/commit/e7938aa5d34ff270e199ec65c4bc514f1c66b8f6))
- switch params and search hooks to evaluate caches by @elcoosp ([abd2384](https://github.com/elcoosp/navi/commit/abd238472bfab56417d9d37cd7b7604f1635e85a))
- use IntoElement for route component registration by @elcoosp ([f0b8603](https://github.com/elcoosp/navi/commit/f0b8603a8681899600c0e5075e99c1fecd44cee3))
- rewrite define_route! parser with key-aware parsing and error reporting by @elcoosp ([3e63327](https://github.com/elcoosp/navi/commit/3e63327ad0a021fecc02720cfc4c127f513a418d))

### Testing
- fix optional segment skipped test assertions by @elcoosp ([f8ce755](https://github.com/elcoosp/navi/commit/f8ce7555df911fd9459d1cd95a5cbbf0daf7311a))

### build
- bump syn to 3.0, remove unused darling/convert_case, inherit lints by @elcoosp ([060aa3c](https://github.com/elcoosp/navi/commit/060aa3c4ee4a7b86e3c8b5892428857d13bc2f9b))
**Full Changelog**: https://github.com/elcoosp/navi/releases/tag/v0.2.0
