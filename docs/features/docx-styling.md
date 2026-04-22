# DOCX Styling

**Status**: Active
**Owner**: Document generation
**Related**: [PDF Styling](pdf-styling.md)

## Overview

`docx_start` accepts an optional `theme` argument that lets the LLM override the bundled report defaults — typography, palette, spacing, page geometry, header/footer templates, and font families. `docx_add_section` accepts an optional `theme_override` that opens a new OOXML section break with distinct page geometry for a run of sections (landscape chart page, wider margins for a reference insert, etc.).

The shape and error semantics mirror the PDF styling contract so agents can move between formats with a consistent mental model.

## Contract

### `docx_start.theme`

| Section | Field | Type | Range | Default |
|---|---|---|---|---|
| `typography` | `body_font_size_pt` | integer | 6–72 | 11 |
| `typography` | `heading_font_size_pt` | integer | 8–96 | 20 |
| `typography` | `line_spacing` | number | 1.0–2.5 | 1.15 |
| `typography` | `heading_scale` | number | 0.5–1.0 | 0.82 |
| `palette.*` | `{ "hex": "RRGGBB" }` | object | 6-char hex, optional `#` | role-specific |
| `spacing.paragraph_space_before_pt` | integer | 0–72 | 0 |
| `spacing.paragraph_space_after_pt` | integer | 0–72 | 8 |
| `spacing.list_indent_pt` | integer | 0–144 | 18 |
| `spacing.table_cell_padding_pt` | integer | 0–36 | 4 |
| `page.paper` | enum | `A4`, `Letter`, `Legal` | A4 |
| `page.landscape` | boolean | — | false |
| `page.margin_*_mm` | number | 5.0–80.0 | 25.0 |
| `decorations.header` | string or `null` | tokens below | `null` |
| `decorations.footer` | string or `null` | tokens below | `null` |
| `decorations.different_first_page` | boolean | — | false |
| `fonts.body` | string | allow-list | Calibri |
| `fonts.heading` | string | allow-list | Calibri |
| `fonts.mono` | string | allow-list | Consolas |

Palette roles: `text`, `heading`, `link`, `code_text`, `code_bg`, `table_header_bg`, `table_border`.

Font allow-list: `Calibri`, `Cambria`, `Arial`, `Helvetica`, `Times New Roman`, `Georgia`, `Verdana`, `Tahoma`, `Courier New`, `Consolas`.

### `docx_add_section.theme_override`

Same shape as `theme`. Only the `page` section takes effect; body-level fields (typography, palette, spacing, fonts) and decorations are fixed at document level. A section break is opened immediately before the section that carries the override.

### Validation

Every leaf is optional. Unknown top-level or nested fields are rejected with `ToolError::InvalidArguments`. Out-of-range numbers, non-finite floats, malformed hex colors, and unknown font names all return a dotted JSON path so the agent can self-correct (e.g. `theme.typography.body_font_size_pt`, `theme.palette.heading.hex`).

Decorations headers/footers use the "double option" pattern: omit the key to leave the base theme alone; pass explicit `null` to clear.

### Token substitution

