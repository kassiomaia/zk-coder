# PDF Authoring Tool — Proposed Architecture

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Describe the layered shape the PDF tool should grow into to
reach the vision in `01-vision.md`, mapped onto the existing Rust / Tauri /
Svelte layering rules in `docs/quality/guidelines/rust-backend.md`.

---

## Guiding principles

- **Preserve the existing layer boundaries.** The tool grows inside the
  layers we already have; it does not introduce a new one.
- **Make illegal states unrepresentable.** Every coarse string (content
  type, alignment, page size, theme name) becomes a typed value at the
  IPC boundary.
- **Keep the render engine swappable.** Our domain model must not leak
  `printpdf` types.
- **Do not reinvent what `printpdf` already does.** Where the library
  offers a primitive (images, hyperlinks, custom fonts), we wrap it; we
  do not parallel-implement it.
- **TDD the domain first, the engine last.** Layout and style decisions
  belong to pure Rust we can test without touching bytes.

---

## Layer map

Mapped to the existing backend layers (`rust-backend.md § Repository-specific
architectural rules`):

| Layer | New responsibility for PDF |
|---|---|
| `core/documents/pdf/` (new) | Typed domain model: `PdfDocument`, `Section`, `Block`, `InlineRun`, `Style`, `Theme`, `PageGeometry`, `Metadata`. No I/O, no engine calls. |
| `core/documents/pdf/` | Domain errors (`PdfBuildError`, `LayoutError`). |
| `application/tools/pdf/` (new) | Use cases: `StartDocument`, `AppendBlock`, `AppendSection`, `SetTheme`, `AddCover`, `AddToc`, `Finalize`. Orchestrates draft store, theme registry, and rendering job. |
| `infrastructure/tools/documents/pdf/` (existing) | Tool-facing adapters that parse JSON, call application use cases, and serialize results. |
| `infrastructure/tools/documents/pdf/render/` (new submodule) | The `printpdf`-backed renderer implementing the `PdfRenderer` trait. Layout helpers, font cache, image decoder, page composer. |
| `commands/` | Only if we surface a Tauri command for user-driven PDF actions later. Not required for phase 1. |

Every file above targets the ≤500 LoC soft limit. `final.rs` is split and
retired.

---

## Domain model (target shape)

All in `core/documents/pdf/`. Everything is `serde`-serializable so drafts
can later persist if needed, but persistence is not required for phase 1.

```rust
pub struct PdfDocument {
    pub metadata: Metadata,
    pub theme: ThemeId,
    pub geometry: PageGeometry,
    pub cover: Option<Cover>,
    pub toc: Option<TocSpec>,
    pub sections: Vec<Section>,
}

pub struct Metadata {
    pub title: String,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub language: Lang,            // newtype over BCP-47
    pub creator: &'static str,     // our product string
}

pub struct PageGeometry {
    pub size: PageSize,            // enum; supports Custom(Mm, Mm)
    pub orientation: Orientation,  // Portrait | Landscape
    pub margins: Margins,          // per-side Mm
}

pub struct Section {
    pub heading: Heading,          // drives TOC + outline
    pub blocks: Vec<Block>,
}

pub enum Block {
    Paragraph(Paragraph),
    List(ListBlock),
    Quote(Paragraph),
    Code(CodeBlock),
    Table(TableBlock),
    Image(ImageBlock),
    Callout(Callout),
    Rule,
    PageBreak,
}

pub struct Paragraph {
    pub runs: Vec<InlineRun>,      // bold / italic / code / link
    pub align: Align,              // Left | Right | Center | Justify
}

pub struct InlineRun {
    pub text: String,
    pub style: InlineStyle,        // bitflags or struct
    pub link: Option<Url>,
}
```

Key properties:

- `Block` is an enum with a closed set of variants. No stringly-typed
  content. New block types are added by extending the enum and the
  renderer trait.
- `ThemeId` references a `Theme` in a registry (see below). Documents do
  not inline their theme; they point at one. This keeps the draft payload
  small and makes re-theming cheap.

## Theme model

```rust
pub struct Theme {
    pub id: ThemeId,
    pub typography: TypographyScale,   // font family chain + sizes
    pub palette: Palette,              // foreground/background/accents
    pub spacing: Spacing,              // leading, block spacing, indents
    pub page: PageDefaults,            // default margins, header/footer
}
```

- A small number of themes ship with the tool (e.g. `report`, `memo`,
  `letter`). Agents pick one by id.
- A theme is **pure data** — it does not know about `printpdf`. The
  renderer reads it.
- Themes can be described in TOML or JSON on disk under
  `src-tauri/resources/pdf/themes/`, loaded once at startup.
- Values like margins and colors are sourced from the theme; the UI for
  configuring them mirrors the design-system tokens where relevant
  (e.g. the default accent maps to `--color-accent`).

## Renderer trait

```rust
#[async_trait]
pub trait PdfRenderer: Send + Sync {
    async fn render(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
        progress: Option<ProgressSink>,
    ) -> Result<RenderOutcome, RenderError>;
}

pub struct RenderOutcome {
    pub page_count: usize,
    pub embedded_fonts: Vec<FontRef>,
    pub warnings: Vec<RenderWarning>,
    pub byte_len: u64,
}
```

- Phase 1 has **one** implementation, `PrintPdfRenderer`, under
  `infrastructure/tools/documents/pdf/render/`.
- The trait lets us evaluate `genpdf` or `oxidize-pdf` as a second backend
  without disturbing the tool surface, if POC evidence requires it.
- `RenderWarning` is a typed enum (`MissingGlyph`, `ImageScaledDown`,
  `LongTableRow`), not a free-form string.

## Layout engine (inside the renderer)

