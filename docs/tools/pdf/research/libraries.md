# PDF Library Landscape — Research Notes

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Evidence-backed comparison of candidate Rust PDF libraries for the PDF authoring tool. Feeds `02-architecture.md` and `04-roadmap.md`.

---

## Scope

We need a library (or composition of libraries) that can satisfy the vision in
`01-vision.md`: agent-driven production of professional PDF documents with
controllable page geometry, fonts, typography, structural blocks (headings,
paragraphs, lists, tables, images, code blocks), metadata, and headers/footers
with page numbers.

This note captures the libraries explicitly called out in the request plus the
incumbent we are already on.

---

## Incumbent: `printpdf = "0.7"`

**Source of truth**: `src-tauri/Cargo.toml:72`, used in
`src-tauri/src/infrastructure/tools/documents/pdf/final.rs`.

- Mid-level Rust PDF builder. Owns the `PdfDocument` / `PdfPage` / `PdfLayer`
  abstractions we already consume.
- Current usage is intentionally minimal: we only call `add_builtin_font` with
  `BuiltinFont::Helvetica` and `HelveticaBold`, and emit glyphs via
  `layer.use_text(text, size, x, y, &font)`.
- `printpdf` does support external TTF/OTF embedding, raw image embedding,
  hyperlinks, bookmarks and layers. We simply do not use those APIs yet.
- Text measurement is not performed by the library itself — the caller must
  bring its own font-metrics story when wrapping lines.
- License permissive (MIT / Apache-2.0 dual).

**Implication**: most of the capability gap we feel today is *our* adapter, not
the library. We have unexercised surface area on `printpdf` we can pick up
before switching engines.

---

## `lopdf`

**Reference**: https://docs.rs/lopdf/latest/lopdf/

- Low-to-mid level PDF **object model** library. Gives direct access to the
  PDF dictionary/stream graph, plus helpers like the `dictionary!` macro.
- Strong on **reading, editing, merging, and re-saving** existing PDFs:
  - loading encrypted files (empty-password auto-decrypt)
  - text extraction
  - object renumbering, compression, and object-stream rewriting (PDF 1.5+
    size reduction of 11–61% per the crate docs)
  - text replacement (exact and partial)
  - multi-document merge with bookmark preservation
- **Does not** provide high-level layout: no paragraph engine, no tables, no
  native image placement helpers, no form field builders.
- Production-ready, MIT licensed, actively maintained. Requires Rust ≥ 1.85
  for object-stream support.

**Fit for us**: a natural choice if/when the tool needs to *edit* existing
PDFs (agent asked to amend a document the user supplied). It is **not** the
right tool to *author* a styled document from scratch on its own — it would
have us reinventing a layout engine at the object level.

---

## `genpdf`

**Reference**: https://docs.rs/genpdf/latest/genpdf/

- High-level, pure-Rust document generator built on top of **printpdf** +
  **rusttype**. This matters: choosing `genpdf` does not move us off the
  engine we are already using — it stacks a layout layer on top.
- Layout primitives:
  - `Element` trait with paragraph, image, and user-defined custom blocks
  - `StyledElement` / `StyledString` inheritance for typography
  - `FontCache` for family loading
  - `PageDecorator` for per-page headers/footers/margins
  - mm-based geometry, point-based font sizes, configurable alignment
- Known gaps called out in the crate docs:
  - **No native table element** — tables would still need to be composed from
    lower-level blocks or implemented ourselves
  - **No page number / TOC primitives**
  - **No native hyperlink primitives**
  - Font embedding is **whole-font only**: every loaded family adds ~100–200
    KiB to the output even if only a few glyphs are used. Docs recommend
    post-processing with `ps2pdf`.
  - Image embedding is feature-flagged.
- Dual licensed Apache-2.0 / MIT. Version pinned at 0.2.0 — small, focused
  crate, maintenance cadence is low.

**Fit for us**: `genpdf` is the cheapest way to gain paragraph wrapping,
styled strings, real font families, and header/footer decorators without
writing a layout engine ourselves. The headline risks are (a) missing table
primitives, (b) font-embedding bloat, (c) thin maintenance.

---

## `oxidize-pdf`

**Reference**: https://github.com/bzsanti/oxidizePdf

- **Both parser and generator**, pure Rust, zero C dependencies.
- Generation surface: text with font selection, color fills, image embedding,
  split / merge / rotate, metadata, page numbering, bounding boxes, element
  classification. Table *parsing* is advertised; table authoring is not
  explicitly documented.
- Security surface: RC4 / AES-128 / AES-256 encryption, PKCS#7 digital
  signature detection.
- Performance claim: 3–4k pages/second generation on the author’s harness.
- Testing claim: 7,993 tests, 99.3% success rate on a 9,000+ real-world PDF
  corpus. Active as of 2026-04 (v2.5.1). Smaller community than `lopdf`.
- MIT licensed.

**Fit for us**: the most feature-dense of the three on paper, and the only
candidate that bundles a parser. But it is the youngest project and has the
smallest community. Adopting it as the only engine would couple our roadmap
to an upstream we do not yet understand well.

---

## Out of scope from this request

- **Firecrawl / `pdf-inspector`** (see `industry.md`) — confirmed to be a
  PDF *parsing and classification* pipeline. It has no story for PDF
  authoring and is not a candidate engine here.

---

## Comparison matrix

| Capability | `printpdf` (current) | `lopdf` | `genpdf` | `oxidize-pdf` |
|---|---|---|---|---|
| Write new PDF from scratch | Yes (mid-level) | Yes (object-level) | Yes (high-level) | Yes |
| Read / edit existing PDF | No | **Yes** (strongest) | No | Yes |
| Paragraph layout + wrapping | No (we DIY) | No | **Yes** | Partial |
| Styled text runs | Manual | Manual | **Yes** | Partial |
| Custom TTF/OTF font embedding | Yes | Yes | Yes | Yes |
| Images | Yes | Manual | Yes (flagged) | Yes |
| Tables | No | No | No native | Parsing only |
| Headers / footers / margins | Manual | Manual | **Yes** | Partial |
| Page numbers | Manual | Manual | Manual | Yes |
| Hyperlinks / bookmarks | Yes | Yes (merge) | Manual | Partial |
| Metadata (title/author/subject) | Yes | Yes | Yes | Yes |
| Encryption | Limited | Yes | No | **Yes** |
| Maturity / community | Large | Large | Small | Small-growing |
| License | MIT/Apache | MIT | MIT/Apache | MIT |

---

## Working recommendation (feeds `02-architecture.md`)

1. **Keep `printpdf` as the rendering engine** for authoring, because it is
   the one we already build and ship against. Most of the current feature
   gap is in our adapter, not the engine.
2. **Introduce a `PdfRenderer` trait** behind our domain model so the engine
   is swappable. This lets us evaluate `genpdf` (for layout) or
   `oxidize-pdf` (for parser/edit) as a second implementation without
   rewriting the tool surface.
3. **Defer adopting `lopdf`** until we have an explicit *edit existing PDF*
   requirement. When that lands, `lopdf` is the correct choice.
4. **Do not adopt `oxidize-pdf` as the only engine in phase 1.** Revisit
   after POC evidence in `03-poc-plan.md` if we need features `printpdf`
   cannot give us cheaply (e.g. production-grade encryption).
5. **Do not adopt `genpdf` blindly.** Its lack of tables and its whole-font
   embedding footprint are real liabilities for a professional tool. If we
   reach for it, scope it to the paragraph/page-decorator concerns and keep
   tables/headers/footers under our own domain model.

The decision is revisited after POC-1 (see `03-poc-plan.md`).
