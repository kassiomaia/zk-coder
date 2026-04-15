//! `PrintPdfRenderer`: the T1.5 renderer.
//!
//! Produces a PDF from a [`PdfDocument`] using real glyph advances pulled
//! from the theme's declared fonts. T1.6 replaces the one-page cap with
//! a layout engine that paginates.
//!
//! Every visual constant lives in the loaded [`Theme`]; the sweep test
//! at the bottom of this module fails on any numeric literal that slips
//! into a function body.

use std::collections::BTreeSet;
use std::io::{Cursor, Write};

use async_trait::async_trait;
use printpdf::{
    IndirectFontRef, Mm, PdfDocument as PrintPdfDoc, PdfDocumentReference, PdfLayerReference,
};
use ttf_parser::Face;

use crate::core::documents::pdf::block::{Block, InlineRun, InlineStyle, Paragraph};
use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::geometry::Orientation;
use crate::core::documents::pdf::render::{
    FontRef, PdfRenderer, ProgressSink, RenderError, RenderOutcome, RenderWarning, Theme,
};
use crate::core::documents::pdf::section::Section;

use super::font_cache::{FontCache, FontSlot, LoadedFont};
use super::layout::{LayoutEngine, LayoutParams, POINTS_PER_MM};

/// Renderer backed by the `printpdf` crate and a [`FontCache`].
pub struct PrintPdfRenderer {
    fonts: FontCache,
}

impl PrintPdfRenderer {
    /// Builds a renderer around a preloaded font cache.
    #[must_use]
    pub const fn new(fonts: FontCache) -> Self {
        Self { fonts }
    }

    /// Exposes the cache for introspection (tests, diagnostics).
    #[must_use]
    pub const fn fonts(&self) -> &FontCache {
        &self.fonts
    }

    /// Returns the family name declared by the regular-weight font.
    #[must_use]
    pub fn family_name(&self) -> &str {
        self.fonts
            .all()
            .into_iter()
            .find(|f| f.slot() == FontSlot::Regular)
            .map_or("Unknown", LoadedFont::family)
    }

    /// Returns `text` wrapped into lines that each fit within
    /// `max_width_pt` at `font_size_pt`, using the regular-weight font's
    /// glyph advances.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if the loaded font cannot be reparsed.
    pub fn wrap_line(
        &self,
        text: &str,
        font_size_pt: f32,
        max_width_pt: f32,
    ) -> Result<Vec<String>, RenderError> {
        let regular = self.fonts.primary_for(&InlineStyle::PLAIN);
        let face = regular.face()?;
        Ok(wrap_with_face(&face, text, font_size_pt, max_width_pt))
    }

    /// Returns the width in points of `text` at `font_size_pt`, measured
    /// with the regular-weight font.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if the loaded font cannot be reparsed.
    pub fn measure_text_pt(&self, text: &str, font_size_pt: f32) -> Result<f32, RenderError> {
        let regular = self.fonts.primary_for(&InlineStyle::PLAIN);
        let face = regular.face()?;
        Ok(measure_with_face(&face, text, font_size_pt))
    }

    fn render_sync(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
    ) -> Result<RenderOutcome, RenderError> {
        let (page_width_mm, page_height_mm) = page_dimensions(doc);
        let (pdf_doc, page_idx, layer_idx) = PrintPdfDoc::new(
            doc.metadata.title(),
            Mm(page_width_mm),
            Mm(page_height_mm),
            "Layer 1",
        );

        let font_refs = embed_all_fonts(&pdf_doc, &self.fonts)?;
        let first_layer = pdf_doc.get_page(page_idx).get_layer(layer_idx);
        let params = LayoutParams::from_doc(doc, theme, page_width_mm, page_height_mm);
        let mut engine = LayoutEngine::new(&pdf_doc, first_layer, params);

        let mut missing: BTreeSet<char> = BTreeSet::new();
        layout_sections(
            &mut engine,
            &font_refs,
            &self.fonts,
            &doc.sections,
            &mut missing,
        )?;
        let page_count = engine.page_count();
        drop(engine);

        let bytes = pdf_doc
            .save_to_bytes()
            .map_err(|e| RenderError::Engine(e.to_string()))?;
        sink.write_all(&bytes)?;
        sink.flush()?;
        let byte_len = u64::try_from(bytes.len())
            .map_err(|_| RenderError::Engine("output exceeds u64".to_string()))?;

        let embedded_fonts: Vec<FontRef> = self
            .fonts
            .all()
            .iter()
            .map(|f| FontRef {
                family: format!("{} ({})", f.family(), f.slot().label()),
                bytes_len: f.bytes().len(),
            })
            .collect();

        let warnings: Vec<RenderWarning> = missing
            .into_iter()
            .map(|c| RenderWarning::MissingGlyph {
                text: c.to_string(),
            })
            .collect();

        Ok(RenderOutcome {
            page_count,
            embedded_fonts,
            warnings,
            byte_len,
        })
    }
}

