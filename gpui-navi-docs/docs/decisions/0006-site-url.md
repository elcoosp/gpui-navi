# 0006 — Site URL for sitemap generation

**Status:** Accepted
**Date:** 2026-11-08

## Context

Starlight enables the Astro sitemap integration. Without a `site` option in
`astro.config.mjs`, the build emits:

```
[WARN] [@astrojs/sitemap] The Sitemap integration requires the `site` astro.config option. Skipping.
```

The zero-warning release gate treats that line as a blocker, so `site` must
be set — but the repository does not declare a documentation deployment URL.

## Decision

Set `site: "https://elcoosp.github.io/gpui-navi/"` — the conventional GitHub
Pages origin for the `elcoosp/gpui-navi` repository. `public/robots.txt`
references the matching absolute `sitemap-index.xml` URL.

## Consequences

- The build emits no sitemap warning and produces a sitemap.
- If the docs are deployed elsewhere, change `site` **and** the `Sitemap:`
  line in `robots.txt` together, and re-run the full QA chain.
