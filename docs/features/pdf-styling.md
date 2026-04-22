# PDF Styling — LLM-Tunable Theme

**Status**: ACTIVE
**Last reviewed**: 2026-04-22
**Purpose**: Contract for the `theme` argument on `pdf_start`, which lets the LLM override the bundled visual defaults per document.

---

## Overview

The PDF tool accepts an optional `theme` object at `pdf_start`. Every field is optional: the LLM supplies only the knobs the user asked to change, and the bundled report theme fills in the rest. The patch is stored on the draft, merged onto the bundled theme at render time, and flows through the existing `printpdf` pipeline unchanged.

## Backend shape

The root `theme` object mirrors the typed `Theme` in `src-tauri/src/core/documents/pdf/theme.rs`. The patch type lives in `src-tauri/src/core/documents/pdf/theme_patch.rs`.

### Supported sections (all optional)

| Section | Fields | Notes |
|---------|--------|-------|
| `typography` | `body_font_size_pt` (6–72), `heading_font_size_pt` (8–96), `line_leading` (1.0–2.5) | Defaults: 11pt / 18pt / 1.3. Use `line_leading: 2.0` for double-spaced. |
| `palette.*` | `text`, `heading`, `code_bg`, `code_text`, `callout_bg`, `callout_border`, `table_header_bg`, `table_border` | Each is an RGB triple with channels `r`, `g`, `b` in `[0.0, 1.0]`. Missing channels inherit the base color. |
| `spacing` | `block_gap` (0–5), `list_indent_mm` (0–40), `block_padding_mm` (0–20), `table_cell_padding_mm` (0–20) | `block_gap` is a multiplier of body leading. |
| `page_defaults.margin_mm` | 5–80 | Uniform page margin. Default 20mm. |
| `decorations` | `header`, `footer`, `suppress_on_first_page`, `font_size_pt` (6–20), `margin_mm` (2–30) | `header` / `footer` accept a string or explicit `null` (drops the strip). Templates support tokens `{title}`, `{page}`, `{total_pages}`, `{section}`. |
| `fonts` | `regular`, `bold`, `italic`, `mono`, `fallbacks` | Filenames must be members of the bundled set: `LiberationSans-Regular.ttf`, `LiberationSans-Bold.ttf`, `LiberationSans-Italic.ttf`, `LiberationMono-Regular.ttf`, `DejaVuSans.ttf`. |

### Precedence

`bundled report theme` → `draft.theme_patch` applied at render time in `theme_for_draft`.

No per-page overrides exist yet (Phase 2 follow-up). Styling is fixed once at `pdf_start` and applies to every page in the draft.

## Validation and errors

`ThemePatch::validate` runs before the draft is stored. On failure the tool returns `ToolError::invalid_args` naming the dotted JSON path of the offending field, for example:

```
pdf_start: theme.palette.heading.r: value 1.50 out of range [0.00, 1.00]
```

This is the next-step signal the LLM uses to self-correct. Errors are surfaced per-field, not batched, so the first failure wins.

Rejected cases:

- RGB channel outside `[0, 1]`
- Font size outside `[6, 72]` body or `[8, 96]` heading
- `line_leading` below 1.0 or above 2.5
- Margin outside `[5, 80]` mm
- Non-finite floats (NaN, infinity)
- Font filename not in the bundled allow-list

## LLM-facing examples

### 1. Recolored headings + bigger body

```json
{
  "filename": "report.pdf",
  "title": "Quarterly Report",
  "theme": {
    "typography": { "body_font_size_pt": 13 },
    "palette":    { "heading": { "r": 0.1, "g": 0.2, "b": 0.8 } }
  }
}
```

### 2. Custom footer, no header, wide left margin

```json
{
  "filename": "letter.pdf",
  "title": "Cover Letter",
  "theme": {
    "decorations": {
      "header": null,
      "footer": "Confidential — {page}/{total_pages}"
    },
    "page_defaults": { "margin_mm": 35 }
  }
}
```

### 3. Monospace body for a code-heavy handout

```json
{
  "filename": "handout.pdf",
  "title": "API Reference",
  "theme": {
    "fonts": { "regular": "LiberationMono-Regular.ttf" },
    "typography": { "body_font_size_pt": 10, "line_leading": 1.2 }
  }
}
```

## Key files

