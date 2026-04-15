# Industry PDF Pipelines — Research Notes

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Capture external engineering lessons that are actually relevant
to our goal of building an **agent-driven PDF authoring tool**. Filter out
anything that is only about parsing.

---

## Source: Firecrawl — "Fire-PDF Launch"

**Reference**: https://www.firecrawl.dev/blog/fire-pdf-launch

### What Fire-PDF is

Fire-PDF is a **parser / ingestion pipeline** optimized for feeding downstream
RAG and LLM workflows. Its core idea is a *classification-first* strategy:

- A fast Rust classifier (`pdf-inspector`) inspects font encodings, text
  operators, and image coverage and decides whether a page is text-native or
  image-native.
- Text-native pages are extracted in milliseconds without touching GPU.
- Only image-native pages are routed to a vision-language model (GLM-OCR).
- A neural layout model detects text blocks, tables, formulas, images,
  headers, and footers as **distinct region types**, with per-region
  handling (LaTeX for formulas, extended time budgets for tables, XY-cut
  for multi-column reading order).
- Lane-based routing keeps large documents from blocking small ones.

The headline performance claim is 3.5–5.7× faster end-to-end on their
internal corpus.

### What it is **not**

Fire-PDF says nothing about *writing* PDFs, embedding fonts, composing
layouts, or emitting output files. It is a read path. The companion
"kucoin.com/news/flash" article summarizing the same launch is a secondary
re-post of the same ground and does not add engineering guidance beyond the
primary post.

### Lessons we can legitimately borrow

Even though Fire-PDF is a reader, a small number of its architectural
decisions transfer cleanly to an authoring tool:

1. **Classify before acting.** In the read path they classify pages before
   choosing a strategy. On the write path, our tool should classify *blocks*
   (paragraph vs. code vs. table vs. image vs. callout) and route each to a
   renderer configured for that region type. One uniform renderer produces
   mediocre output for every kind of block.
2. **Region-specific handling is a feature, not noise.** Tables, code, and
   images want different measurement, spacing, and wrapping rules than
   prose. Our domain model in `02-architecture.md` reflects this.
3. **Lane isolation.** Their pipeline avoids coupling fast jobs behind slow
   jobs. For us this maps to: finalization of a large PDF must not block
   the tool runtime or the streaming UI. Rendering should be an async
   background job with structured progress events, not a blocking call
   inside `execute`.
4. **Reading order matters.** Multi-column layouts need explicit ordering.
   Even though we are the *author*, we decide reading order implicitly via
   our block sequence — so our domain model should guarantee a total order
   of blocks per page and never rely on post-hoc rediscovery.

### Lessons we are explicitly **not** taking from this source

- No recommendation about a PDF *generator* library.
- No recommendation about font embedding or glyph metrics.
- No recommendation about encryption, signatures, or accessibility tagging.

These are out of scope for Firecrawl's write-up and we will not invent
recommendations we cannot source.

---

## Other sources we considered

- `https://www.kucoin.com/news/flash/firecrawl-rewrites-pdf-parser-in-rust-speed-boosts-up-to-5-7x`
  — restates the Firecrawl launch. No net new signal. Not re-cited below.

- `bzsanti/oxidizePdf` is covered in `libraries.md`, not here.

We did **not** expand research into unrelated PDF SaaS vendors, browser
print-to-PDF pipelines, or LaTeX toolchains. They do not match the chosen
stack (Rust + Tauri + Svelte) and would not produce actionable guidance for
the current tool.

---

## How this influences the plan

| Borrowed lesson | Where it lands |
|---|---|
| Block/region classification drives rendering | `02-architecture.md → Domain Model` |
| Async, non-blocking finalization with progress | `02-architecture.md → Communication Layer` |
| Explicit total block order per page | `02-architecture.md → Domain Model` |
| Classification before action | `04-roadmap.md → Phase 2` |
