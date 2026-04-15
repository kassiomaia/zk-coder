# Bundled fonts

Fonts loaded by the PDF renderer through the theme (`resources/pdf/themes/`).

## LiberationSans-Regular.ttf, LiberationSans-Bold.ttf, LiberationSans-Italic.ttf

- Source: Fedora `liberation-sans-fonts` package (upstream: Red Hat, Google Arimo).
- License: SIL Open Font License 1.1 — see `LiberationSans-LICENSE.txt`.
- Why bundled: provide regular / bold / italic weights for the T1.5 font cache.

## DejaVuSans.ttf

- Source: DejaVu Fonts project.
- License: DejaVu/Bitstream Vera license (free for bundling, see the project
  website for full text).
- Why bundled: wider Unicode coverage than Liberation Sans — used as the
  fallback family so that glyphs absent from the primary family (box-drawing,
  certain math symbols, etc.) still render without a MissingGlyph warning.
