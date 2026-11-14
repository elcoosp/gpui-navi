# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.2.0] - 2026-11-14

### Bug Fixes
- correct tab index mapping and add regression test by @elcoosp ([62a58d6](https://github.com/elcoosp/navi/commit/62a58d6611bc64760ff03a79f0433865a0f34267))
- run clippy by @elcoosp ([caed1eb](https://github.com/elcoosp/navi/commit/caed1ebb7d94de15ef9edb7eec1bcf5c79d4a0b5))
- rm warns by @elcoosp ([56909bb](https://github.com/elcoosp/navi/commit/56909bb86ae7ba07d83233d925e28c03318f675e))
- build by @elcoosp ([68a2429](https://github.com/elcoosp/navi/commit/68a2429013caa638e2955db28436a4301944a6ca))
- event detail log icon by @elcoosp ([b1ad9ae](https://github.com/elcoosp/navi/commit/b1ad9aeabdf4ddc02d23714907e9dc4e1cd716f5))
- include search in event log by @elcoosp ([6d9bed6](https://github.com/elcoosp/navi/commit/6d9bed6a4deff833c5c6618fd986ecb02a784e43))
- keybinding in expanded and collapsed state by @elcoosp ([db268f7](https://github.com/elcoosp/navi/commit/db268f7d57fb69d07f3cac69a3e94ea88d0e3086))
- ui timeline no events by @elcoosp ([3bd267d](https://github.com/elcoosp/navi/commit/3bd267dc5b2165093504520e356962332335959d))
- rm subscribe input search event by @elcoosp ([62d3416](https://github.com/elcoosp/navi/commit/62d34164efd9243220dd72a498ba08e7dd49e949))
- use_loader_data retriggered while rendering & already loading by @elcoosp ([e233a07](https://github.com/elcoosp/navi/commit/e233a07fdb73f19492cfe905f8a2ba646a2e72ca))
- working define_route and use_params by @elcoosp ([ba13847](https://github.com/elcoosp/navi/commit/ba13847d0983740c6db08d6161ddb37fac43d8e7))
- devtools tab click by @elcoosp ([38c8913](https://github.com/elcoosp/navi/commit/38c8913bb460398750c4d95b92a398d999a91a19))
- use edition = 2024 by @elcoosp ([b69aa48](https://github.com/elcoosp/navi/commit/b69aa48163b83b94a1f1141ada036cdc7ca3be2f))

### Documentation
- add Vercel documentation link to Cargo.toml by @elcoosp ([725838f](https://github.com/elcoosp/navi/commit/725838fe987aff7380e56051b17bba8c89f7882b))
- enforce SKILL.md rules (jsDelivr logos, absolute links, remove license sections) by @elcoosp ([3c6e69e](https://github.com/elcoosp/navi/commit/3c6e69ecec649d85954a2d527b44e42a569458e9))
- rewrite for framework-agnostic quoin architecture by @elcoosp ([2c1f239](https://github.com/elcoosp/navi/commit/2c1f2394f5d546eca166fc34b48f96a6e0455902))
- rewrite performance section highlighting allocation-free loops by @elcoosp ([eb6ed84](https://github.com/elcoosp/navi/commit/eb6ed84075c902ecbb2ef74289fc39d0e6a02e10))
- update dependency stack documentation for gpui-kit migration by @elcoosp ([9d61a86](https://github.com/elcoosp/navi/commit/9d61a86ed18daac9dfbdb2ab9f15709caabdc5cb))
- update README with new features, development workflow and quality gates by @elcoosp ([5530ac7](https://github.com/elcoosp/navi/commit/5530ac7e57450fe1736c2e0c47b4721f30b71224))
- quoin mig & rm old files by @elcoosp ([f6f6632](https://github.com/elcoosp/navi/commit/f6f6632fe30ba54bbb06e0d8ef83622457857f06))

### Features
- add clear blockers to devtools by @elcoosp ([59b2f42](https://github.com/elcoosp/navi/commit/59b2f42e76b6064988ddaa6822f6851ab18aca37))
- integration with nexum by @elcoosp ([ea18b8d](https://github.com/elcoosp/navi/commit/ea18b8d43be5367bb3362cfa65e968e2663eb149))
- nexum integration poc by @elcoosp ([f3a5b61](https://github.com/elcoosp/navi/commit/f3a5b61ad29381cd4e1ab0d9830eff1e8c079694))
- cache datatable by @elcoosp ([f6d3bb3](https://github.com/elcoosp/navi/commit/f6d3bb34432c306add5b92a223a7bdba33b07fb6))
- working rs-query integration with loader by @elcoosp ([82a1057](https://github.com/elcoosp/navi/commit/82a10575b4e688731dbf63c5fd67d782f977f805))
- add logo to readme by @elcoosp ([75db2b5](https://github.com/elcoosp/navi/commit/75db2b549e55f0c44ec2f181d9db7eb0adf52ec3))
- add advanced filtering, collapsible route tree, and route testing playground by @elcoosp ([0fde988](https://github.com/elcoosp/navi/commit/0fde988dae2631871094de72d2e9cf73ff179489))
- cache tab by @elcoosp ([192ba0f](https://github.com/elcoosp/navi/commit/192ba0f3b203f7dd66e3eee116b4fa1ac3e865d3))
- jump to state by @elcoosp ([8dc17fe](https://github.com/elcoosp/navi/commit/8dc17fe87494e700b945b8abfe350843a8557a5e))
- better event render by @elcoosp ([c36a92f](https://github.com/elcoosp/navi/commit/c36a92fc194b95ab7bbfb94dc6cd1e11fc4790cb))
- keyboard nav ctrl-1-4 by @elcoosp ([0124447](https://github.com/elcoosp/navi/commit/0124447bef2e938b9baaf981db2ca85b37a6e53f))
- add keybinding by @elcoosp ([2d4836a](https://github.com/elcoosp/navi/commit/2d4836a798d4a12273aaf0a618803959c7d1fb8d))
- working search highlight in timeline by @elcoosp ([b4f77c1](https://github.com/elcoosp/navi/commit/b4f77c1eff25e9892d48ea18f5205759ee6ec427))
- better timeline by @elcoosp ([46c619c](https://github.com/elcoosp/navi/commit/46c619c6eef0ef8f7cb2dc7810ef22b209b49893))
- icons working but not text by @elcoosp ([287184d](https://github.com/elcoosp/navi/commit/287184ded63c6d1d108e9c145fa5e185689780e3))
- correct validation search params by @elcoosp ([be285b1](https://github.com/elcoosp/navi/commit/be285b1ca5442446466201fd2b2914c17b5ddafd))
- use define route by @elcoosp ([b2ed371](https://github.com/elcoosp/navi/commit/b2ed371b364d3b778f61577b05b65ef40f1a0cc8))
- example with loaders by @elcoosp ([f9fea6d](https://github.com/elcoosp/navi/commit/f9fea6da2e2f411e80d9bc54e2b469de427d444f))
- compiling crates by @elcoosp ([83da0d4](https://github.com/elcoosp/navi/commit/83da0d433e10b485972397c987a56b5dfd25782e))
- init by @elcoosp ([e7488fb](https://github.com/elcoosp/navi/commit/e7488fb8d9bafba7bbfce6e5807cda5152ae3417))

### Miscellaneous
- inject missing metadata and point to root README by @elcoosp ([4fe4624](https://github.com/elcoosp/navi/commit/4fe4624455fc2b6f2476f3173374c27df1541314))
- inject precisely curated keywords for crates.io by @elcoosp ([ea31ad8](https://github.com/elcoosp/navi/commit/ea31ad8756433fd1b01ba3c2c2b5a099187828da))
- update lib.rs by @elcoosp ([ab0118b](https://github.com/elcoosp/navi/commit/ab0118b770538042d5121c14f190e6c651b0f41e))
- update devtools.rs by @elcoosp ([665455c](https://github.com/elcoosp/navi/commit/665455c637c74b311d6fd33f7b579ba0aa0937a9))
- update Cargo.toml by @elcoosp ([6f900f1](https://github.com/elcoosp/navi/commit/6f900f1e33b6066e5adf01c0e8c86047f50f488f))
- logo by @elcoosp ([4bfc15e](https://github.com/elcoosp/navi/commit/4bfc15ebbeaf218119b0257ccab8a751e0e936a5))
- readme by @elcoosp ([48a8988](https://github.com/elcoosp/navi/commit/48a89889c2cc23ec6188bdf8b8ea0fa090e2b709))
- fmt by @elcoosp in [#3](https://github.com/elcoosp/navi/pull/3) ([2e5ce01](https://github.com/elcoosp/navi/commit/2e5ce01a6c447512f76bc98ca1cdbc9ad3952df1))
- bump all crate versions to 0.2.0 using workspace version by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([7e7fcf9](https://github.com/elcoosp/navi/commit/7e7fcf93ca75679f10cd18ac7eb8821cd3d8bb70))
- final fixes, docs, and cleanup for decoupling release by @elcoosp in [#2](https://github.com/elcoosp/navi/pull/2) ([d34fa5c](https://github.com/elcoosp/navi/commit/d34fa5c45ae26f1c51dab77fba5a61bff9092948))
- add rs-query devtools feature to navi-devtools by @elcoosp ([e3695ee](https://github.com/elcoosp/navi/commit/e3695eee5cff3c14e8e55aba578cad8cb0f142c6))
- up readme by @elcoosp ([2549ff1](https://github.com/elcoosp/navi/commit/2549ff1c625f915fd12df0fbe44d75e092348b4c))

### Refactor
- rename gpui-navi → navi across repo and docs by @elcoosp ([e7938aa](https://github.com/elcoosp/navi/commit/e7938aa5d34ff270e199ec65c4bc514f1c66b8f6))
- fix clippy pedantic lints in devtools timeline by @elcoosp ([26eeab0](https://github.com/elcoosp/navi/commit/26eeab033882fab5459f28d2cf2b182aefcd06e5))
- major devtools refactor: extract render_node, remove nexum, fix clippy lints by @elcoosp ([0dbd87c](https://github.com/elcoosp/navi/commit/0dbd87cbd8607e732cc41b6e0e71b84b70ea002f))
- remove deep_link_view.rs (nexum feature dropped) by @elcoosp ([47f2459](https://github.com/elcoosp/navi/commit/47f24592bf0caf78d8e2110dc50e02f79e20dcd0))

### Testing
- fix optional segment skipped test assertions by @elcoosp ([f8ce755](https://github.com/elcoosp/navi/commit/f8ce7555df911fd9459d1cd95a5cbbf0daf7311a))

### build
- remove unused deps and nexum feature from navi-devtools by @elcoosp ([0b3364b](https://github.com/elcoosp/navi/commit/0b3364b7207c9e91e30675524537408ece3027cf))
**Full Changelog**: https://github.com/elcoosp/navi/releases/tag/v0.2.0
