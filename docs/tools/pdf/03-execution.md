# PDF Authoring Tool — Execution Plan

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: The single source of truth for *what to build next*. Replaces
the previous `03-poc-plan.md` and `04-roadmap.md` (deleted). If a task is
not listed here, it is not in flight.

---

## Core rules of this plan

1. **Three milestones. No more.** M1 ships honest rendering. M2 ships
   structural documents. M3 ships UX integration. Nothing after M3 is on
   this plan — if scope grows, `01-vision.md` is edited first.
2. **Every task is one PR.** If it cannot be one PR, split it.
3. **Every task is test-first.** Failing tests are the first commit on
   the task branch.
4. **Every task retires a numbered gap** from `00-as-is.md`. If it does
   not, it should not exist.
5. **A milestone is Done when its exit gate is green.** No half-done
   milestones.
6. **Delete the superseded code in the same PR.** Per
   `docs/quality/guidelines/rust-backend.md § Refactor discipline`.

---

## Task template

Every task below follows this template. Read it once.

> **Tn.m — <Title>**
> **Deliverable**: one sentence describing the user or agent visible
>   outcome.
> **Build**: the concrete code.
> **Tests (written first)**: the failing tests that define success.
> **Done when**: the binary acceptance check.
> **Retires gap**: `00-as-is.md` gap number(s).

If any of these four fields is vague, the task is not ready to start.

---

## Milestone 1 — Typed core + honest rendering

**Ship this**: the agent produces a multi-page PDF with real typography,
a bundled theme, no truncated lines, no silent content loss, artifacts
under the Tauri app-data tree, and a background render that can be
cancelled.

**8 tasks, in order.** Later tasks depend on earlier ones.

### T1.1 — Typed domain model in `core/documents/pdf/`

- **Deliverable**: a pure, `serde`-able domain model callers can build
  without touching `printpdf`.
- **Build**: `PdfDocument`, `Metadata`, `PageGeometry`, `Margins`,
  `Section`, `Heading`, `Paragraph`, `InlineRun`, `InlineStyle`,
  `Align`, `Block` enum (start with `Paragraph`, `Heading`, `PageBreak`
  variants only — other blocks come in M2).
- **Tests (first)**:
  - serde round-trip for each top-level struct
  - `Metadata` rejects empty title
  - `Paragraph` preserves run order
  - `Block::PageBreak` serializes / deserializes correctly
- **Done when**: `cargo test -p coder_core` green AND `core/documents/pdf/`
  has zero `use printpdf::` imports AND every file ≤ 500 LoC.
- **Retires gap**: #1.

### T1.2 — `PdfRenderer` trait + minimal `PrintPdfRenderer`

- **Deliverable**: a renderer that produces a one-page PDF from the
  typed model with **real glyph metrics**.
- **Build**:
  - `PdfRenderer` async trait in `core/documents/pdf/render.rs`
  - `PrintPdfRenderer` impl in
    `infrastructure/tools/documents/pdf/render/printpdf_renderer.rs`
  - font metrics loaded from the TTF / OTF file, not from a constant
  - `lopdf` added as a **dev-dependency** only for test parsing
- **Tests (first)**:
  - render fixture → `lopdf` parse → assert `%PDF-1.`, `%%EOF`,
    `/Pages/Count == 1`, `/Title` present
  - line wrapping test: a known string at a known width produces a
    known line count (NOT the character-count heuristic)
- **Done when**: `AVG_CHAR_WIDTH_PT` constant is deleted from the tree.
- **Retires gap**: #6 (partially; completed by T1.6).

### T1.3 — Wire tool adapters through a new application service, delete legacy render

- **Deliverable**: the existing `pdf.start → pdf.add_page →
  pdf.finalize` surface still works end-to-end, but drives the new
  renderer. `title` is no longer dropped.
