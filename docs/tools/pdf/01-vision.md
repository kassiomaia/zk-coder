# PDF Authoring Tool — Vision and Goals

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Define what "production-grade PDF authoring tool for an AI
agent" means for this repository. Bound the scope so later phases do not
drift.

---

## Statement of intent

Give the agent a first-class tool surface for producing **professional,
publication-quality PDF documents** from natural-language prompts, with full
control over structure, typography, and layout, while staying inside the
established Rust / Tauri / Svelte stack and the existing design system.

The tool is for **two audiences at once**:

- the **AI agent**, which must see a predictable, typed, composable tool
  surface that it can plan against;
- the **professional end user**, who must see a polished result that looks
  like it came from a deliberate tool, not an "AI-generated document".

Both audiences are equally load-bearing. A feature that helps one at the
expense of the other is rejected.

---

## What "production-grade" means here

A PDF produced by this tool is considered production-grade when it meets all
of the following, every time:

1. **Structural integrity** — `%PDF` header, cross-reference table, `%%EOF`,
   deterministic page count, document metadata (`/Title`, `/Author`,
   `/Subject`, `/Keywords`, `/Creator`, `/Producer`) present and correct.
2. **Typographic integrity** — real font metrics, no character-count
   heuristics, no truncated lines, no silent overflow. Baselines align,
   widows/orphans are controlled, tab stops and leading are consistent.
3. **Layout integrity** — user-specified margins respected, page sizes
   honoured, pagination automatic, headers/footers and page numbers where
   requested, images embedded at their real pixel data (not placeholders),
   tables rendered with measurable column widths.
4. **Style consistency** — every document renders against an explicit theme
   (font family, scale, color, spacing) rather than hardcoded constants.
   Two documents produced with the same theme are visually consistent.
5. **Reproducibility** — given the same inputs, the tool produces the same
   bytes (modulo timestamps). Golden-file tests are possible and enforced.
6. **Observability** — the UI and tool result expose page count, warnings,
   embedded-asset list, and validation report, so both the agent and the
   user can reason about the output.

If any of these is missing, the tool is not production-grade yet.

---

## Capabilities the agent must gain

Grouped by category so the roadmap can map phases to them. Each is in scope
for the tool. None of these require external services — everything runs in
Rust in-process.

### Document-level

- Set page size (A4, Letter, Legal, A3, custom mm) and orientation.
- Set margins per side, independently.
- Set PDF metadata: title, author, subject, keywords, language, creator.
- Apply a named **theme** (font family, typographic scale, color palette,
  spacing scale) that drives all downstream rendering.
- Add/replace a **cover page**.
- Add/replace a **table of contents** that is auto-generated from headings.
- Attach **headers** and **footers** with page numbers, section titles, and
  document metadata tokens.

### Block-level (on a page)

- Headings (levels 1–6) with outline/bookmark entries.
- Paragraphs with justification, leading, and inline runs (bold, italic,
  code, link).
- Unordered and ordered lists, with nesting.
- Block quotes.
- Code blocks with monospace font and optional syntax-level styling.
- Tables with column sizing, header row, and cell-level alignment.
- Images embedded from an on-disk path, with caption and width/height.
- Horizontal rules and explicit page breaks.
- Callout / note boxes (professional documents need them, and our design
  system already has callout styling we can mirror).

### Font-level

- Embed custom TTF/OTF fonts (regular, bold, italic, bold-italic, mono).
- Fallback chain: if a glyph is missing from the primary family, use the
  next font in the chain.
- Use **real glyph metrics** for line wrapping (not character-count).

### Interactive / structural

- Internal hyperlinks (table-of-contents → heading, footnote refs).
- External hyperlinks.
- PDF outline (bookmarks) mirroring the heading tree.

### Validation and reporting

- Pre-finalize validation report (structural, typographic, asset).
- Post-finalize validation report (true structural parse, not substring
  search).
- Human-readable warning list surfaced in the `DocumentGenerationCard`.

---

## User journey

We design from the user story outward, not from the library inward.

### Journey A — Agent authors a report end-to-end

1. User asks the agent: *"Write me a 6-page report on X, in A4 with our
   brand typography, with a cover page, TOC, page numbers in the footer,
   and tables showing Y."*
2. Agent calls `pdf.start` with document-level settings (size, margins,
   theme name, metadata).
3. Agent calls `pdf.add_cover` with the cover-page content.
4. Agent calls `pdf.add_section` / `pdf.add_block` repeatedly. Each call
   yields typed progress the UI reflects live.
5. Agent calls `pdf.add_toc` once content is in place (TOC resolves
   against recorded headings).
6. Agent calls `pdf.finalize`. The backend produces the PDF on a background
   task, streams progress, validates, and emits `ArtifactCreated`.
7. The `DocumentGenerationCard` morphs to "ready", shows `6 pages, 2 images,
   1 warning`, and offers Download / Open / Copy-link.

### Journey B — Agent edits an existing document the user dropped in

Deferred to a later phase, tracked in `04-roadmap.md`. Requires `lopdf` or
equivalent. Explicitly **not** in the first POC.

### Journey C — Professional uses the tool directly

A developer uses the chat UI and the same tool surface to draft a PDF by
hand (no agent). The tool must feel first-class in this mode too, because
it validates that the surface is human-legible and not just LLM-friendly.

---

## Non-goals (explicit)

The following are **not** in scope for this initiative and must not creep
in. Adding any of these would violate "be tied to reality, avoid
hallucination".

- **OCR / PDF ingestion for LLM RAG.** That is a read path (see
  `research/industry.md`). This tool is a write path.
- **Digital signatures / PKCS#7 signing.** Not requested by the user
  prompt; belongs to a separate security workstream if ever needed.
- **PDF/A archival conformance.** Substantial compliance work; add only
  when a customer explicitly requires it.
- **Forms / AcroForms / widgets.** Not in the stated prompt.
- **Server-side rendering pipeline / browser print-to-PDF.** Wrong stack
  for this project. Rust in-process rendering only.
- **Template marketplace.** The theme system is local to the codebase.

---

## Success criteria

We declare the initiative successful when *all* of the following hold:

1. An agent can author a 6+ page report end-to-end with a theme, cover,
   TOC, headers/footers, images, tables, and internal hyperlinks, without
   the tool producing a single truncated line or placeholder string.
2. Golden-file tests lock the rendered output of reference documents so
   regressions are detectable byte-for-byte (modulo timestamps).
3. `PdfValidator` no longer uses substring searches for structural checks
   — it parses the output PDF and asserts against the object graph.
4. `DocumentGenerationCard` reflects structural facts (page count,
   embedded fonts, images, warnings) sourced from real backend events.
5. The `pdf/` module in `src-tauri/src/infrastructure/tools/documents/`
   remains within the soft 500-line limit per file from
   `docs/quality/guidelines/rust-backend.md`, because layout and theming
   live in their own submodules.
6. No hardcoded magic numbers remain in the render path: margins, font
   sizes, line heights, and colors are sourced from a `Theme` value.
7. The frontend uses only design-system tokens from `design-system.css`
   for the authoring surface — no new colors, no new shadows.

These are the acceptance gates that `03-poc-plan.md` and `04-roadmap.md`
work back from.
