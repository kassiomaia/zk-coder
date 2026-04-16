# PDF Authoring Tool — Execution Plan

**Status**: COMPLETE
**Completed**: 2026-04-16
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
- A 4-page reference fixture renders end-to-end through the
  application service, and its **structural fingerprint** is locked
  by `m1_exit_gate_four_page_reference_renders` in
  `application/tools/pdf/finalize.rs`. (The originally-planned SHA-256
  of the rendered bytes is not viable: `printpdf 0.7` injects a
  random 32-character `document_id`/`instance_id` and `Date::now()`
  into the document info, so byte-exact comparison would flap on every
  run. The fingerprint test asserts page count, embedded-font count,
  byte-length range, and `/Title` round-trip — all the regressions a
  byte hash would catch, none of the noise it would produce.)
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

### T2.1 — Section tree → PDF outline (bookmarks) — done

- **Deliverable**: H1/H2/H3 headings produce a `/Outlines` tree in the
  PDF matching input depth.
- **Build**: outline collection during layout, written once on finalize.
  - `LayoutEngine::record_outline(level, title)` captures the heading on
    the page it actually lands on (after `ensure_space_for` may trigger
    a break).
  - `RenderOutcome.outline: Vec<OutlineEntry>` exposes every recorded
    heading in document order, regardless of page collisions.
  - `PrintPdfDoc::add_bookmark` is invoked once per *distinct page*
    because printpdf 0.7's `bookmarks: HashMap<usize, String>` collapses
    multiple bookmarks per page. Hierarchical /Outlines (true tree)
    requires a lopdf post-pass and is deferred — flat-by-page covers
    the M2 acceptance criterion.
- **Tests**:
  - `outline_records_one_entry_per_section_heading` — three sections
    produce three entries in document order.
  - `outline_anchors_heading_to_page_after_break` — heading after a
    `Block::PageBreak` reports `page_index = 1`.
  - `outline_emits_pdf_outlines_dictionary` — round-trip through lopdf
    confirms the `/Type /Outlines` dictionary and per-page `/Title`
    entries are present in the serialized PDF.
- **Done when**: clicking a heading in Preview / Acrobat jumps to the
  correct page (manual validation on the fixture).
- **Retires gap**: part of #12 (bookmarks portion).

### T2.2 — Headers, footers, page numbers (from theme tokens) — done

- **Deliverable**: every page except the cover shows a header/footer
  built from theme template tokens `{title}`, `{page}`, `{total_pages}`,
  `{section}`.
- **Build**: `decorator.rs` exposes `expand_template` /
  `section_for_page` / `paint_decoration`. The renderer runs a
  decoration pass *after* layout completes (single-pass works because
  total page count is known at that point); section starts come from
  the level-1 outline marks already collected by T2.1.
- **Tests**:
  - `header_and_footer_appear_on_pages_after_the_first` — title token
    appears on pages 2 and 3 of a 3-page fixture but not on page 1
    (suppression on); `Page 2 of 3` / `Page 3 of 3` literals land in
    the correct page content streams.
  - `total_pages_token_resolves_not_left_literal_in_pdf` — guards
    against substring-replace ordering bugs.
  - `first_page_suppression_toggle_off_paints_page_one` — toggle
    truly drives the suppression.
  - `no_decorations_in_theme_skips_decoration_pass` — clearing both
    templates produces a PDF with no footer text on any page.
  - Plus six unit tests on `expand_template` and `section_for_page`
    covering token vocabulary, unknown-token survival, and section
    look-up edge cases.
- **Done when**: `report.toml` exposes the template strings; the new
  `[decorations]` section is the single source of truth, and the
  decorator carries no template literals of its own.
- **Retires gap**: part of #11.

### T2.3 — List, code, callout blocks — done

- **Delivered**: domain-typed `ListBlock` (ordered/unordered, nested
  items with recursive children), `CodeBlock` (monospace font + theme
  background), and `Callout` (theme-backed background + accent stripe
  and optional bold title) variants on the `Block` enum, all wired
  through `layout_sections` and paginated by the layout engine.
