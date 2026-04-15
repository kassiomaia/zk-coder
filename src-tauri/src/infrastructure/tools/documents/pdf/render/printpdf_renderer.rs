//! `PrintPdfRenderer`: the T1.2 renderer.
//!
//! Produces a single-page PDF from a [`PdfDocument`] using real glyph
//! advances pulled from a TTF file. T1.6 replaces the one-page cap with
//! a layout engine that paginates; T1.4 replaces the ad-hoc constants
//! here with a loaded theme.

use std::io::{Cursor, Write};
use std::path::Path;

use async_trait::async_trait;
use printpdf::{IndirectFontRef, Mm, PdfDocument as PrintPdfDoc, PdfLayerReference};
use ttf_parser::{name_id, Face};

use crate::core::documents::pdf::block::{Block, InlineRun, Paragraph};
use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::geometry::Orientation;
use crate::core::documents::pdf::render::{
    FontRef, PdfRenderer, ProgressSink, RenderError, RenderOutcome, Theme,
};
use crate::core::documents::pdf::section::Section;

/// PostScript points per millimetre (`72 / 25.4`).
const POINTS_PER_MM: f32 = 2.834_645_7;

/// Renderer backed by the `printpdf` crate and a single embedded TTF.
///
/// One font is enough for T1.2. T1.5 introduces the font cache with a
/// fallback chain.
pub struct PrintPdfRenderer {
    font_bytes: Vec<u8>,
    family_name: String,
}

impl PrintPdfRenderer {
    /// Builds a renderer from raw TTF / OTF bytes.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if `font_bytes` is not a readable
    /// font file.
    pub fn new(font_bytes: Vec<u8>) -> Result<Self, RenderError> {
        let family_name = {
            let face =
                Face::parse(&font_bytes, 0).map_err(|e| RenderError::Font(e.to_string()))?;
            read_family_name(&face).unwrap_or_else(|| "Unknown".to_string())
        };
        Ok(Self {
            font_bytes,
            family_name,
        })
    }