- **Build**:
  - `application/tools/pdf/` with `StartDocument`, `AppendBlock`,
    `Finalize` use cases
  - adapter that maps legacy `PdfPageContent::{Text, Markdown}` to
    `Paragraph` / `Heading`
  - **delete** `infrastructure/tools/documents/pdf/final.rs`'s
    `render_page_content`, `wrap_text`, `html_to_plain_text`, and
    `get_page_dimensions` — all superseded
- **Tests (first)**:
  - characterization test: the current `pdf.start/add_page/finalize`
    smoke test still passes, now asserting on the new renderer's output
  - `pdf.finalize` output metadata `/Title` matches the `title` passed
    to `pdf.start`
- **Done when**: `final.rs` is ≤ 200 LoC and contains only the tool
  struct + `Tool` impl AND the legacy path is gone AND `pdf.add_page`
  maps losslessly to the new model.
- **Retires gap**: #2, #4.

### T1.4 — `Theme` TOML loader + `report` theme, zero magic numbers

- **Deliverable**: every margin, font size, color, and leading currently
  hardcoded in the render path is sourced from a bundled theme file.
- **Build**:
  - `core/documents/pdf/theme.rs`: `Theme`, `TypographyScale`,
    `Palette`, `Spacing`, `PageDefaults`
  - `src-tauri/resources/pdf/themes/report.toml`
  - loader in `infrastructure/.../pdf/render/theme_loader.rs`
- **Tests (first)**:
  - parse `report.toml` into typed `Theme`
  - two renders of the same document with different theme values
    produce measurably different baselines (parsed via `lopdf`)
  - **no-magic-numbers sweep**: a unit test that
    `include_str!("printpdf_renderer.rs")` and greps for `Mm(` /
    `f32` literals inside function bodies, fails on any match
- **Done when**: the sweep test is green AND `cargo clippy` is clean.
- **Retires gap**: #3.

### T1.5 — Custom font embedding with fallback chain

- **Deliverable**: documents can use a real font family (not just
  `Helvetica`), with fallback when a glyph is missing.
- **Build**:
  - `FontCache` in `render/font_cache.rs` loading TTF/OTF from paths
    listed in the theme
  - primary + fallback chain with `MissingGlyph` warning emission
  - `InlineRun` variants (bold, italic, code) map to loaded families
- **Tests (first)**:
  - glyph absent in primary → picked up from fallback
  - glyph absent in both → `MissingGlyph` warning in `RenderOutcome`,
    no panic, document still renders
  - bold and italic runs use the loaded variant (parse PDF font dict)
- **Done when**: Helvetica is no longer the only font in the tree.
- **Retires gap**: #5.

### T1.6 — Automatic pagination, zero silent truncation

- **Deliverable**: content longer than one page flows to new pages
  at block boundaries. Nothing is ever silently dropped.
- **Build**: layout engine in `render/layout.rs` that tracks usable
  height, creates a new page when a block does not fit, splits
  paragraphs at line boundaries (never mid-word), honors
  `Block::PageBreak`.
- **Tests (first)**:
  - a 50-paragraph fixture renders into the expected page count,
    every paragraph appears in the output (parse and grep)
  - `Block::PageBreak` forces a new page even mid-section
  - adversarial test: a single paragraph longer than one page wraps
    across pages, every word accounted for
- **Done when**: a search for "silent truncation" style `break;` in
  the render path returns nothing AND the 50-paragraph fixture test
  is green.
- **Retires gap**: #6 (completes), #7.

### T1.7 — Artifacts under app-data, real `session_id`

- **Deliverable**: finalized PDFs live under
  `AppHandle::path().app_local_data_dir()`; `ArtifactCreated` events
  carry the real session id.
- **Build**:
  - path resolution through `AppHandle`
  - `ToolExecutionContext` threads `session_id` into the finalize use
    case
  - one-time migration: on startup, move existing files from
    `~/.coder/artifacts/` to the new location
- **Tests (first)**:
  - integration test asserts the output path resolves under
    `app_local_data_dir()`
  - event emission test asserts `session_id` equals the caller value,
    not the literal `"current"`
  - migration test: legacy files are moved exactly once, idempotent on
    second run, noop when source is empty
