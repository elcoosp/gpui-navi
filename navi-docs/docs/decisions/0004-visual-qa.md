# 0004 — Visual QA: Playwright screenshots of every route

**Status:** Accepted
**Date:** 2026-11-08

## Context

A green build does not prove the site works. The QA phase renders every built
route and inspects it, both automatically and by eye.

## Decision

`scripts/qa-screenshots.mjs` (run via `pnpm qa`) enumerates routes from the
build output — every `dist/**/index.html` maps to its URL path — and for each
route captures **desktop (1440×900)** and **mobile (390×844)** in **light and
dark** themes against the production preview server. It fails (exit 1) on:

- console errors and uncaught page errors,
- HTTP ≥ 400 responses (including missing assets and broken images),
- horizontal overflow,
- missing or duplicate `<h1>`,
- `<main>` text under 200 characters,
- the requested theme not being applied,
- broken `<img>` elements.

Screenshots are written to `.qa/screenshots/` and the failure report to
`.qa/failures.json`. `.qa/` is gitignored and excluded from the deliverable
archive.

An optional `@axe-core/playwright` pass (zero critical/serious violations per
route) was considered but not added at this time; the built-in probe set plus
the manual eyeball pass cover the current risk surface. Revisit when the site
grows interactive components beyond Starlight's defaults.

## Consequences

- `pnpm qa` exiting 0 is a release blocker, alongside zero-warning
  `pnpm build`, `pnpm check`, and `pnpm lint`.
- Any change to config, CSS, logo, theme, or content after a QA pass
  invalidates it — re-run the full chain before shipping.
- The per-route eyeball results are recorded in `docs/qa-report.md`.
