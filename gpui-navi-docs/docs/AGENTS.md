# AGENTS.md — Working on the Navi documentation site

This is the Astro Starlight documentation site for
[gpui-navi](https://github.com/elcoosp/gpui-navi) ("Navi"), a file-based router
for Rust UI frameworks. It is a standalone project; the Navi source lives in
the parent repository, not here.

## Commands

| Command | What it does |
| :--- | :--- |
| `pnpm install` | Install dependencies. |
| `pnpm dev` | Dev server at `localhost:4321`. |
| `pnpm build` | Production build **plus** `dist/llms.txt` + `dist/llms-full.txt` generation. |
| `pnpm preview` | Serve the production build. |
| `pnpm check` | `astro check` — TypeScript/content diagnostics. |
| `pnpm lint` | markdownlint-cli2 over `src/content/docs/**/*.{md,mdx}`. |
| `pnpm qa` | Playwright screenshot + runtime QA over every built route (needs `pnpm build` + `pnpm preview` running). |

pnpm is the package manager of record. TypeScript is pinned to 6.x for
`astro check` compatibility (see `docs/decisions/0001-typescript-version.md`).

## Information architecture

Content follows the Diátaxis framework — **one type per page**, declared in
frontmatter (`type: tutorial | how-to | reference | explanation`):

| Type | Folder | What belongs there |
| :--- | :--- | :--- |
| Tutorial | `src/content/docs/tutorials/` | One golden path, learning-oriented, repeatable steps. |
| How-to | `src/content/docs/guides/` | Task-oriented steps to reach one goal. |
| Reference | `src/content/docs/reference/` | Description-oriented API/config surface, tables. |
| Explanation | `src/content/docs/explanation/` | Understanding-oriented design and background. |

Every page has `title`, `description`, and `type` frontmatter. The landing
page is `src/content/docs/index.mdx`.

## Content rules

- **Never invent features.** Document only what exists in the Navi source.
  When unsure, read the crate source in the parent repository (`navi-router/`,
  `navi-router-core/`, `navi-macros/`, `navi-codegen/`, `navi-devtools/`).
- Second person, present tense, active voice, imperative instructions.
- Banished words: "simply", "just", "easy", "obviously".
- Every code block declares a language tag.
- Internal links are validated at build time by `starlight-links-validator` —
  a broken internal link fails `pnpm build`.
- Run `pnpm lint` and `pnpm check` after content changes; run the full QA
  (`pnpm build` → `pnpm preview` → `pnpm qa`) after any config, CSS, logo, or
  theme change.

## Branding

- Accent colors derive from the project logo's Rust-orange gradient
  (`#FF6B2B → #C23B0A`) — see `src/styles/custom.css`.
- Logo variants live in `src/assets/` (`logo-light.svg`, `logo-dark.svg`),
  derived from the repository's `assets/logo/horizontal.svg`, SVGO-optimized
  with `viewBox` preserved (see `docs/decisions/0002-logo-format.md`).
- Keep Starlight's built-in dark/light toggle; verify contrast in both themes.

## QA

`scripts/qa-screenshots.mjs` enumerates every `dist/**/index.html` route and
screenshots each one at desktop (1440×900) and mobile (390×844) in light and
dark themes, failing on console errors, HTTP ≥ 400, horizontal overflow,
missing/duplicate `<h1>`, empty `<main>`, theme-application failure, or broken
images. Artifacts land in `.qa/` (gitignored, never shipped).