- `src-tauri/src/core/documents/pdf/theme.rs` — base `Theme`, applies patch via `Theme::apply_patch` / `Theme::with_patch`.
- `src-tauri/src/core/documents/pdf/theme_patch.rs` — `ThemePatch`, nested `*Patch` structs, validation, `DoubleOption<T>` for header/footer clearing semantics.
- `src-tauri/src/infrastructure/tools/documents/pdf/start.rs` — tool schema, `decode_theme_patch` validation at the tool boundary.
- `src-tauri/src/infrastructure/tools/documents/pdf/render/theme_loader.rs` — `theme_for_draft(&draft)` merges the bundled theme with the draft's patch.
- `src-tauri/src/infrastructure/tools/documents/pdf/write.rs` — render seam that calls `theme_for_draft` instead of the unconditional bundled load.
- `src-tauri/src/infrastructure/tools/documents/shared/draft_store.rs` — `PdfDraft.theme_patch` persists the override for the life of the draft.
- `src-tauri/src/infrastructure/agent/registry.rs` — "PDF STYLING" block in the `DOCUMENT GENERATION` section teaches the LLM the new surface.

## Backwards compatibility

- `theme` is optional; existing call patterns work unchanged.
- Absent patch → `theme_for_draft` returns the bundled report theme, byte-identical to pre-change behaviour. Verified by the `theme_for_draft_without_patch_matches_bundled` unit test.

## Content types: cover page and sized images

`pdf_add_page` accepts four `content` variants. The Phase 1 surface shipped `markdown` / `text` / `image` (auto-sized). Phase 2 adds explicit image sizing and a first-class cover page.

### Cover page — `type: "cover"`

Renders a dedicated page with title / subtitle / author / date centred horizontally and vertically within the usable area.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `title` | string | yes | Rendered at heading font size. |
| `subtitle` | string | no | Italic, body font size. |
| `author` | string | no | Body font size. |
| `date` | string | no | Body font size. |

```json
{ "type": "cover", "title": "Quarterly Report", "subtitle": "Q1 2026", "author": "Ada Lovelace", "date": "2026-04-22" }
```

Behaviour:

- Forces a page break before the cover if the cursor has already moved below the top margin.
- Records the title on the PDF outline so bookmarks still point at the cover.
- Always forces a page break after so subsequent content starts on a fresh page.
- Header / footer suppression on the cover is handled by the existing `decorations.suppress_on_first_page` flag — the bundled default is `true`, which leaves the cover clean. Set it to `false` in the document `theme` if you want decorations on page 1.

### Image sizing — `type: "image"`

`width_mm`, `height_mm`, and `fit` are all optional.

| Field | Type | Range | Notes |
|-------|------|-------|-------|
| `width_mm` | number | `[1, 400]` | Display width in mm. Omit to fit the page. |
| `height_mm` | number | `[1, 400]` | Display height in mm. Omit to derive from aspect ratio. |
| `fit` | enum | `contain` \| `stretch` | Only used when both dimensions are set. |

Sizing rules:

- Neither set → full usable page width, aspect preserved.
- Only `width_mm` → height from aspect ratio.
- Only `height_mm` → width from aspect ratio.
- Both set + `fit: "contain"` (default) → scale to fit inside the box, aspect preserved (letterboxed).
- Both set + `fit: "stretch"` → use exact box, may distort.

```json
{ "type": "image", "path": "/abs/path/logo.png", "width_mm": 80, "height_mm": 40, "fit": "contain", "caption": "Acme Inc." }
```

## Backwards compatibility

- `theme` on `pdf_start` is optional; existing call patterns work unchanged.
- Absent patch → `theme_for_draft` returns the bundled report theme, byte-identical to pre-change behaviour. Verified by the `theme_for_draft_without_patch_matches_bundled` unit test.
- `cover` and the new image sizing fields are additive — existing `pdf_add_page` calls without them behave identically to before.

## Non-goals (tracked as follow-ups)

- Per-page theme overrides. Styling is document-wide for now. Implementing this requires rebuilding `LayoutParams` at page boundaries inside the `printpdf` renderer (a ~130 LOC change through ~15 function signatures) — deferred to Phase 3 so it can be reviewed in isolation from cover/image work.
- Custom font upload. Only bundled TTFs are accepted.
- Named-color escape hatch (e.g., `"navy"`). Colors are always RGB triples.
- HTML/CSS rendering path.
