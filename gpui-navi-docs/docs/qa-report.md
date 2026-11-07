# QA Report — gpui-navi docs site

**Build:** 24 pages · Astro 7.4.7 · Starlight 0.42.10
**Gates (all green at the time of this report):**

| Gate | Result |
| :--- | :--- |
| `pnpm build` | exit 0, **zero** warning lines (asserted via grep on `.qa/build.log`) |
| `pnpm check` | **0 errors, 0 warnings, 0 hints** on TypeScript 6.0.3 |
| `pnpm lint` | 0 issues in 24 files (markdownlint-cli2) |
| `pnpm qa` | exit 0 — **24 routes × 2 viewports × 2 themes = 96 screenshots** |
| Search probe | `Ctrl/Cmd+K` opens the Pagefind modal; "blocker" → 8 results with titles + excerpts |

Automated QA failures per screenshot: console errors, uncaught page errors,
HTTP ≥ 400, horizontal overflow, missing/duplicate `<h1>`, `<main>` under 200
characters, theme not applied, broken `<img>`. All zero across all 96 shots.

## Per-route results

Legend: ✓ = automated checks passed; 👁 = included in the manual eyeball pass.
No defect was found; nothing was accepted-with-defect.

| Route | desktop-light | desktop-dark | mobile-light | mobile-dark | Notes |
| :--- | :---: | :---: | :---: | :---: | :--- |
| `/` | ✓ 👁 | ✓ 👁 | ✓ 👁 | ✓ | Splash hero, CardGrid, LinkCard, frameworks table; logo replacesTitle |
| `/404/` | ✓ 👁 | ✓ 👁 | ✓ 👁 | ✓ | Custom 404 entry rendered by the catch-all route (`disable404Route: true`) |
| `/tutorials/getting-started/` | ✓ 👁 | ✓ 👁 | ✓ | ✓ | Shiki multi-token highlighting, copy buttons, ToC, prev/next |
| `/guides/installation/` | ✓ 👁 | ✓ 👁 | ✓ | ✓ | Tabs (Cargo.toml / cargo add), Note aside, per-framework table |
| `/guides/file-based-routing/` | ✓ 👁 | ✓ 👁 | ✓ | ✓ | Naming table, FileTree, Tip + Caution asides |
| `/guides/loaders-and-suspense/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/route-guards/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/navigation-blockers/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/search-params/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/validation/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/preloading/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/scroll-restoration/` | ✓ | ✓ | ✓ | ✓ | |
| `/guides/devtools/` | ✓ | ✓ | ✓ | 👁 | Tables + inline code legible on mobile dark |
| `/reference/crates/` | ✓ | ✓ | ✓ | ✓ | |
| `/reference/navi-config/` | ✓ | ✓ | ✓ | ✓ | |
| `/reference/define-route/` | ✓ | ✓ | ✓ | ✓ | |
| `/reference/macros/` | ✓ | ✓ | ✓ | ✓ | |
| `/reference/components/` | ✓ 👁 | ✓ | ✓ | ✓ | Longest tables — no column cut off |
| `/reference/runtime/` | ✓ | ✓ | ✓ | ✓ | |
| `/reference/core-types/` | ✓ | ✓ | ✓ | ✓ | |
| `/explanation/architecture/` | ✓ | ✓ | ✓ | ✓ | |
| `/explanation/framework-agnosticism/` | ✓ | ✓ | ✓ | ✓ | |
| `/explanation/route-matching/` | ✓ | ✓ | ✓ | ✓ | |
| `/explanation/performance/` | ✓ | ✓ | ✓ | ✓ | Benchmark tables render without overflow |

## Manual eyeball checklist

- [x] **Logo sharp** — SVG with `viewBox`; header crops magnified 3× (`zz-logo-zoom-*.png`): crisp edges in both themes, no pixelation/halo/clipping; dark variant wordmark switches to `#E2E8F0`.
- [x] **Brand accent** — Rust orange (`hsl(19 …)`) visible on hero buttons, sidebar active item, links, focus states; not Starlight default blue.
- [x] **Code blocks** — multi-token Shiki highlighting (rust/toml/json/bash), copy button present, horizontal scroll inside the block (no page overflow); line highlighting not used (no config need).
- [x] **Tables** — fit the content column; long tokens wrap inside `code` spans; no cut-off columns.
- [x] **Callouts / CardGrid / Tabs / FileTree** — Note (blue), Tip (violet), Caution (amber) asides render with icons; cards and tabs styled correctly.
- [x] **Sidebar** — correct active state, four Diátaxis groups, no duplicates.
- [x] **On-page TOC** — right rail present on long pages, current heading highlighted.
- [x] **Pagination** — prev/next pairs verified on sampled pages (Installation ↔ File-Based Routing, Macros ↔ Components ↔ Runtime).
- [x] **Mobile** — hamburger nav, no clipping/overlap, tables and code scroll inside their boxes.
- [x] **Dark mode** — no pure-white panels, no invisible text, code-block background distinct from page background.
- [x] **Search** — Pagefind modal opens via `Ctrl/Cmd+K`, returns titled, excerpted results for domain terms ("blocker").

## Fix log during QA bring-up

| Issue | Fix |
| :--- | :--- |
| Home route enumerated as `//` (empty slug in screenshot names) | Normalized `path.relative` result in `scripts/qa-screenshots.mjs`; re-ran full QA |
| `dist/404.html` not enumerated by the index.html walker | Added `/404` explicitly to the QA route list |
| Build warned: sidebar slug not found | `locales` used `en:` (expects `en/` content dir) → switched to the canonical single-locale `root:` config |
| Build warned: i18n collection missing/empty | Defined the Starlight `i18n` collection in `content.config.ts` + `src/content/i18n/en.json` |
| Build warned: `/404` route conflict | Set `disable404Route: true` (custom 404 entry ships in the docs collection) |
| Build warned: sitemap skipped without `site` | Set `site` (ADR 0006) + absolute robots.txt Sitemap line |
| `astro check` could not import `@astrojs/check` | pnpm peer-resolution bug: `@emnapi/runtime@2.0.0-beta.2` paired with `@emnapi/core@1.11.3` → override to `1.11.3`; plus `.npmrc` public-hoist for resolution |

## Re-run rule

Any change to config, CSS, logo, theme, or content invalidates this report —
re-run `pnpm build` → `pnpm preview` → `pnpm qa`, then update this file.
