# PDF Authoring Tool — Shipped

**Status**: COMPLETE
**Completed**: 2026-04-16

The PDF authoring tool has shipped through M1–M3. The original as-is
audit that lived here catalogued 20 gaps; all are now retired (see the
gap tracker in `03-execution.md`).

## Code pointers

- **Core model**: `src-tauri/src/core/documents/pdf/` — typed document,
  block enum, section, metadata, geometry, theme, error types
- **Renderer**: `src-tauri/src/infrastructure/tools/documents/pdf/render/`
  — `PrintPdfRenderer`, layout engine, decorator, font cache, theme loader
- **Tool surface**: `src-tauri/src/infrastructure/tools/documents/pdf/`
  — start, add_page, delete_page, list, write (atomic + cancellable),
  finalize, validate
- **Frontend**: `src-ui/src/lib/components/chat/messages/DocumentGenerationCard.svelte`
  + `documentGenerationTracker.ts` — live progress card with warnings
- **Settings**: `src-tauri/src/commands/config/pdf_theme.rs` — theme picker
  persisted in `coder_config.db`
- **Golden**: `docs/tools/pdf/goldens/journey-a.pdf` — M2 exit gate fixture