- **Done when**: `env::var("HOME")` is gone from the render path AND
  the `"current"` string is gone from `final.rs`.
- **Retires gap**: #14, #15.

### T1.8 — Background render + cancellation

- **Deliverable**: finalize runs on a dedicated task; the user can
  cancel from the `DocumentGenerationCard` and no partial file is left
  behind.
- **Build**:
  - render via `tokio::task::spawn_blocking` wrapped in a
    `CancellationToken`
  - atomic write pattern: render to `<name>.pdf.tmp`, rename on success,
    delete on cancel
- **Tests (first)**:
  - cancel mid-render deletes the `.tmp` file, leaves no `.pdf`
  - cancel budget: cancellation completes within a bounded time
  - happy path: rename happens atomically on success
- **Done when**: a stress test (10 concurrent finalize + cancel pairs)
  leaves zero stray files.
- **Retires gap**: #16.

### M1 exit gate

M1 is Done when **all** of the following are true:

- T1.1–T1.8 are merged.
- A 4-page reference document
  (`docs/tools/pdf/goldens/m1-reference.pdf`) is committed alongside
  its SHA-256. The test harness re-renders and compares on every run.
- `cargo test` and `cargo clippy` are green.
- `00-as-is.md` gaps #1, #2, #3, #4, #5, #6, #7, #14, #15, #16 are
  ticked off in the gap tracker in this file (see end).
- No task introduced a hardcoded constant the sweep test in T1.4 would
  miss (manual review).

If any of those is false, M1 is not Done. No M2 work starts.

---

## Milestone 2 — Structural documents

**Ship this**: the agent can author Journey A from `01-vision.md`
end-to-end: cover, TOC, headers/footers, tables, images, code,
hyperlinks. A real report.

**7 tasks. Each depends on M1.** Order is deliberate.

### T2.1 — Section tree → PDF outline (bookmarks)

- **Deliverable**: H1/H2/H3 headings produce a `/Outlines` tree in the
  PDF matching input depth.
- **Build**: outline collection during layout, written once on finalize.
- **Tests (first)**:
  - 3-level fixture → parsed outline matches input tree
  - first-heading-on-page test: outline entry points at the right page
- **Done when**: clicking a heading in Preview / Acrobat jumps to the
  correct page (manual validation on the fixture).
- **Retires gap**: part of #12.

### T2.2 — Headers, footers, page numbers (from theme tokens)

- **Deliverable**: every page except the cover shows a header/footer
  built from theme template tokens `{title}`, `{page}`, `{total_pages}`,
  `{section}`.
- **Build**: `PageDecorator` in `render/decorator.rs`; template
  expansion at write time (two-pass so `{total_pages}` is resolved).
- **Tests (first)**:
  - 3-page doc: parsed text of each page contains the expected header
    and footer
  - first-page suppression toggle
  - `{total_pages}` resolves correctly (not `{total_pages}` literal)
- **Done when**: `report.toml` exposes the template strings; no strings
  are hardcoded in the decorator.
- **Retires gap**: part of #11.

### T2.3 — List, code, callout blocks

- **Deliverable**: agent can emit ordered/unordered (nested) lists, code
  blocks with a monospace font, and callout boxes, all styled from the
  theme.
- **Build**: `ListBlock`, `CodeBlock`, `Callout` domain variants plus
  renderer support. Callout colors reference the theme palette, not new
  design tokens.
- **Tests (first)**:
  - nested list renders at correct indents per level
  - code block uses mono font from the theme (parse font dict)
  - callout box honors theme palette; no hex colors in code
- **Done when**: one fixture per block type exists under
  `tests/fixtures/pdf/` and each has a golden hash.

### T2.4 — Tables with row-boundary splitting

- **Deliverable**: `TableBlock` with fixed and `auto` columns; header
  row repeats when the table crosses a page boundary.
- **Build**: table layout in `render/table.rs`.
- **Tests (first)**:
  - column widths honored
  - oversized table splits at a row boundary, header repeats on the
    second page
  - empty table is a no-op, no panic