const fn page_dimensions(doc: &PdfDocument) -> (f32, f32) {
    let (portrait_w, portrait_h) = doc.geometry.size.dimensions_mm();
    match doc.geometry.orientation {
        Orientation::Portrait => (portrait_w, portrait_h),
        Orientation::Landscape => (portrait_h, portrait_w),
    }
}

fn layout_sections(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    sections: &[Section],
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    for section in sections {
        emit_heading(engine, font_refs, fonts, section.heading.text(), missing)?;
        let block_gap = engine.params().block_gap_mm;
        engine.advance(block_gap);
        for block in &section.blocks {
            match block {
                Block::Paragraph(p) => emit_paragraph(engine, font_refs, fonts, p, missing)?,
                Block::Heading(h) => emit_heading(engine, font_refs, fonts, h.text(), missing)?,
                Block::PageBreak => engine.break_page(),
            }
            let gap = engine.params().block_gap_mm;
            engine.advance(gap);
        }
    }
    Ok(())
}

/// Emits each [`InlineRun`] of `p` on its own lines using the run's
/// style-matched font. Lines that would overflow the page are pushed
/// onto a fresh page by the [`LayoutEngine`].
fn emit_paragraph(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    p: &Paragraph,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    for run in p.runs() {
        emit_run(engine, font_refs, fonts, run, missing)?;
    }
    Ok(())
}

fn emit_run(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    run: &InlineRun,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let primary = fonts.primary_for(&run.style);
    let face = primary.face()?;
    let body_size_pt = engine.params().body_size_pt;
    let usable_width_pt = engine.params().usable_width_pt;
    let lines = wrap_with_face(&face, run.text(), body_size_pt, usable_width_pt);
    for line in lines {
        let leading = engine.params().body_leading_mm;
        engine.ensure_space_for(leading);
        let layer = engine.current_layer();
        let x = engine.params().x_mm;
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &run.style,
            &line,
            body_size_pt,
            x,
            y,
            missing,
        )?;
        engine.advance(leading);
    }
    Ok(())
}

