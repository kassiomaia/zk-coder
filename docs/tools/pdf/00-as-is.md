# PDF Authoring Tool — As-Is Audit

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Ground-truth inventory of what the PDF tool surface, runtime,
and UI look like *today*, before any design change. No proposals — those
live in `01-vision.md` and `02-architecture.md`.

---

## Scope of audit

Only paths actually shipped on `main` as of this review:

- `src-tauri/src/infrastructure/tools/documents/pdf/` — the tool surface
- `src-tauri/src/infrastructure/tools/documents/shared/` — draft store,
  artifact store, validation helpers
- `src-ui/src/lib/components/chat/messages/DocumentGenerationCard.svelte`
  and `.module.css` — the UI the user sees during and after generation
- `src-ui/src/lib/utils/documentGenerationTracker.ts`
- `src-tauri/Cargo.toml` — the `printpdf = "0.7"` dependency

---

## Tool surface (Rust side)

Re-exported from `pdf/mod.rs`:

| Tool ID | Struct | What it does today |
|---|---|---|
| `pdf.start` | `PdfStartTool` | Creates an in-memory draft in the global `DraftStore`. Accepts `filename`, `title`, optional `page_size` ∈ {A4, Letter, Legal, A3}. Returns a `pdf_id` and a placeholder status. **Note:** the `title` argument is captured in the output payload but is **not** stored on `PdfDraft`, so it is lost before `finalize` runs. |
| `pdf.add_page` | `PdfAddPageTool` | Appends a page to a draft. Content variant is one of `markdown`, `html`, `text`, `image`. The `position` field advertises `start`/`end` but is unused — all pages are appended. |
| `pdf.delete_page` | `PdfDeletePageTool` | Removes a page by index/id. |
| `pdf.list_pages` | `PdfListPagesTool` | Returns a summary or full content listing for the draft. |
| `pdf.get_page` | `PdfGetPageTool` | Returns a specific page by 1-based number or `page_id`. |
| `pdf.finalize` | `PdfFinalizeTool` | Drives the actual PDF write via `printpdf`, saves to `~/.coder/artifacts/<filename>`, and emits an `AgentEvent::ArtifactCreated` through Tauri. |
| (internal) | `PdfValidator` | Pre- and post-finalization sanity checks (has pages, content not empty, `%PDF` header, `%%EOF` marker, encryption sniff). |

All tools execute through the shared `Tool` trait and are registered via
`PdfCategory`. None of them touch the workspace root.

## Draft model

`src-tauri/src/infrastructure/tools/documents/shared/draft_store.rs` is the
single source of state:

- `DraftStore` is a `OnceLock<Arc<Mutex<_>>>` singleton.
- `PdfDraft` holds: `pdf_id`, `filename`, `pages: Vec<DraftPage>`,
  `page_size: PdfPageSize`, `status: DraftStatus`, `created_at`, `updated_at`.
- `DraftPage` holds `id`, `index`, and one `PdfPageContent` variant.
- `PdfPageContent` variants:
  - `Markdown { content }`
  - `Html { content, styles }`
  - `Text { content }`
  - `Image { path, caption }`

The draft store is process-global and has no persistence. A restart loses
every draft in flight. `cleanup_expired_drafts` exists but is not wired to
any scheduler.

## Rendering pipeline

`pdf/final.rs::generate_pdf` is where the actual bytes are produced:

1. Resolves `~/.coder/artifacts/` as the output directory (via `HOME` /
   `USERPROFILE`). Not using `AppHandle::path().app_data_dir()` — the
   location is outside Tauri’s app-data tree.
2. Maps `PdfPageSize` to millimetre dimensions via `get_page_dimensions`.
3. Handles the empty-draft case by producing a one-page blank document.
4. Creates a `PdfDocument`, registers `BuiltinFont::Helvetica` and
   `BuiltinFont::HelveticaBold` once, then iterates pages and calls
   `render_page_content` for each.
5. `render_page_content` hardcodes:
   - margins: `20 mm` on every side
   - title font size: `18 pt`
   - body font size: `12 pt`
   - line height: `5 pt` added to font size
6. Per content variant:
   - **Text** — calls `wrap_text` (character-count heuristic, not font
     metrics) and emits lines until `current_y < margin_bottom`, at which
     point trailing content is silently dropped.
   - **Markdown** — handles `# `, `## `, `### `, and plain paragraphs. No
     bold, italics, inline code, lists, blockquotes, code blocks, or tables.
     Same silent overflow behavior.
   - **Html** — strips a hand-coded list of tags and renders the result as
     plain text. CSS from the `styles` field is ignored.
   - **Image** — writes a literal `[Image: <path>]` placeholder plus a note
     stating "Image embedding will be implemented in a future update". The
     image file is never opened.