- **Done when**: a 20-row table across 2 pages renders deterministically.

### T2.5 — Real image embedding

- **Deliverable**: `ImageBlock` embeds the real image bytes. No
  placeholder text.
- **Build**: image decode via the `image` crate (confirm it's already
  in the tree; if not, add it and justify per
  `docs/quality/guidelines/configuration.md`).
- **Tests (first)**:
  - rendered PDF has an `XObject` whose pixel dimensions match the
    source
  - grep the rendered bytes — the literal string `"[Image:"` must not
    appear anywhere
  - missing image file → typed error, not panic
- **Done when**: the grep test is green.
- **Retires gap**: #10.

### T2.6 — TOC block + internal and external hyperlinks

- **Deliverable**: `pdf.add_toc` inserts a TOC whose page numbers are
  correct after pagination converges. `InlineRun::link` produces link
  annotations.
- **Build**:
  - two-pass layout: render, collect heading positions, patch TOC, emit
  - link annotations for both internal (heading anchor) and external
    (`Url`) targets
- **Tests (first)**:
  - TOC entries point at the correct real page (not `N+1` because TOC
    insertion shifted pagination)
  - internal link target resolves to the heading's page
  - external link has the expected URI action
- **Done when**: clicking a TOC entry and clicking an external link
  both work in a real viewer on the fixture.
- **Retires gap**: #11 (completes), #12 (completes).

### T2.7 — Structural `PdfValidator`

- **Deliverable**: validator parses the output PDF and asserts on the
  object graph instead of substring-searching the bytes.
- **Build**: replace `check_eof_marker` / `check_encryption` with
  `lopdf`-based checks. Decide whether `lopdf` graduates from
  dev-dependency to production dependency and record the decision in
  `research/libraries.md`.
- **Tests (first)**:
  - encrypted test fixture → validator reports encryption via the
    parsed `/Encrypt` dict, not a string match
  - file containing the literal text `"/Encrypt"` in a paragraph does
    not false-positive
  - missing `/Pages` → hard failure with typed error
- **Done when**: no substring PDF inspection remains in the validator.
- **Retires gap**: #18.

### M2 exit gate

M2 is Done when:

- T2.1–T2.7 are merged.
- The **Journey A golden** is committed:
  `docs/tools/pdf/goldens/journey-a.pdf` — a report with cover,
  metadata, TOC, headers/footers, 2 images, 1 table, nested lists, one
  internal link, one external link. Hash locked in the test suite.
- `PdfValidator::validate_file` returns `passed: true` with **zero
  warnings** on that golden.
- `00-as-is.md` gaps #8, #9, #10, #11, #12, #18 are ticked off.

Gaps #8 and #9 are retired via **deletion**, not via improvement: the
HTML and Markdown content blobs from the old model are removed in
T1.3 and never come back. `04-roadmap.md` used to explain this; the
rationale is preserved at the end of this document.

---

## Milestone 3 — UX integration

**Ship this**: the user perceives the PDF tool as first-class. No new
design-system tokens.

**4 tasks. Each depends on M2.**

### T3.1 — Structured progress events end-to-end

- **Deliverable**: the frontend receives typed progress events and maps
  them onto the existing `DocumentGenerationCard` phases.
- **Build**:
  - new `AgentEvent::Pdf*` variants: `PdfPhaseChanged`, `PdfProgress`,
    `PdfWarning`
  - emission from the background render job in T1.8
  - `documentGenerationTracker.ts` maps them to card state
- **Tests (first)**:
  - backend: events arrive in declared order
  - backend: progress is monotonic
  - frontend: tracker unit tests cover each new event type
- **Done when**: the card shows an honest page count and phase label
  during a real render.
- **Retires gap**: #19.

### T3.2 — Validation report surfaced in the card

- **Deliverable**: if `PdfValidator` emits warnings, the card shows a
  "view details" affordance using existing design tokens.
- **Build**: pass warnings through the event stream; reuse the expand /
  details mechanism already on the card.
