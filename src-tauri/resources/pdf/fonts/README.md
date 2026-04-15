# Bundled fonts

Fonts loaded by the PDF renderer (`core/documents/pdf/render.rs`).

## LiberationSans-Regular.ttf

- Source: Fedora `liberation-sans-fonts` package (upstream: Red Hat, Google Arimo).
- License: SIL Open Font License 1.1 — see `LiberationSans-LICENSE.txt`.
- Why bundled: T1.2 requires a TTF file on disk so the renderer can compute
  real glyph advances via `ttf-parser` instead of a character-count heuristic.
- T1.5 introduces a proper font cache with a fallback chain. Until then this
  file is the single font the renderer loads.
