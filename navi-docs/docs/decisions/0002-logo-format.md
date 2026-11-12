# 0002 — Logo format: SVG with viewBox, light/dark variants

**Status:** Accepted
**Date:** 2026-11-08

## Context

The repository ships two logo assets:

- `assets/logo/horizontal.svg` — the vector original: a compass icon with a
  Rust-orange gradient ring (`#FF6B2B → #C23B0A`) and the "Navi" wordmark,
  `viewBox="0 0 400 120"`.
- `docs/logo.png` — a 1264×1264 raster used by the README.

Using the raster in a header would pixelate when scaled, and the README
itself uses the raster while a vector original exists (a logo-quality debt in
the source repo worth fixing here).

## Decision

1. **Use the vector original.** `src/assets/logo-light.svg` is a copy of
   `assets/logo/horizontal.svg`, SVGO-optimized.
2. **Derive a dark variant.** `src/assets/logo-dark.svg` swaps the three
   slate `#1E293B` fills (arrows, compass center, "avi" text) for
   `#E2E8F0`; the gradient stays. This is not a new logo — it is the project
   logo adjusted for legibility on dark surfaces, following Starlight's
   light/dark logo pattern.
3. **Optimize with SVGO, preserving `viewBox`.** Note for SVGO 4.x:
   `removeViewBox`/`removeIds` are no longer configurable members of
   `preset-default` (the viewBox is kept by default), but `cleanupIds`
   minifies ids — it renames `gradRust` → `a` and rewrites every
   `url(#gradRust)` reference consistently, which preserves correctness. The
   optimization script (`scripts/optimize-logos.mjs` in the workspace) asserts
   after each run that the `viewBox` survived, the `#ff6b2b` gradient stop
   survived, and every `url(#…)` reference resolves within the document.
4. **Favicon** is derived from the logo's compass geometry only
   (`public/favicon.svg`, square `viewBox="25 20 80 80"`), not invented.
5. **`replacesTitle: true`** in the Starlight logo config, because the
   horizontal logo already contains the wordmark; the text title stays in the
   DOM for accessibility and SEO but is not shown twice.
6. The experimental Astro `svgOptimizer` was **not** enabled: it emits an
   "experimental feature" warning line, which the zero-warning release gate
   treats as a blocker, and the committed assets are already optimized.

## Consequences

- The logo is sharp at any zoom in both themes (verified in the QA
  screenshots at 2×).
- Any future logo change must re-run the optimization + assertion script and
  re-run visual QA.