- **Tests (first)**:
  - Svelte test: warnings array → "details" toggle renders
  - `css-integrity.test.ts` continues green (zero new tokens)
- **Done when**: no new colors, shadows, radii, or fonts introduced.

### T3.3 — Settings → Tools → PDF theme picker

- **Deliverable**: the user picks a default theme; the next
  `pdf.finalize` uses it automatically.
- **Build**:
  - Tauri command to list installed themes
  - settings persisted in `coder_config.db` via an additive
    `CREATE TABLE IF NOT EXISTS` migration (same pattern as
    `web_search`)
  - Svelte modal follows baseline settings patterns
- **Tests (first)**:
  - DB migration idempotency test
  - end-to-end: pick theme → finalize → rendered PDF matches expected
    theme
- **Done when**: the modal ships, persists, and survives app restart.
- **Retires gap**: #20.

### T3.4 — Documentation close-out

- **Deliverable**: the planning docs in `docs/tools/pdf/` are marked
  `COMPLETE`, the changelog reflects the shipped feature, and
  `00-as-is.md` is replaced by a short "shipped" note pointing at the
  code.
- **Build**: doc updates only, no code.
- **Done when**: every file in `docs/tools/pdf/` has an accurate status
  header per `RULES.md § Documentation Governance` AND `CHANGELOG.md`
  has a one-line entry.

### M3 exit gate

M3 is Done when **all seven success criteria in
`01-vision.md § Success criteria` are true**, and every gap below is
closed.

---

## Gap retirement tracker

Single source of truth for which task closes which gap from
`00-as-is.md`. Update as tasks merge.

| Gap | # | Task | Status |
|---|---|---|---|
| Coarse content blobs | 1 | T1.1 | pending |
| `title` dropped | 2 | T1.3 | done |
| No document theme | 3 | T1.4 | done |
| `position` unused | 4 | T1.3 | done |
| Only builtin fonts | 5 | T1.5 | pending |
| Character-count wrapping | 6 | T1.2 + T1.6 | pending |
| Silent truncation | 7 | T1.6 | pending |
| Lossy HTML | 8 | T1.3 (via deletion) | done |
| Primitive Markdown | 9 | T1.3 (via deletion) | done |
| Image placeholder | 10 | T2.5 | pending |
| No headers/footers/TOC | 11 | T2.2 + T2.6 | pending |
| No hyperlinks/bookmarks/metadata | 12 | T2.1 + T2.6 | pending |
| Non-persistent drafts | 13 | **deferred** | not scheduled |
| Artifacts outside app-data | 14 | T1.7 | pending |
| Hardcoded session_id | 15 | T1.7 | pending |
| Blocking render | 16 | T1.8 | pending |
| No golden-file tests | 17 | M1 exit gate | pending |
| Substring validator | 18 | T2.7 | pending |
| Card lacks structural facts | 19 | T3.1 | pending |
| No theme settings UI | 20 | T3.3 | pending |

Gap #13 is the only one deferred. It is only promoted if a user report
shows draft loss is a real problem.

---

## Why HTML and Markdown are retired, not improved

`PdfPageContent::Html` and `PdfPageContent::Markdown` exist today as
string blobs the renderer tries to interpret at finalize time. The
typed model from T1.1 already has first-class headings, paragraphs,
runs, lists, tables, code, and images. Asking the agent to emit those
typed blocks directly is **less lossy, faster to test, and a clearer
contract** than asking it to emit a string of Markdown and then
re-parsing it inside the renderer.

If a caller ever needs Markdown → typed model, that conversion lives
in a small, testable utility in the **application** layer — never
inside the renderer.

---

## Reading order for an implementer

1. This file (`03-execution.md`).
2. `00-as-is.md` — gaps referenced everywhere.
3. `02-architecture.md` — layer boundaries T1.1/T1.2 must respect.
4. `01-vision.md § Success criteria` — M3 exit gate.
5. `research/libraries.md` — only if a task changes the engine
   dependency footprint (T2.5 image crate, T2.7 lopdf graduation).

Everything else is optional background.