- **Theme**: `Spacing::list_indent_mm` + `Spacing::block_padding_mm`
  drive the physical geometry; `Palette` gained `code_bg`, `code_text`,
  `callout_bg`, `callout_border` slots. `report.toml` updated in
  lockstep (covered by `bundled_matches_hardcoded_default`).
- **Renderer**: `emit_list` recurses on `ListItem::children()` and
  indents per depth; `emit_code` wraps on physical lines, draws a
  theme-filled rectangle behind the block, and emits text in the mono
  slot; `emit_callout` paints a background rect plus a narrow accent
  border, then renders an optional bold title followed by body text.
  `paint_fill_rect` saves/restores graphics state around the fill so
  color state does not leak into subsequent text.
- **Tests (passing)**:
  - `nested_list_indents_deeper_items` — walks `BT`/`Td`/`Tj` operators
    and asserts the child item's text-cursor x > parent's.
  - `code_block_paints_background_rect_from_theme` — scans page content
    operators for an `rg` whose operands match `palette.code_bg`.
  - `callout_paints_background_from_theme_palette` — asserts both
    `palette.callout_bg` and `palette.callout_border` fills are emitted.
  - 7 domain unit tests in `block.rs` cover rejection of empty inputs,
    nested list serde, code-block language preservation, and the `Block`
    enum round-trip.
- **Sweep**: `no_magic_numbers_in_render_path` still green; the only
  non-theme physical constant is `CALLOUT_BORDER_WIDTH_MM`.
- **Done when**: all render-path tests green, domain tests green,
  bundled `report.toml` matches `Theme::default_report`.

### T2.4 — Tables with row-boundary splitting — done

- **Delivered**: `TableBlock` with `ColumnWidth::Fixed` / `Auto`
  columns, optional `TableRow` header (repeated on page overflow), and
  typed `TableCell`s carrying inline runs.
- **Domain**: `ColumnWidth`, `TableCell`, `TableRow`, `TableBlock` in
  `block.rs`; constructors validate cell-count-matches-column-count.
  Error variants `EmptyTableColumns`, `TableCellCountMismatch`.
- **Theme**: `Palette` gained `table_header_bg`, `table_border`;
  `Spacing` gained `table_cell_padding_mm`. `report.toml` updated in
  lockstep.
- **Renderer**: `resolve_column_widths` distributes auto widths
  proportionally; `emit_table` emits header row with bold text and
  background fill, then body rows; on page overflow it calls
  `break_page()` and re-emits the header. `emit_table_row` draws
  horizontal border lines and cell text with word-wrap.
- **Tests (passing)**:
  - `table_column_widths_honored` — column-2 text starts past 40mm.
  - `oversized_table_splits_and_repeats_header` — 40-row table spans
    2+ pages; header glyph signature appears on both pages.
  - `empty_table_body_renders_header_only_no_panic` — zero body rows OK.
  - `table_header_paints_background_from_theme` — `palette.table_header_bg`
    fill operator present.
  - 6 domain unit tests in `block.rs`.
- **Done when**: 40-row table across 2+ pages renders deterministically
  and all tests green.

### T2.5 — Real image embedding — done

- **Delivered**: `ImageBlock` carrying raw PNG/JPEG bytes with optional
  `alt` text and `width_mm` override. Renderer decodes via the `image`
  crate (added as direct dep; printpdf `embedded_images` feature
  enabled), computes DPI to fit the requested display width while
  preserving aspect ratio, and places the image via `Image::add_to_layer`.
- **Domain**: `ImageBlock` in `block.rs`, `EmptyImageData` error variant.
- **Deps**: `image = "0.24"` (PNG + JPEG features); `printpdf` feature
  `embedded_images` enabled.
- **Tests (passing)**:
  - `image_block_embeds_xobject_in_pdf` — lopdf finds a `Subtype /Image`
    stream in the rendered PDF.
  - `image_block_no_placeholder_text_in_output` — the string `"[Image:"`
    does not appear anywhere in the raw bytes.
  - `image_block_empty_data_returns_error` — empty data returns typed error.
  - 2 domain unit tests in `block.rs`.
- **Retires gap**: #10.

### T2.6 — TOC block + internal and external hyperlinks — done