    /// Builds a renderer by reading a font file from disk.
    ///
    /// # Errors
    /// Returns [`RenderError::Io`] if the file cannot be read and
    /// [`RenderError::Font`] if it cannot be parsed.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RenderError> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::new(bytes)
    }

    /// Reads the family name declared by the loaded font.
    #[must_use]
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    /// Returns the raw font bytes embedded in the output.
    #[must_use]
    pub fn font_bytes(&self) -> &[u8] {
        &self.font_bytes
    }

    fn face(&self) -> Result<Face<'_>, RenderError> {
        Face::parse(&self.font_bytes, 0).map_err(|e| RenderError::Font(e.to_string()))
    }

    /// Returns `text` wrapped into lines that each fit within
    /// `max_width_pt` at `font_size_pt`, using real glyph advances.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if the loaded font cannot be
    /// reparsed (should not happen once `new` has succeeded).
    pub fn wrap_line(
        &self,
        text: &str,
        font_size_pt: f32,
        max_width_pt: f32,
    ) -> Result<Vec<String>, RenderError> {
        let face = self.face()?;
        Ok(wrap_with_face(&face, text, font_size_pt, max_width_pt))
    }

    /// Returns the width in points of `text` at `font_size_pt`.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if the loaded font cannot be
    /// reparsed.
    pub fn measure_text_pt(&self, text: &str, font_size_pt: f32) -> Result<f32, RenderError> {
        let face = self.face()?;
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

        let font = pdf_doc
            .add_external_font(Cursor::new(self.font_bytes.as_slice()))
            .map_err(|e| RenderError::Engine(e.to_string()))?;

        let face = self.face()?;
        let layer = pdf_doc.get_page(page_idx).get_layer(layer_idx);
        let params = LayoutParams::from_doc(doc, theme, page_width_mm, page_height_mm);
        layout_sections(&layer, &font, &face, &doc.sections, &params);

        let bytes = pdf_doc
            .save_to_bytes()
            .map_err(|e| RenderError::Engine(e.to_string()))?;
        sink.write_all(&bytes)?;
        sink.flush()?;
        let byte_len = u64::try_from(bytes.len())
            .map_err(|_| RenderError::Engine("output exceeds u64".to_string()))?;

        Ok(RenderOutcome {
            page_count: 1,
            embedded_fonts: vec![FontRef {
                family: self.family_name.clone(),
                bytes_len: self.font_bytes.len(),
            }],
            warnings: Vec::new(),
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

struct LayoutParams {
    usable_width_pt: f32,
    body_leading_mm: f32,
    heading_leading_mm: f32,
    block_gap_mm: f32,
    min_y_mm: f32,
    x_mm: f32,
    initial_y_mm: f32,
    body_size_pt: f32,
    heading_size_pt: f32,
}

impl LayoutParams {
    fn from_doc(doc: &PdfDocument, theme: &Theme, page_width_mm: f32, page_height_mm: f32) -> Self {
        let margin_left = doc.geometry.margins.left_mm();
        let margin_right = doc.geometry.margins.right_mm();
        let margin_top = doc.geometry.margins.top_mm();
        let margin_bottom = doc.geometry.margins.bottom_mm();

        let body_leading_mm = theme.body_font_size_pt * theme.line_leading / POINTS_PER_MM;
        let heading_leading_mm = theme.heading_font_size_pt * theme.line_leading / POINTS_PER_MM;
        Self {
            usable_width_pt: (page_width_mm - margin_left - margin_right) * POINTS_PER_MM,
            body_leading_mm,
            heading_leading_mm,
            block_gap_mm: body_leading_mm * theme.block_gap,
            min_y_mm: margin_bottom,
            x_mm: margin_left,
            initial_y_mm: page_height_mm - margin_top - heading_leading_mm,
            body_size_pt: theme.body_font_size_pt,
            heading_size_pt: theme.heading_font_size_pt,
        }
    }
}

fn paragraph_text(p: &Paragraph) -> String {
    p.runs().iter().map(InlineRun::text).collect()
}

fn layout_sections(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    face: &Face<'_>,
    sections: &[Section],
    params: &LayoutParams,
) {
    let mut cursor_y_mm = params.initial_y_mm;
    for section in sections {
        if !emit_heading(layer, font, face, section.heading.text(), params, &mut cursor_y_mm) {
            return;
        }
        cursor_y_mm -= params.block_gap_mm;
        for block in &section.blocks {
            let cont = match block {
                Block::Paragraph(p) => {
                    let text = paragraph_text(p);
                    emit_body(layer, font, face, &text, params, &mut cursor_y_mm)
                }
                Block::Heading(h) => {
                    emit_heading(layer, font, face, h.text(), params, &mut cursor_y_mm)
                }
                // T1.6 will turn this into a real new-page; T1.2 stops
                // at the first page boundary.
                Block::PageBreak => return,
            };
            if !cont {
                return;
            }
            cursor_y_mm -= params.block_gap_mm;
        }
    }
}

/// Emits `text` at the heading size. Returns `false` if the y cursor
/// dropped below the bottom margin and the caller should abort.
fn emit_heading(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    face: &Face<'_>,
    text: &str,
    params: &LayoutParams,
    cursor_y_mm: &mut f32,
) -> bool {
    for line in wrap_with_face(face, text, params.heading_size_pt, params.usable_width_pt) {
        if *cursor_y_mm < params.min_y_mm {
            return false;
        }
        layer.use_text(
            line,
            params.heading_size_pt,
            Mm(params.x_mm),
            Mm(*cursor_y_mm),
            font,
        );
        *cursor_y_mm -= params.heading_leading_mm;
    }
    true
}

/// Emits `text` at the body size. Returns `false` if the y cursor
/// dropped below the bottom margin and the caller should abort.
fn emit_body(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    face: &Face<'_>,
    text: &str,
    params: &LayoutParams,
    cursor_y_mm: &mut f32,
) -> bool {
    for line in wrap_with_face(face, text, params.body_size_pt, params.usable_width_pt) {
        if *cursor_y_mm < params.min_y_mm {
            return false;
        }
        layer.use_text(
            line,
            params.body_size_pt,
            Mm(params.x_mm),
            Mm(*cursor_y_mm),
            font,
        );
        *cursor_y_mm -= params.body_leading_mm;
    }
    true
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

fn read_family_name(face: &Face<'_>) -> Option<String> {
    face.names().into_iter().find_map(|n| {
        if n.name_id == name_id::FAMILY {
            n.to_string()
        } else {
            None
        }
    })
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

/// Greedy word-wrap keyed on a raw TTF / OTF buffer.
///
/// Exposed so the legacy `final.rs` path can share a single measurement
/// implementation until it is deleted in T1.3.
///
/// # Errors
/// Returns [`RenderError::Font`] if `font_bytes` is not a readable font.
pub fn wrap_greedy(
    font_bytes: &[u8],
    text: &str,
    font_size_pt: f32,
    max_width_pt: f32,
) -> Result<Vec<String>, RenderError> {
    let face = Face::parse(font_bytes, 0).map_err(|e| RenderError::Font(e.to_string()))?;
    Ok(wrap_with_face(&face, text, font_size_pt, max_width_pt))
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
    use crate::core::documents::pdf::block::{Block, Paragraph};
    use crate::core::documents::pdf::document::PdfDocument;
    use crate::core::documents::pdf::metadata::Metadata;
    use crate::core::documents::pdf::section::{Heading, HeadingLevel, Section};
    use std::path::PathBuf;

    fn font_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/pdf/fonts/LiberationSans-Regular.ttf")
    }

    fn renderer() -> PrintPdfRenderer {
        PrintPdfRenderer::from_path(font_path()).expect("bundled font loads")
    }

    fn fixture_doc() -> PdfDocument {
        let mut doc = PdfDocument::new(Metadata::new("T1.2 Fixture").unwrap());
        let heading = Heading::new(HeadingLevel::new(1).unwrap(), "Introduction").unwrap();
        let mut section = Section::new(heading);
        section.push(Block::Paragraph(
            Paragraph::plain("Hello from the typed PDF model.").unwrap(),
        ));
        doc.push_section(section);
        doc
    }

    #[test]
    fn renderer_loads_real_font() {
        let r = renderer();
        assert!(r.family_name().to_lowercase().contains("liberation"));
        assert!(r.font_bytes().len() > 1000);
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
        assert_eq!(one.len(), 1, "text must fit on one line when width ≥ full measure");
        let tight = r.wrap_line(text, 12.0, single / 4.0).unwrap();
        assert!(
            tight.len() >= 4,
            "with a quarter-width budget the wrap must emit at least 4 lines, got {}",
            tight.len()
        );
        let all_joined = tight.join(" ");
        for word in text.split_whitespace() {
            assert!(
                all_joined.contains(word),
                "wrapped output must preserve every word; missing {word}"
            );
        }
    }

    #[tokio::test]
    async fn render_fixture_is_valid_parseable_pdf() {
        let r = renderer();
        let doc = fixture_doc();
        let theme = Theme::default();
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &theme, &mut buf, None)
            .await
            .expect("render ok");

        assert_eq!(outcome.page_count, 1);
        assert_eq!(outcome.embedded_fonts.len(), 1);
        assert!(outcome.warnings.is_empty());
        assert!(outcome.byte_len > 0);
        assert_eq!(outcome.byte_len as usize, buf.len());

        assert!(
            buf.starts_with(b"%PDF-1."),
            "output must start with %PDF-1., got {:?}",
            &buf[..10.min(buf.len())]
        );
        assert!(
            buf.windows(5).any(|w| w == b"%%EOF"),
            "output must contain %%EOF trailer"
        );

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let pages = parsed.get_pages();
        assert_eq!(pages.len(), 1, "/Pages must contain exactly one page");

        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(|o| parsed.dereference(o).map(|(_, v)| v))
            .expect("trailer has Info reference");
        let info_dict = info.as_dict().expect("Info is a dict");
        let title_obj = info_dict.get(b"Title").expect("/Title present in Info");
        let title_bytes = match title_obj {
            lopdf::Object::String(s, _) => s.clone(),
            other => panic!("unexpected /Title object: {other:?}"),
        };
        let title_text = decode_pdf_string(&title_bytes);
        assert!(
            title_text.contains("T1.2 Fixture"),
            "/Title must match metadata, got {title_text:?}"
        );
    }

    /// Best-effort decoding of PDF string literals (`PDFDocEncoding` or
    /// `UTF-16BE` with a byte-order mark). Used only by tests.
    fn decode_pdf_string(bytes: &[u8]) -> String {
        if bytes.starts_with(&[0xFE, 0xFF]) {
            let u16s: Vec<u16> = bytes[2..]
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect();
            String::from_utf16_lossy(&u16s)
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}
