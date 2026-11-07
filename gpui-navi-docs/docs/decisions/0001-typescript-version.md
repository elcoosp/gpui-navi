# 0001 — TypeScript version: pin to 6.x

**Status:** Accepted
**Date:** 2026-11-08

## Context

`pnpm check` (`astro check`) does not support TypeScript 7. With TypeScript
7.0.2 installed, the check fails with:

> `astro check does not currently support TypeScript 7.0`

Astro's guidance is to stay on TypeScript 6.x; the experimental workaround
(`tsc` + `@astrojs/ts-content-mapper`) exists but is not the stable default.

## Decision

Pin `typescript` to the latest 6.x release — **`6.0.3`** — in
`package.json`:

```json
"typescript": "6.0.3"
```

## Consequences

- `astro check` runs on the supported line and reports real diagnostics.
- We re-evaluate when Astro announces TypeScript 7 support; the pin is a
  single line in `package.json`.