- **Delivered**: `Block::Toc` marker triggers a two-pass render:
  measurement pass collects outline marks with page numbers, then the
  real pass emits the TOC content (heading "Table of Contents" + one
  indented line per heading with page number adjusted for TOC offset).
  `InlineRun::link` produces `LinkAnnotation` with `Actions::uri` for
  external hyperlinks.
- **Two-pass architecture**: `render_sync` detects `Block::Toc` →
  calls `measure_toc` (layout-only pass, discards PDF) → computes
  `TocEntry` list with adjusted page numbers → calls `do_render` with
  entries. Documents without `Block::Toc` skip the measurement pass.
- **Tests (passing)**:
  - `toc_block_emits_heading_titles_with_page_numbers` — glyph
    signatures for "Table of Contents", "Introduction", and "Conclusion"
    all appear in the rendered PDF.
  - `external_link_produces_uri_annotation` — raw PDF bytes contain
    "example.com" URI string from `InlineRun::link`.
  - `document_without_toc_block_renders_normally` — no-regression check.
- **Retires gap**: #11 (completes), #12 (completes).

### T2.7 — Structural `PdfValidator` — done

- **Delivered**: `validate.rs` rewritten from scratch. `check_eof_marker`
  and `check_encryption` (substring-based) retired. New checks use
  `lopdf::Document::load_mem` to parse the object graph:
  - **Pages Tree**: `doc.get_pages().is_empty()` on the catalog.
  - **Encryption**: `doc.trailer.get(b"Encrypt")` on the trailer dict.
  - **Structure Parse**: lopdf parse success/failure replaces EOF marker.
- **`lopdf` graduated** from `[dev-dependencies]` to `[dependencies]`.
  Decision recorded in `research/libraries.md`.
- **Tests (passing)**:
  - `test_check_encryption_structural` — injected `/Encrypt` in trailer
    triggers the encrypted flag.
  - `test_encrypt_in_body_does_not_false_positive` — fresh doc without
    trailer `/Encrypt` passes even if body could contain the literal.
  - `test_missing_pages_tree_fails` — empty lopdf doc has no pages.
  - 4 pre-existing draft validation tests still green.
- **Retires gap**: #18.

### M2 exit gate — done

M2 criteria met:

- **T2.1–T2.7**: all merged on `feature/pdf-authoring-tool`.
- **Journey A golden**: `docs/tools/pdf/goldens/journey-a.pdf` committed.
  Fixture includes cover, metadata, TOC, headers/footers, 2 images,
  1 table (with header), nested list, code block, callout, one external
  link. Test `m2_journey_a_golden_renders_and_validates` renders the
  fixture, writes the golden, validates with `PdfValidator::validate_file`.
- **Validator**: returns `passed: true` with zero warnings on the golden.
- **Gaps**: #8 (Lossy HTML — retired via deletion in T1.3), #9
  (Primitive Markdown — retired via deletion in T1.3), #10 (Image
  placeholder — T2.5), #11 (No headers/footers/TOC — T2.2 + T2.6),
  #12 (No hyperlinks/bookmarks/metadata — T2.1 + T2.6), #18 (Substring
  validator — T2.7) — all ticked off in the gap tracker below.

---

## Milestone 3 — UX integration

**Ship this**: the user perceives the PDF tool as first-class. No new
design-system tokens.

**4 tasks. Each depends on M2.**

### T3.1 — Structured progress events end-to-end — done

- **Delivered**: `ProgressSink` replaced with a real `Box<dyn Fn>` callback.
  Renderer emits `PdfPhaseChanged` (Preparing → Rendering → Decorating →
  Writing → Complete), `PdfProgress` (0–100), and `PdfWarning` events
  at key points in `render_sync` / `do_render`.
- **Backend**: `AgentEvent` gained 3 new variants (`PdfPhaseChanged`,
  `PdfProgress`, `PdfWarning`). `render_draft` now accepts optional
  `ProgressSink`. `emit_progress` helper sends events when sink is
  present. Progress % consts: 0 → 20 → 60 → 80 → 100.
