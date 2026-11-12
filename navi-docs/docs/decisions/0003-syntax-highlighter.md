# 0003 — Syntax highlighter: Shiki via Expressive Code (default)

**Status:** Accepted
**Date:** 2026-11-08

## Context

Starlight renders code blocks through Expressive Code, whose supported
highlighter is Shiki (VS Code TextMate grammars, 180+ languages, dual themes,
build-time execution, zero client-side JavaScript). A lightweight alternative
highlighter (e.g. Rangi, ~13 kB, ~46 languages, regex tokenizer) exists for
chat UIs and small apps, but it is not integrated with Expressive Code.

## Decision

Ship with Starlight's default Shiki setup — no custom highlighter, no
replacement plugin. The default configuration already provides:

- dual light/dark themes tied to Starlight's theme toggle,
- the copy button, line highlighting (`{1,3-5}`), and word wrap,
- the `ansi` language tag for terminal output,
- zero client-side highlighter JavaScript.

No `expressiveCode` overrides were needed: the docs use Rust, TOML, JSON,
bash, and fenced text — all covered by the default grammar set, and no
custom theme was required since the brand accent is applied through CSS.

## Consequences

- All fenced code blocks must declare a language (enforced by review and
  markdownlint); an unlabeled fence renders as plain text.
- If a future page needs a language outside the defaults, Shiki loads
  grammars on demand — record it here if the default set must be pinned.