Line wrapping uses `AVG_CHAR_WIDTH_PT = 0.5` as a constant, independent of
the actual font — lines are therefore either under- or over-filled depending
on the string content.

The render function is approaching the soft file-size limit from
`docs/quality/guidelines/rust-backend.md` (final.rs is ~570 lines and owns
multiple concerns).

## Artifact emission

`PdfFinalizeTool` emits `AgentEvent::ArtifactCreated` with a hardcoded
`session_id: "current"` (explicit `TODO` in the source). The frontend listens
and morphs the `DocumentGenerationCard` into a download-ready state.

## Validator

`PdfValidator` runs two distinct checks:

- **Pre-finalize** on `PdfDraft`: has pages, page content non-empty, page
  count reporting.
- **Post-finalize** on the file: file exists, size > 100 B, `%PDF` header,
  `%%EOF` trailer, encryption sniff by scanning for the literal substring
  `/Encrypt`, and a size-based page-count estimate.

`%%EOF` and `/Encrypt` checks run on raw bytes without parsing, so they are
best-effort. The encryption check is string-matching against the whole file.

## Frontend integration

`DocumentGenerationCard.svelte` already understands the `Pdf` doctype. It
models five phases — `preparing → building → finalizing → completed → failed`
— plus `retrying`. It is wired into `documentGenerationTracker.ts` and
renders through `DocumentGenerationCard.module.css` using design-system
tokens and `lucide-svelte` icons.

The card already has the right **shape** for a professional authoring tool:
filename, type badge, live status line, progress bar, expandable error,
download / copy / dismiss actions. What it does **not** have today is any
signal about the structure of the document being built — the number of
pages, sections, embedded fonts, images, or validation warnings. Those data
points do not flow from the backend yet.

## Dependency footprint

`src-tauri/Cargo.toml:72` — `printpdf = "0.7"` is the only PDF engine in the
tree. No `lopdf`, `genpdf`, or `oxidize-pdf`. See `research/libraries.md` for
how we evaluate those.

---

## Gap summary (observed, not proposed)

The following are **facts** about the current code, not improvement ideas.
Mitigations live in `01-vision.md`/`02-architecture.md`.

### Modeling gaps

1. `PdfPageContent` is a coarse content-blob per page. There is no notion
   of structured blocks inside a page, nor any per-block style.
2. `title` passed to `pdf.start` is dropped before `finalize`, so the
   document has no authoritative title, author, subject, or keyword
   metadata.
3. There is no document-level theme: every render is 20 mm margins,
   Helvetica, 12/18 pt, black.
4. `position` on `pdf.add_page` is declared but unused.

### Rendering gaps

5. Only two builtin fonts are used. No custom TTF/OTF embedding, even
   though `printpdf` supports it.
6. Line wrapping is a character-count heuristic; it is not aware of actual
   glyph widths.
7. Content that overflows the bottom margin is silently truncated; no
   pagination is performed.
8. HTML input is lossy string substitution; CSS is ignored.
9. Markdown support is line-prefix matching; no inline formatting, lists,
   tables, code blocks, links, or images.
10. The `Image` variant does not embed images at all. The file path is
    written into the PDF as literal text.
11. There are no headers, footers, page numbers, or table of contents.
12. There are no hyperlinks, bookmarks, or PDF metadata tags
    (`/Title`, `/Author`, `/Subject`, `/Keywords`).

### Runtime gaps

13. Draft store is in-memory, singleton, and non-persistent. Crashes lose
    work.
14. Artifacts are written under `~/.coder/artifacts/` rather than the Tauri
    app-data tree; they are not under `AppHandle` control.
15. `session_id` on the `ArtifactCreated` event is hardcoded `"current"`.
16. Finalization runs inside the tool future on the caller thread. There
    is no backpressure, cancellation, or progress stream for long writes.

### Test / evidence gaps

17. No golden-file tests. We cannot detect regressions in visual output.
18. `PdfValidator` file checks grep raw bytes instead of parsing — the
    encryption check can false-positive on any document that contains the
    literal string `/Encrypt` anywhere.

### UI gaps

19. `DocumentGenerationCard` cannot show structural facts about the PDF
    being built (page count, embedded fonts, images, warnings) because the
    backend does not emit them.
20. There is no UI surface for a user to configure a default theme, margin
    preset, or font family for PDFs produced by the agent.

These gaps define the delta between "what we have" and the "production-grade
PDF authoring tool" described in `01-vision.md`.