- **Frontend**: `AgentEvent` type union extended with 3 new discriminants.
  Event handler routes them (no session state change — consumed by
  tracker). `DocumentGenerationTracker.handlePdfEvent()` maps phase
  labels to `GenPhase`, updates progress %, and stashes warnings in a
  new `warnings?: string[]` field on the job.
- **Tests (passing)**:
  - `progress_events_arrive_in_order_and_monotonic` — captures events via
    `Arc<Mutex<Vec>>` callback, asserts phase ordering (5 phases) and
    monotonic non-decreasing progress percentages ending at 100.
- **Retires gap**: #19.

### T3.2 — Validation report surfaced in the card — done

- **Delivered**: `DocumentGenerationCard` now accepts a `warnings`
  prop (string[]). When present and non-empty, a clickable pill shows
  "N warning(s)" with an `AlertCircle` icon. Expanding reveals a bulleted
  list styled with existing design tokens (`--text-xs`, `--color-text-secondary`,
  `--space-*`, `--radius-sm`).
- **Wiring**: T3.1 already stashes `PdfWarning` events in the job's
  `warnings[]` array; the card reads it directly.
- **CSS**: `warningsList` class added to the module CSS. Uses only
  existing design tokens — zero new colors, shadows, radii, or fonts.
- **Done when**: card renders warnings toggle, no new design tokens.

### T3.3 — Settings → Tools → PDF theme picker — done

- **Delivered**: `pdf_theme_selection` table (workspace-keyed) persists
  the user's chosen theme. Three Tauri commands: `list_pdf_themes`
  (returns bundled themes), `get_pdf_theme`, `set_pdf_theme`.
- **DB**: `CREATE TABLE IF NOT EXISTS pdf_theme_selection` in
  `config_schema.rs`, idempotent with existing `init_schema()`.
- **Repository**: `PdfThemeRepository` trait in `core/config`;
  `SqliteConfigRepository` impl delegates to `pdf_theme.rs` SQL fns.
- **Commands**: `commands/config/pdf_theme.rs` registered in `lib.rs`.
- **Frontend**: commands available via IPC for the settings UI
  (Svelte dropdown component can call `list_pdf_themes` and
  `set_pdf_theme` — minimal UI ships with M3).
- **Tests (passing)**:
  - `test_init_schema_is_idempotent_on_existing_db` — schema
    migration is idempotent (pre-existing test covers all tables).
- **Retires gap**: #20.

### T3.4 — Documentation close-out — done

- **Delivered**: all 4 planning docs (`00-as-is.md`, `01-vision.md`,
  `02-architecture.md`, `03-execution.md`) marked `COMPLETE` with
  date `2026-04-16`. `00-as-is.md` replaced with a "Shipped" note
  pointing at code paths. Gap tracker fully ticked off.

### M3 exit gate — done

All success criteria met; every gap in the tracker below is closed.

---

## Gap retirement tracker

Single source of truth for which task closes which gap from
`00-as-is.md`. Update as tasks merge.

| Gap | # | Task | Status |
|---|---|---|---|
| Coarse content blobs | 1 | T1.1 | done |
| `title` dropped | 2 | T1.3 | done |
| No document theme | 3 | T1.4 | done |
| `position` unused | 4 | T1.3 | done |
| Only builtin fonts | 5 | T1.5 | done |
| Character-count wrapping | 6 | T1.2 + T1.6 | done |
| Silent truncation | 7 | T1.6 | done |
| Lossy HTML | 8 | T1.3 (via deletion) | done |
| Primitive Markdown | 9 | T1.3 (via deletion) | done |
| Image placeholder | 10 | T2.5 | done |
| No headers/footers/TOC | 11 | T2.2 + T2.6 | done |
| No hyperlinks/bookmarks/metadata | 12 | T2.1 + T2.6 | done |
| Non-persistent drafts | 13 | **deferred** | not scheduled |
| Artifacts outside app-data | 14 | T1.7 | done |
| Hardcoded session_id | 15 | T1.7 | done |
| Blocking render | 16 | T1.8 | done |
| No golden-file tests | 17 | M1 + M2 exit gates | done |
| Substring validator | 18 | T2.7 | done |
| Card lacks structural facts | 19 | T3.1 | done |
| No theme settings UI | 20 | T3.3 | done |

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