/// Emits `text` at the heading size using the regular-weight font,
/// paginating through the engine.
fn emit_heading(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    text: &str,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let primary = fonts.primary_for(&InlineStyle::PLAIN);
    let face = primary.face()?;
    let heading_size_pt = engine.params().heading_size_pt;
    let usable_width_pt = engine.params().usable_width_pt;
    let lines = wrap_with_face(&face, text, heading_size_pt, usable_width_pt);
    for line in lines {
        let leading = engine.params().heading_leading_mm;
        engine.ensure_space_for(leading);
        let layer = engine.current_layer();
        let x = engine.params().x_mm;
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &InlineStyle::PLAIN,
            &line,
            heading_size_pt,
            x,
            y,
            missing,
        )?;
        engine.advance(leading);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
fn emit_line_with_fallback(
    layer: &PdfLayerReference,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    style: &InlineStyle,
    line: &str,
    font_size_pt: f32,
    start_x_mm: f32,
    y_mm: f32,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let primary = fonts.primary_for(style);
    let mut x_mm = start_x_mm;
    let mut current_slot: FontSlot = primary.slot();
    let mut current_text = String::new();

    for c in line.chars() {
        let slot = fonts.resolve(c, style).map_or_else(
            || {
                missing.insert(c);
                primary.slot()
            },
            LoadedFont::slot,
        );
        if slot != current_slot && !current_text.is_empty() {
            x_mm = emit_segment(
                layer,
                font_refs,
                fonts,
                current_slot,
                &current_text,
                font_size_pt,
                x_mm,
                y_mm,
            )?;
            current_text.clear();
        }
        current_slot = slot;
        current_text.push(c);
    }
    if !current_text.is_empty() {
        emit_segment(
            layer,
            font_refs,
            fonts,
            current_slot,
            &current_text,
            font_size_pt,
            x_mm,
            y_mm,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_segment(
    layer: &PdfLayerReference,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    slot: FontSlot,
    text: &str,
    font_size_pt: f32,
    x_mm: f32,
    y_mm: f32,
) -> Result<f32, RenderError> {
    let font_ref = font_refs
        .get(slot)
        .ok_or_else(|| RenderError::Engine(format!("missing font ref for slot {slot:?}")))?;
    let font = fonts
        .all()
        .into_iter()
        .find(|f| f.slot() == slot)
        .ok_or_else(|| RenderError::Engine(format!("missing loaded font for slot {slot:?}")))?;
    let face = font.face()?;
    let width_pt = measure_with_face(&face, text, font_size_pt);
    layer.use_text(text, font_size_pt, Mm(x_mm), Mm(y_mm), font_ref);
    Ok(x_mm + width_pt / POINTS_PER_MM)
}

struct FontRefMap {
    regular: IndirectFontRef,
    bold: IndirectFontRef,
    italic: IndirectFontRef,
    mono: IndirectFontRef,
    fallbacks: Vec<IndirectFontRef>,
}

impl FontRefMap {
    fn get(&self, slot: FontSlot) -> Option<&IndirectFontRef> {
        match slot {
            FontSlot::Regular => Some(&self.regular),
            FontSlot::Bold => Some(&self.bold),
            FontSlot::Italic => Some(&self.italic),
            FontSlot::Mono => Some(&self.mono),
            FontSlot::Fallback(i) => self.fallbacks.get(i),
        }
    }
}

fn embed_all_fonts(
    pdf: &PdfDocumentReference,
    cache: &FontCache,
) -> Result<FontRefMap, RenderError> {
    let mut regular: Option<IndirectFontRef> = None;
    let mut bold: Option<IndirectFontRef> = None;
    let mut italic: Option<IndirectFontRef> = None;
    let mut mono: Option<IndirectFontRef> = None;
    let mut fallbacks: Vec<(usize, IndirectFontRef)> = Vec::new();

    for font in cache.all() {
        let reference = pdf
            .add_external_font(Cursor::new(font.bytes()))
            .map_err(|e| RenderError::Engine(e.to_string()))?;
        match font.slot() {
            FontSlot::Regular => regular = Some(reference),
            FontSlot::Bold => bold = Some(reference),
            FontSlot::Italic => italic = Some(reference),
            FontSlot::Mono => mono = Some(reference),
            FontSlot::Fallback(i) => fallbacks.push((i, reference)),
        }
    }

    fallbacks.sort_by_key(|(i, _)| *i);
    Ok(FontRefMap {
        regular: regular.ok_or_else(|| RenderError::Engine("no regular font".to_string()))?,
        bold: bold.ok_or_else(|| RenderError::Engine("no bold font".to_string()))?,
        italic: italic.ok_or_else(|| RenderError::Engine("no italic font".to_string()))?,
        mono: mono.ok_or_else(|| RenderError::Engine("no mono font".to_string()))?,
        fallbacks: fallbacks.into_iter().map(|(_, r)| r).collect(),
    })
}

#[async_trait]
impl PdfRenderer for PrintPdfRenderer {
    async fn render(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
        _progress: Option<ProgressSink>,
    ) -> Result<RenderOutcome, RenderError> {
        self.render_sync(doc, theme, sink)
    }
}

fn glyph_advance_pt(face: &Face<'_>, c: char, font_size_pt: f32) -> f32 {
    let units = face
        .glyph_index(c)
        .and_then(|gid| face.glyph_hor_advance(gid))
        .unwrap_or(0);
    let upem = f32::from(face.units_per_em());
    if upem == 0.0 {
        return 0.0;
    }
    (f32::from(units) / upem) * font_size_pt
}

fn measure_with_face(face: &Face<'_>, text: &str, font_size_pt: f32) -> f32 {
    text.chars()
        .map(|c| glyph_advance_pt(face, c, font_size_pt))
        .sum()
}

fn wrap_with_face(
    face: &Face<'_>,
    text: &str,
    font_size_pt: f32,
    max_width_pt: f32,
) -> Vec<String> {
    let space_width = glyph_advance_pt(face, ' ', font_size_pt);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0.0_f32;
    for word in text.split_whitespace() {
        let word_width = measure_with_face(face, word, font_size_pt);
        if current.is_empty() {
            current.push_str(word);
            current_width = word_width;
        } else if current_width + space_width + word_width <= max_width_pt {
            current.push(' ');
            current.push_str(word);
            current_width += space_width + word_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
            current_width = word_width;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use crate::core::documents::pdf::block::{Block, InlineRun, InlineStyle, Paragraph};
    use crate::core::documents::pdf::document::PdfDocument;
    use crate::core::documents::pdf::metadata::Metadata;
    use crate::core::documents::pdf::section::{Heading, HeadingLevel, Section};
    use crate::core::documents::pdf::theme::Fonts;
    use std::path::PathBuf;

    fn fonts_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdf/fonts")
    }

    fn test_fonts_spec() -> Fonts {
        Fonts {
            regular: "LiberationSans-Regular.ttf".to_string(),
            bold: "LiberationSans-Bold.ttf".to_string(),
            italic: "LiberationSans-Italic.ttf".to_string(),
            mono: "LiberationSans-Regular.ttf".to_string(),
            fallbacks: vec!["DejaVuSans.ttf".to_string()],
        }
    }

    fn renderer() -> PrintPdfRenderer {
        let cache = FontCache::load(&test_fonts_spec(), &fonts_root()).expect("fonts load");
        PrintPdfRenderer::new(cache)
    }

    fn fixture_doc() -> PdfDocument {
        let mut doc = PdfDocument::new(Metadata::new("T1.5 Fixture").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Introduction").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(
            Paragraph::plain("Hello from the typed PDF model.").unwrap(),
        ));
        doc.push_section(section);
        doc
    }

    #[test]
    fn renderer_exposes_regular_family_name() {
        let r = renderer();
        assert!(r.family_name().to_lowercase().contains("liberation"));
    }

    #[test]
    fn measure_known_string_uses_real_metrics() {
        let r = renderer();
        let empty = r.measure_text_pt("", 12.0).unwrap();
        assert!((empty - 0.0).abs() < f32::EPSILON);
        let w_i = r.measure_text_pt("i", 12.0).unwrap();
        let w_w = r.measure_text_pt("W", 12.0).unwrap();
        assert!(
            w_w > w_i,
            "proportional metrics: W ({w_w}) should be wider than i ({w_i})"
        );
    }

    #[test]
    fn wrap_line_known_width_produces_known_line_count() {
        let r = renderer();
        let text = "Lorem ipsum dolor sit amet consectetur adipiscing elit";
        let single = r.measure_text_pt(text, 12.0).unwrap();
        let one = r.wrap_line(text, 12.0, single + 10.0).unwrap();
        assert_eq!(one.len(), 1);
        let tight = r.wrap_line(text, 12.0, single / 4.0).unwrap();
        assert!(tight.len() >= 4, "got {}", tight.len());
        let all_joined = tight.join(" ");
        for word in text.split_whitespace() {
            assert!(all_joined.contains(word), "missing word {word}");
        }
    }

    #[tokio::test]
    async fn render_fixture_embeds_every_cache_font() {
        let r = renderer();
        let doc = fixture_doc();
        let theme = Theme::default();
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &theme, &mut buf, None)
            .await
            .expect("render ok");

        assert_eq!(outcome.page_count, 1);
        assert!(outcome.byte_len > 0);
        assert_eq!(outcome.byte_len as usize, buf.len());
        // The cache holds regular / bold / italic / mono + fallback → 5.
        assert_eq!(
            outcome.embedded_fonts.len(),
            5,
            "every cache font must land in the embedded_fonts list"
        );

        assert!(buf.starts_with(b"%PDF-1."));
        assert!(buf.windows(5).any(|w| w == b"%%EOF"));
    }

    #[tokio::test]
    async fn render_bold_and_italic_adds_multiple_fonts_to_pdf_font_dict() {
        let r = renderer();

        let mut doc = PdfDocument::new(Metadata::new("Fonts In Dict").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Styles").unwrap();
        let mut section = Section::new(heading);

        let plain = InlineRun::new("plain text").unwrap();
        let bold = InlineRun::new("bold text")
            .unwrap()
            .with_style(InlineStyle {
                bold: true,
                ..InlineStyle::PLAIN
            });
        let italic = InlineRun::new("italic text")
            .unwrap()
            .with_style(InlineStyle {
                italic: true,
                ..InlineStyle::PLAIN
            });

        section.push(Block::Paragraph(Paragraph::new(vec![plain]).unwrap()));
        section.push(Block::Paragraph(Paragraph::new(vec![bold]).unwrap()));
        section.push(Block::Paragraph(Paragraph::new(vec![italic]).unwrap()));
        doc.push_section(section);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .unwrap();

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let font_count = count_font_entries(&parsed);
        assert!(
            font_count >= 3,
            "rendered PDF must register at least regular/bold/italic in its font dict, got {font_count}"
        );
        // And Helvetica must not be the only thing in the tree.
        assert!(
            !is_only_helvetica(&parsed),
            "renderer must not fall back to bare Helvetica"
        );
    }

    #[tokio::test]
    async fn missing_glyph_in_every_font_emits_warning() {
        let r = renderer();

        // U+E000 is Private Use Area — no mainstream font has it.
        let mut doc = PdfDocument::new(Metadata::new("Missing Glyph").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Warn").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(Paragraph::plain("before\u{E000}after").unwrap()));
        doc.push_section(section);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render does not panic on missing glyphs");

        assert!(
            outcome
                .warnings
                .iter()
                .any(|w| matches!(w, RenderWarning::MissingGlyph { text } if text.contains('\u{E000}'))),
            "missing PUA glyph must surface as a MissingGlyph warning: {:?}",
            outcome.warnings
        );
        assert!(buf.starts_with(b"%PDF-1."));
    }

    /// Helper: counts `/Font` subtype entries in the parsed PDF.
    fn count_font_entries(pdf: &lopdf::Document) -> usize {
        let mut n = 0;
        for (_, obj) in &pdf.objects {
            if let lopdf::Object::Dictionary(d) = obj {
                if let Ok(t) = d.get(b"Type") {
                    if matches!(t, lopdf::Object::Name(name) if name == b"Font") {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    fn is_only_helvetica(pdf: &lopdf::Document) -> bool {
        let mut saw_non_helvetica = false;
        let mut saw_font = false;
        for (_, obj) in &pdf.objects {
            if let lopdf::Object::Dictionary(d) = obj {
                if let Ok(t) = d.get(b"Type") {
                    if matches!(t, lopdf::Object::Name(n) if n == b"Font") {
                        saw_font = true;
                        let base = d
                            .get(b"BaseFont")
                            .ok()
                            .and_then(|o| match o {
                                lopdf::Object::Name(n) => Some(n.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        let name = String::from_utf8_lossy(&base).to_lowercase();
                        if !name.contains("helvetica") {
                            saw_non_helvetica = true;
                        }
                    }
                }
            }
        }
        saw_font && !saw_non_helvetica
    }

    #[tokio::test]
    async fn different_themes_produce_different_output() {
        let r = renderer();
        let doc = fixture_doc();

        let mut theme_a = Theme::default();
        theme_a.typography.body_font_size_pt = 10.0;
        theme_a.typography.heading_font_size_pt = 14.0;

        let mut theme_b = Theme::default();
        theme_b.typography.body_font_size_pt = 20.0;
        theme_b.typography.heading_font_size_pt = 28.0;

        let mut buf_a: Vec<u8> = Vec::new();
        let mut buf_b: Vec<u8> = Vec::new();
        r.render(&doc, &theme_a, &mut buf_a, None).await.unwrap();
        r.render(&doc, &theme_b, &mut buf_b, None).await.unwrap();

        assert_ne!(buf_a, buf_b);

        let pdf_a = lopdf::Document::load_mem(&buf_a).expect("lopdf a");
        let pdf_b = lopdf::Document::load_mem(&buf_b).expect("lopdf b");
        let content_a = first_page_content(&pdf_a);
        let content_b = first_page_content(&pdf_b);
        assert_ne!(
            content_a, content_b,
            "page content streams must differ when typography changes"
        );
    }

    fn first_page_content(pdf: &lopdf::Document) -> Vec<u8> {
        let pages = pdf.get_pages();
        let (_, page_id) = pages.into_iter().next().expect("at least one page");
        pdf.get_page_content(page_id).expect("page content")
    }

    /// Collects the raw glyph-id bytes from every `Tj`/`TJ` operand on
    /// the given page. Printpdf emits text for custom TTF fonts as
    /// `<gid_hex>Tj`, so contiguous words show up as contiguous u16 BE
    /// pairs. That makes presence/absence checks a byte-substring
    /// operation.
    fn page_gid_bytes(pdf: &lopdf::Document, page_number: u32) -> Vec<u8> {
        let pages = pdf.get_pages();
        let page_id = pages.get(&page_number).copied().expect("page exists");
        let content = pdf
            .get_and_decode_page_content(page_id)
            .expect("decode page content");
        let mut flat: Vec<u8> = Vec::new();
        for op in &content.operations {
            if op.operator == "Tj" || op.operator == "TJ" {
                for operand in &op.operands {
                    collect_text_bytes(&mut flat, operand);
                }
            }
        }
        flat
    }

    fn collect_text_bytes(sink: &mut Vec<u8>, obj: &lopdf::Object) {
        match obj {
            lopdf::Object::String(bytes, _) => sink.extend_from_slice(bytes),
            lopdf::Object::Array(arr) => {
                for item in arr {
                    collect_text_bytes(sink, item);
                }
            }
            _ => {}
        }
    }

    /// Encodes `text` as the big-endian glyph-id byte sequence the
    /// renderer would emit for `font`. Returns empty for empty input.
    fn gid_signature(face: &Face<'_>, text: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(text.len() * 2);
        for c in text.chars() {
            let gid = face
                .glyph_index(c)
                .map_or(0_u16, |g| g.0);
            bytes.push((gid >> GID_BYTE_SHIFT) as u8);
            bytes.push((gid & GID_LOW_BYTE_MASK) as u8);
        }
        bytes
    }

    const GID_BYTE_SHIFT: u16 = 8;
    const GID_LOW_BYTE_MASK: u16 = 0xFF;

    fn contains_gid_signature(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() || haystack.len() < needle.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[tokio::test]
    async fn fifty_paragraph_fixture_paginates_fully() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Pagination Fixture").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Long Document").unwrap();
        let mut section = Section::new(heading);
        for i in 0..FIFTY_PARA_COUNT {
            let text = format!(
                "PARAGRAPHMARKER{i:02} this line fills a paragraph with filler words to give the wrapper real work"
            );
            section.push(Block::Paragraph(Paragraph::plain(text).unwrap()));
        }
        doc.push_section(section);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");
        assert!(
            outcome.page_count > 1,
            "50-paragraph fixture must paginate beyond one page, got {}",
            outcome.page_count
        );

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let mut all: Vec<u8> = Vec::new();
        for pn in 1..=u32::try_from(outcome.page_count).unwrap() {
            all.extend(page_gid_bytes(&parsed, pn));
        }

        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        for i in 0..FIFTY_PARA_COUNT {
            let marker = format!("PARAGRAPHMARKER{i:02}");
            let sig = gid_signature(&face, &marker);
            assert!(
                contains_gid_signature(&all, &sig),
                "paragraph {i} missing from rendered output — silent truncation"
            );
        }
    }

    const FIFTY_PARA_COUNT: usize = 50;

    #[tokio::test]
    async fn explicit_page_break_forces_new_page() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Page Break").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Top").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(Paragraph::plain("BEFOREBREAK").unwrap()));
        section.push(Block::PageBreak);
        section.push(Block::Paragraph(Paragraph::plain("AFTERBREAK").unwrap()));
        doc.push_section(section);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");
        assert_eq!(
            outcome.page_count, 2,
            "explicit page break must produce exactly two pages"
        );

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let before = gid_signature(&face, "BEFOREBREAK");
        let after = gid_signature(&face, "AFTERBREAK");

        let p1 = page_gid_bytes(&parsed, 1);
        let p2 = page_gid_bytes(&parsed, 2);
        assert!(contains_gid_signature(&p1, &before), "page 1 missing BEFOREBREAK");
        assert!(!contains_gid_signature(&p1, &after), "page 1 leaked AFTERBREAK");
        assert!(contains_gid_signature(&p2, &after), "page 2 missing AFTERBREAK");
        assert!(!contains_gid_signature(&p2, &before), "page 2 leaked BEFOREBREAK");
    }

    #[tokio::test]
    async fn adversarial_single_paragraph_wraps_across_pages() {
        let r = renderer();
        let words: Vec<String> = (0..BIG_PARA_WORDS).map(|i| format!("wx{i:04}")).collect();
        let big = words.join(" ");

        let mut doc = PdfDocument::new(Metadata::new("Adversarial").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Huge").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(Paragraph::plain(big).unwrap()));
        doc.push_section(section);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");
        assert!(
            outcome.page_count > 1,
            "adversarial paragraph must span multiple pages, got {}",
            outcome.page_count
        );

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let mut all: Vec<u8> = Vec::new();
        for pn in 1..=u32::try_from(outcome.page_count).unwrap() {
            all.extend(page_gid_bytes(&parsed, pn));
        }
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        for w in &words {
            let sig = gid_signature(&face, w);
            assert!(
                contains_gid_signature(&all, &sig),
                "word {w} missing — adversarial paragraph lost content across page boundary"
            );
        }
    }

    const BIG_PARA_WORDS: usize = 2000;

    /// Sweep test: no numeric literal (other than `0.0` / `1.0`) may
    /// appear outside `const` items in the render layout path (this
    /// file and `layout.rs`). Style knobs belong in the theme TOML;
    /// physical constants belong in named `const` bindings. This is
    /// the T1.4 enforcement mechanism, extended by T1.6 to cover the
    /// split-out layout engine.
    #[test]
    fn no_magic_numbers_in_render_path() {
        let sources: &[(&str, &str)] = &[
            ("printpdf_renderer.rs", include_str!("printpdf_renderer.rs")),
            ("layout.rs", include_str!("layout.rs")),
        ];

        let re = regex::Regex::new(r"\b\d+(?:_\d+)*\.\d+(?:_\d+)*\b")
            .expect("regex compiles");

        let mut violations: Vec<String> = Vec::new();
        for (name, src) in sources {
            let cutoff = src.find("#[cfg(test)]").unwrap_or(src.len());
            let body = &src[..cutoff];
            for (ln, line) in body.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with("*")
                {
                    continue;
                }
                // Skip `const` items regardless of visibility
                // (`const`, `pub const`, `pub(super) const`, …).
                let without_vis = trimmed
                    .strip_prefix("pub(crate) ")
                    .or_else(|| trimmed.strip_prefix("pub(super) "))
                    .or_else(|| trimmed.strip_prefix("pub "))
                    .unwrap_or(trimmed);
                if without_vis.starts_with("const ") {
                    continue;
                }
                for m in re.find_iter(line) {
                    let s = m.as_str();
                    if s == "0.0" || s == "1.0" {
                        continue;
                    }
                    violations.push(format!("{name}:{}: {}  ({s})", ln + 1, line.trim()));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "magic numeric literals found outside const items — move them to the theme:\n{}",
            violations.join("\n")
        );
    }
}
