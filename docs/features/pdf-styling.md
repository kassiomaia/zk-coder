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

`bundled report theme` → `draft.theme_patch` (document-level, set at `pdf_start`) → `page.theme_override` (per-page, set at `pdf_add_page`). Each layer merges on top of the previous one; `Some` fields replace, `None` leaves the lower layer untouched.

### Per-page `theme_override`

`pdf_add_page` accepts an optional `theme_override` with the same shape as the document `theme`. Use it when a specific page needs styling that differs from the document (for example a coloured chapter divider, a larger-type section, or different margins on a fold-out table).

```json
{
  "pdf_id": "pdf_...",
  "content": {"type": "markdown", "content": "# Chapter 2\n\n..."},
  "theme_override": {
    "typography": {"body_font_size_pt": 14},
    "palette":    {"heading": {"r": 0.7, "g": 0.1, "b": 0.1}}
  }
}
```

Semantics:

- Applies only to the page(s) that explicitly carry it. When the renderer reaches the next page without a `theme_override`, it automatically resets to the document theme at that page boundary — `build_document` inserts a synthetic reset `ThemeSwitch` so styling does not bleed forward.
- Only body-level fields take effect: `typography`, `palette`, `spacing`, `page_defaults.margin_mm`.
- `decorations` (header/footer templates) and `fonts.*` (font family filenames) are **not** honoured per-page. The decorator runs a single pass after layout completes, and bundled fonts are loaded once at document construction — both remain document-wide. If the LLM supplies those sections in a page override, they are accepted at validation but ignored at render. This is called out explicitly in the tool description and in `docs/features/pdf-styling.md`.
- Validation ranges are the same as the document `theme`; out-of-range values reject the `pdf_add_page` call with a `ToolError::invalid_args` naming the dotted JSON path.

Under the hood, `build_document` in `src-tauri/src/application/tools/pdf/finalize.rs` emits a `Block::ThemeSwitch(ThemePatch)` marker before each overridden page (and an empty-patch reset after) into the block stream. The renderer handles the marker in `layout_sections` by calling `LayoutEngine::set_params` with freshly-built `LayoutParams` for the merged theme — cursor position and page state are preserved, only the layout knobs swap.

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

- Per-page overrides of `decorations` or `fonts.*`. Header/footer templates and bundled font family filenames are fixed document-wide because the decorator runs a single post-layout pass and the font cache is built once at document construction. If the LLM includes these sections in a page `theme_override`, they are accepted at validation (same shape) but ignored at render.
- Custom font upload. Only bundled TTFs are accepted.
- Named-color escape hatch (e.g., `"navy"`). Colors are always RGB triples.
- HTML/CSS rendering path.