The renderer contains a small internal layout engine. It does **not** try
to be a browser. It handles exactly the blocks the domain model exposes.

- **Font metrics**: load each font once through `printpdf`'s font API and
  compute glyph advances from the parsed TTF using the existing
  dependencies (e.g. `rusttype`/`ttf-parser` — pick whichever is already
  pulled in transitively). Cache per-font metrics.
- **Line breaking**: greedy word-wrap against measured widths; no
  character-count heuristic. Dashes and soft hyphens are respected.
- **Pagination**: the layout engine knows the usable height and creates a
  new page when a block cannot fit. Blocks are allowed to split at
  paragraph boundaries; `Block::PageBreak` forces one.
- **Headers / footers / page numbers**: rendered by the engine from the
  theme's `PageDefaults`, using template tokens (`{title}`, `{page}`,
  `{total_pages}`).
- **Table layout**: column widths come from the `TableBlock`. The phase 1
  implementation supports fixed and `auto` columns; fancier fitting is
  deferred.
- **TOC / outline**: collected while laying out, written on a second pass
  (or patched in-place) once `{total_pages}` is known.

Each of these is testable in isolation without producing a PDF file.

---

## Communication layer

This is the contract between agent / tool, runtime, and UI.

### Tool surface (re-shaped)

The **existing** tool names stay the same so agents relying on
`pdf.start` / `pdf.add_page` / `pdf.finalize` keep working. New tools are
**added**, and the old ones gain typed fields instead of stringly variants.

| Tool ID | Status | Change |
|---|---|---|
| `pdf.start` | existing | Accepts typed `metadata`, `geometry`, `theme` object. **Stores** `title` on the draft (currently dropped). |
| `pdf.set_theme` | new | Change theme on an in-flight draft. |
| `pdf.add_section` | new | Append a `Section` (heading + blocks). |
| `pdf.add_block` | new | Append a single `Block` to the current section. Preferred path for granular agent actions. |
| `pdf.add_page` | existing | Kept as a compatibility shim that maps to `add_section` with a single `PageBreak`. |
| `pdf.add_cover` | new | Set or replace the cover page. |
| `pdf.add_toc` | new | Request a TOC insert at a position. |
| `pdf.list_blocks` | new | Mirror of `pdf.list_pages` at block granularity. |
| `pdf.delete_block` | new | Remove a specific block. |
| `pdf.validate` | new | Run `PdfValidator` on the current draft and return a report. |
| `pdf.finalize` | existing | Accepts an `auto_save` flag; returns `RenderOutcome` + artifact id. |

Every tool's `input_schema` is machine-generated from a typed request
struct, so there is one source of truth per tool and the agent always
sees the authoritative shape.

### Events

New `AgentEvent` variants or reuse of the existing ones, emitted from the
render job:

- `PdfPhaseChanged { pdf_id, phase }` — building → laying_out → writing
  → validating → complete
- `PdfProgress { pdf_id, percent, current_block, total_blocks }`
- `PdfWarning { pdf_id, warning }`
- `ArtifactCreated { … }` — existing, emitted once on success. The
  hardcoded `session_id: "current"` is replaced by the actual session
  value flowing through `ToolExecutionContext`.

The `DocumentGenerationCard` already understands the
`preparing/building/finalizing/completed/failed/retrying` phase vocabulary
— we map the new events onto it and surface extra structural facts
(page count, warning count) in the card's status area without adding new
card variants.

### Async and cancellation

Render runs inside a `tokio::task::spawn_blocking` (or a dedicated worker
thread if the engine is strictly synchronous), wrapped in a cancellation
token so the user can dismiss the job. The tool future returns once the
job is accepted; progress streams through events. This mirrors the lane
isolation lesson from `research/industry.md`.

---

## Storage and persistence

- Drafts remain in the existing `DraftStore` singleton in phase 1. The
  typed domain model lives there as the source of truth.
- Finalized PDFs move **from** `~/.coder/artifacts/` **to** the Tauri
  app-data directory via `AppHandle::path().app_local_data_dir()`. This is
  a breaking path change for existing users; a one-time migration on
  startup moves old artifacts.
- Themes are loaded from bundled resources plus an optional user override
  directory under the same app-data tree.

---

## Design-system alignment (frontend)

The authoring-side surfaces (live card, theme picker, validation report)
must only use tokens from
`src-ui/src/lib/styles/design-system.css` and the shared animations /
scrollbar / reduced-motion files documented in `RULES.md`. Concretely:

- No new colors, shadows, radii, or fonts — reuse existing tokens.
- Icons come from `lucide-svelte`, matching `DocumentGenerationCard`'s
  existing usage.
- Any new modal (theme picker, validation report) follows the baseline
  patterns already used in settings panels so it reads as part of the
  product, not an add-on.

No visual surface ships until it passes the CSS integrity test
(`npx vitest run src/lib/styles/css-integrity.test.ts`) per `RULES.md §
CSS Verification Checklist`.

---

## Open questions (tracked; resolved via POC)

1. **Do we need `genpdf` or does `printpdf` + our layout engine suffice?**
   Decided after POC-1 produces real numbers on font embedding size and
   layout correctness.
2. **How do we ship default themes?** TOML files bundled in
   `src-tauri/resources/pdf/themes/` vs. hardcoded Rust constants. We pick
   after POC-2.
3. **Is in-memory draft store enough, or do we need SQLite-backed drafts?**
   Revisit when drafts start exceeding a few MB or survive-restart is
   requested.
4. **Where does the typed `input_schema` generation live?** Candidate:
   `schemars` crate on request structs. Decision deferred until the first
   tool is rewritten to typed form.

Each question has a gating POC in `03-poc-plan.md`.