Header/footer templates support `{title}` (expanded from the draft's title). `{page}` and `{total}` currently render as literal placeholder text — the underlying crate does not publicly expose the `InstrText` type needed to emit the OOXML field instructions. A follow-up can swap them for live Word fields once the crate exposes the type or we vendor a small patch.

## LLM examples

### Partial recolor

```json
{
  "filename": "design-review.docx",
  "title": "Design Review",
  "theme": {
    "palette": {
      "heading": { "hex": "3B82F6" },
      "link":    { "hex": "2563EB" }
    }
  }
}
```

### Landscape section in a portrait document

```json
// docx_start — portrait defaults
// docx_add_section — heading
// docx_add_section — paragraph
// docx_add_section — table, with theme_override to switch to landscape
{
  "content": { "type": "Table", "headers": [...], "rows": [...] },
  "theme_override": {
    "page": { "landscape": true, "margin_left_mm": 15.0, "margin_right_mm": 15.0 }
  }
}
```

### Inline formatting

```json
{
  "content": {
    "type": "Paragraph",
    "text": [
      { "text": "Important: ", "bold": true, "color": "B91C1C" },
      { "text": "always ", "italic": true },
      { "text": "check your backups before upgrading." }
    ]
  }
}
```

## Content primitives

| Type | Notes |
|---|---|
| `Heading` | Levels 1–9. Accepts `alignment`, `page_break_before`. |
| `Paragraph` | Plain string or rich runs. Accepts `alignment`, `role` (`body`, `quote`, `code`). |
| `BulletList` / `NumberedList` | Real Word numbering definition, one `num_id` per list, nesting via `ListItem.level` (0–8). `start_at` supported on `NumberedList`. |
| `Table` | `headers` + `rows` + optional `caption`. Header row is shaded and bold; cells use theme's `table_border`. |
| `Image` | Real PNG/JPEG embedding via `Drawing`/`Picture`. `width_mm` (10–300, default 120); aspect ratio preserved. |
| `PageBreak` | Hard page break paragraph. |
| `HorizontalRule` | Bottom-bordered empty paragraph (`pBdr`). |

## Precedence

1. Bundled report defaults (`DocxTheme::report_default`).
2. Document-level theme patch from `docx_start` (full theme surface).
3. Per-section `theme_override` from `docx_add_section` (page-level only).

## Crate decision

Implemented against `docx-rust = "0.1.11"` after a capability spike confirmed support for real image embedding, numbering definitions, section properties with distinct page geometry, headers/footers, paragraph borders, runs with bold/italic/underline/color, and hyperlinks. Known crate limitations surfaced during implementation:

- `InstrText` type is not re-exported; `{page}` / `{total}` tokens render as placeholder text rather than live fields.
- `add_rel` registers footers with the header schema; worked around by pre-populating `document.xml.rels` with the correct `SCHEMA_FOOTER` URL before the crate's ZIP writer runs.
- Image bytes are held as `&Vec<u8>` with a lifetime that cannot be tied to our plan; we use `Box::leak` to produce `&'static Vec<u8>`. Bounded trickle per generated document; memory is reclaimed at process exit.

A crate migration is tracked as a follow-up but is not required to ship parity with PDF styling.

## Architecture

- Core value objects and patch type: `src-tauri/src/core/documents/docx/{theme.rs, theme_patch.rs}`
- Tool boundary (schema + validation): `src-tauri/src/infrastructure/tools/documents/docx/{start.rs, add_section.rs, list.rs, schema.rs, validate.rs}`
- Generator pipeline: `src-tauri/src/infrastructure/tools/documents/docx/generator/` — one file per concern (`plan.rs`, `runs.rs`, `paragraphs.rs`, `lists.rs`, `images.rs`, `tables.rs`, `sectpr.rs`, `styles.rs`, `decorations.rs`, `content_types.rs`, `blocks.rs`, `units.rs`).
- Draft store adds `theme_patch: Option<DocxThemePatch>` on `DocxDraft` and `theme_override` on `DraftSection` alongside richer `DocxContent` types with inline run support.

## Verification

1. `cargo test -p coder -- docx` — 64 unit tests cover validation, schemas, plan building, paragraph/run emission, list dispatch, and end-to-end ZIP output.
2. `cargo build` / `cargo check` — clean on `docx-rust = "0.1.11"`.
3. Manual: drive the agent with "Generate a design review as a Word document with 12pt Inter body, 1.4 line spacing, 20mm margins, accent `#3B82F6`, a numbered feature list, and one embedded PNG." Open the result in LibreOffice Writer and Microsoft Word to confirm fonts, spacing, margins, color, inline formatting, real image, and real numbered list all render correctly.
4. Negative: supply an intentionally invalid theme (e.g. `body_font_size_pt: 500`) and confirm the tool call surfaces a dotted-path error so the LLM can correct.

## Out of scope

- Live `PAGE` / `NUMPAGES` fields (blocked on crate support; tokens render as placeholders).
- Tracked changes, comments, revision marks.
- Embedded equations, charts, SmartArt.
- LLM-uploadable custom fonts (bundled allow-list only).
- PPTX styling parity (explicit follow-up).
