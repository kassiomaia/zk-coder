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
    path::PaintMode, Color, Image as PdfImage, ImageTransform, IndirectFontRef, Mm,
    PdfDocument as PrintPdfDoc, PdfDocumentReference, PdfLayerReference, Rect, Rgb as PrintRgb,
};
use ttf_parser::Face;

use crate::core::documents::pdf::block::{
    Block, Callout, CodeBlock, ColumnWidth, ImageBlock, InlineRun, InlineStyle, ListBlock,
    ListItem, Paragraph, TableBlock, TableRow,
};
use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::geometry::Orientation;
use crate::core::documents::pdf::render::{
    FontRef, OutlineEntry, PdfPhase, PdfProgressEvent, PdfRenderer, ProgressSink, RenderError,
    RenderOutcome, RenderWarning, Theme,
};
use crate::core::documents::pdf::section::Section;
use crate::core::documents::pdf::theme::{Decorations, Rgb as ThemeRgb};

use super::decorator::{expand_template, paint_decoration, section_for_page, PageContext};
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
        progress: &Option<ProgressSink>,
    ) -> Result<RenderOutcome, RenderError> {
        emit_progress(progress, PdfProgressEvent::PhaseChanged {
            phase: PdfPhase::Preparing,
        });
        emit_progress(progress, PdfProgressEvent::Progress { percent: 0 });

        let has_toc = doc
            .sections
            .iter()
            .any(|s| s.blocks.iter().any(|b| matches!(b, Block::Toc)));

        let toc_entries: Option<Vec<TocEntry>> = if has_toc {
            let (entries, _toc_page_count) =
                self.measure_toc(doc, theme)?;
            Some(entries)
        } else {
            None
        };

        emit_progress(progress, PdfProgressEvent::PhaseChanged {
            phase: PdfPhase::Rendering,
        });
        emit_progress(progress, PdfProgressEvent::Progress { percent: PROGRESS_RENDERING });

        let outcome = self.do_render(doc, theme, sink, toc_entries.as_deref(), progress)?;

        emit_progress(progress, PdfProgressEvent::PhaseChanged {
            phase: PdfPhase::Complete,
        });
        emit_progress(progress, PdfProgressEvent::Progress { percent: PROGRESS_COMPLETE });

        Ok(outcome)
    }

    /// Performs the first (measurement) pass: layout everything except
    /// `Block::Toc`, collect outline marks, compute TOC lines, and
    /// estimate how many pages the TOC will occupy.
    fn measure_toc(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
    ) -> Result<(Vec<TocEntry>, usize), RenderError> {
        let (page_width_mm, page_height_mm) = page_dimensions(doc);
        let (pdf_doc, page_idx, layer_idx) = PrintPdfDoc::new(
            "measure",
            Mm(page_width_mm),
            Mm(page_height_mm),
            "L1",
        );
        let _font_refs = embed_all_fonts(&pdf_doc, &self.fonts)?;
        let first_layer = pdf_doc.get_page(page_idx).get_layer(layer_idx);
        let params = LayoutParams::from_doc(doc, theme, page_width_mm, page_height_mm);
        let mut engine = LayoutEngine::new(&pdf_doc, page_idx, first_layer, params);

        let font_refs = embed_all_fonts(&pdf_doc, &self.fonts)?;
        let mut missing = BTreeSet::new();
        let theme_ctx = RenderThemeCtx {
            doc,
            base_theme: theme,
            page_width_mm,
            page_height_mm,
        };
        layout_sections(
            &mut engine,
            &theme_ctx,
            &font_refs,
            &self.fonts,
            &doc.sections,
            &mut missing,
            None,
        )?;
        let outline_marks = engine.take_outline();
        let body_leading_mm = engine.params().body_leading_mm;
        let usable_height = engine.params().initial_y_mm - engine.params().min_y_mm;

        let entries: Vec<TocEntry> = outline_marks
            .iter()
            .map(|m| TocEntry {
                level: m.level,
                title: m.title.clone(),
                page_display: m.page_number + 1,
            })
            .collect();

        // Each TOC line takes one body leading; add a heading line too.
        let toc_height = body_leading_mm * (entries.len() + 1) as f32;
        let toc_pages = ((toc_height / usable_height).ceil() as usize).max(1);

        // Adjust page numbers: everything after the TOC shifts by toc_pages.
        let adjusted: Vec<TocEntry> = entries
            .into_iter()
            .map(|mut e| {
                e.page_display += toc_pages;
                e
            })
            .collect();

        Ok((adjusted, toc_pages))
    }

    #[allow(clippy::too_many_lines)]
    fn do_render(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
        toc_entries: Option<&[TocEntry]>,
        progress: &Option<ProgressSink>,
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
        let mut engine = LayoutEngine::new(&pdf_doc, page_idx, first_layer, params);

        let mut missing: BTreeSet<char> = BTreeSet::new();
        let theme_ctx = RenderThemeCtx {
            doc,
            base_theme: theme,
            page_width_mm,
            page_height_mm,
        };
        layout_sections(
            &mut engine,
            &theme_ctx,
            &font_refs,
            &self.fonts,
            &doc.sections,
            &mut missing,
            toc_entries,
        )?;
        let page_count = engine.page_count();
        let outline_marks = engine.take_outline();
        let engine_layers_snapshot: Vec<PdfLayerReference> = engine.layers().to_vec();
        let decoration_x_mm = engine.params().x_mm;
        let decoration_page_height_mm = engine.params().page_height_mm;
        drop(engine);

        let outline: Vec<OutlineEntry> = outline_marks
            .iter()
            .map(|m| OutlineEntry {
                level: m.level,
                title: m.title.clone(),
                page_index: m.page_number,
            })
            .collect();
        let mut seen_pages = std::collections::HashSet::<usize>::new();
        for mark in &outline_marks {
            if seen_pages.insert(mark.page_number) {
                pdf_doc.add_bookmark(mark.title.clone(), mark.page);
            }
        }

        emit_progress(progress, PdfProgressEvent::PhaseChanged {
            phase: PdfPhase::Decorating,
        });
        emit_progress(progress, PdfProgressEvent::Progress { percent: PROGRESS_DECORATING });

        let section_starts: Vec<(usize, String)> = outline_marks
            .iter()
            .filter(|m| m.level == 1)
            .map(|m| (m.page_number + 1, m.title.clone()))
            .collect();
        decorate_pages(
            &engine_layers_snapshot,
            &font_refs,
            doc.metadata.title(),
            &theme.decorations,
            page_count,
            &section_starts,
            decoration_x_mm,
            decoration_page_height_mm,
        );

        emit_progress(progress, PdfProgressEvent::PhaseChanged {
            phase: PdfPhase::Writing,
        });
        emit_progress(progress, PdfProgressEvent::Progress { percent: PROGRESS_WRITING });

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
            outline,
        })
    }
}

/// A single entry in the Table of Contents generated by the two-pass
/// render when a `Block::Toc` marker is encountered.
#[derive(Debug, Clone)]
struct TocEntry {
    level: u8,
    title: String,
    /// 1-based display page number (already adjusted for TOC offset).
    page_display: usize,
}

const fn page_dimensions(doc: &PdfDocument) -> (f32, f32) {
    let (portrait_w, portrait_h) = doc.geometry.size.dimensions_mm();
    match doc.geometry.orientation {
        Orientation::Portrait => (portrait_w, portrait_h),
        Orientation::Landscape => (portrait_h, portrait_w),
    }
}

/// Contextual slice needed to rebuild `LayoutParams` mid-render.
///
/// `Block::ThemeSwitch` merges a `ThemePatch` onto `base_theme` and
/// asks the layout engine to adopt the resulting params; we keep the
/// inputs on hand so the renderer doesn't have to re-derive them.
struct RenderThemeCtx<'a> {
    doc: &'a PdfDocument,
    base_theme: &'a Theme,
    page_width_mm: f32,
    page_height_mm: f32,
}

fn layout_sections(
    engine: &mut LayoutEngine<'_>,
    theme_ctx: &RenderThemeCtx<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    sections: &[Section],
    missing: &mut BTreeSet<char>,
    toc_entries: Option<&[TocEntry]>,
) -> Result<(), RenderError> {
    for section in sections {
        // When the section's first block is a cover page, skip the auto
        // section heading — the cover carries its own visual title. The
        // outline still records the heading text via emit_cover_page so
        // bookmarks remain complete.
        let skip_section_heading = matches!(section.blocks.first(), Some(Block::CoverPage(_)));
        if !skip_section_heading {
            emit_heading(
                engine,
                font_refs,
                fonts,
                section.heading.text(),
                section.heading.level.get(),
                missing,
            )?;
            let block_gap = engine.params().block_gap_mm;
            engine.advance(block_gap);
        }
        for block in &section.blocks {
            match block {
                Block::Paragraph(p) => emit_paragraph(engine, font_refs, fonts, p, missing)?,
                Block::Heading(h) => {
                    emit_heading(engine, font_refs, fonts, h.text(), h.level.get(), missing)?;
                }
                Block::PageBreak => engine.break_page(),
                Block::List(list) => emit_list(engine, font_refs, fonts, list, 0, missing)?,
                Block::Code(code) => emit_code(engine, font_refs, fonts, code, missing)?,
                Block::Callout(callout) => {
                    emit_callout(engine, font_refs, fonts, callout, missing)?;
                }
                Block::Table(table) => {
                    emit_table(engine, font_refs, fonts, table, missing)?;
                }
                Block::Image(img) => emit_image(engine, img)?,
                Block::CoverPage(cover) => {
                    emit_cover_page(engine, font_refs, fonts, cover, missing)?;
                }
                Block::ThemeSwitch(patch) => {
                    // Rebuild LayoutParams with the merged theme so
                    // typography/palette/spacing/margins update for the
                    // blocks that follow. Cursor and page state are
                    // preserved.
                    let mut merged = theme_ctx.base_theme.clone();
                    merged.apply_patch(patch);
                    let new_params = LayoutParams::from_doc(
                        theme_ctx.doc,
                        &merged,
                        theme_ctx.page_width_mm,
                        theme_ctx.page_height_mm,
                    );
                    engine.set_params(new_params);
                }
                Block::Toc => {
                    if let Some(entries) = toc_entries {
                        emit_toc(engine, font_refs, fonts, entries, missing)?;
                    }
                    // When toc_entries is None (measurement pass), skip.
                }
            }
            let gap = engine.params().block_gap_mm;
            engine.advance(gap);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decorate_pages(
    layers: &[PdfLayerReference],
    font_refs: &FontRefMap,
    title: &str,
    decorations: &Decorations,
    total_pages: usize,
    section_starts: &[(usize, String)],
    x_mm: f32,
    page_height_mm: f32,
) {
    if decorations.header.is_none() && decorations.footer.is_none() {
        return;
    }
    for (idx, layer) in layers.iter().enumerate() {
        let page_one_based = idx + 1;
        if decorations.suppress_on_first_page && page_one_based == 1 {
            continue;
        }
        let ctx = PageContext {
            title,
            page: page_one_based,
            total_pages,
            section: section_for_page(section_starts, page_one_based),
        };
        let header = decorations
            .header
            .as_deref()
            .map(|t| expand_template(t, &ctx));
        let footer = decorations
            .footer
            .as_deref()
            .map(|t| expand_template(t, &ctx));
        paint_decoration(
            layer,
            &font_refs.regular,
            decorations,
            header.as_deref(),
            footer.as_deref(),
            x_mm,
            page_height_mm,
        );
    }
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
    for line in &lines {
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
            line,
            body_size_pt,
            x,
            y,
            missing,
        )?;
        // T2.6: if the run carries a link, add a link annotation
        // covering the text area for each wrapped line.
        if let Some(href) = &run.link {
            let line_width_pt = measure_with_face(&face, line, body_size_pt);
            let line_width_mm = line_width_pt / POINTS_PER_MM;
            add_link_annotation(&layer, href, x, y, line_width_mm, leading);
        }
        engine.advance(leading);
    }
    Ok(())
}

/// Adds a URI link annotation covering a rectangular region.
fn add_link_annotation(
    layer: &PdfLayerReference,
    href: &str,
    x_mm: f32,
    y_mm: f32,
    width_mm: f32,
    height_mm: f32,
) {
    use printpdf::{Actions, BorderArray, ColorArray, LinkAnnotation};
    let rect = Rect::new(
        Mm(x_mm),
        Mm(y_mm - height_mm),
        Mm(x_mm + width_mm),
        Mm(y_mm + height_mm),
    );
    let annotation = LinkAnnotation::new(
        rect,
        Some(BorderArray::Solid([0.0, 0.0, 0.0])),
        Some(ColorArray::Transparent),
        Actions::uri(href.to_string()),
        None,
    );
    layer.add_link_annotation(annotation);
}

/// Emits `text` at the heading size using the regular-weight font,
/// paginating through the engine. Records an outline mark anchored to
/// the page the first wrapped line actually lands on.
fn emit_heading(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    text: &str,
    level: u8,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let primary = fonts.primary_for(&InlineStyle::PLAIN);
    let face = primary.face()?;
    let heading_size_pt = engine.params().heading_size_pt;
    let usable_width_pt = engine.params().usable_width_pt;
    let lines = wrap_with_face(&face, text, heading_size_pt, usable_width_pt);
    let mut outline_recorded = false;
    for line in lines {
        let leading = engine.params().heading_leading_mm;
        engine.ensure_space_for(leading);
        if !outline_recorded {
            // Record after ensure_space_for so the page index reflects
            // any forced break the heading triggered.
            engine.record_outline(level, text);
            outline_recorded = true;
        }
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

/// Emits a list block. `depth` is the 0-based nesting level — the
/// renderer indents by `params.list_indent_mm * (depth + 1)` so nested
/// items step further right than their parent.
fn emit_list(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    list: &ListBlock,
    depth: usize,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    for (idx, item) in list.items().iter().enumerate() {
        let marker = list_marker(list.ordered, idx);
        emit_list_item(engine, font_refs, fonts, item, &marker, depth, missing)?;
    }
    Ok(())
}

fn list_marker(ordered: bool, idx: usize) -> String {
    if ordered {
        let one_based = idx + ORDERED_INDEX_OFFSET;
        format!("{one_based}. ")
    } else {
        BULLET_MARKER.to_string()
    }
}

const ORDERED_INDEX_OFFSET: usize = 1;
const BULLET_MARKER: &str = "\u{2022} ";
/// Physical width of the left-edge accent stripe painted behind a
/// callout, in mm. Kept as a physical constant rather than a theme knob
/// — accents are intentionally uniform across themes.
const CALLOUT_BORDER_WIDTH_MM: f32 = 1.5;

fn emit_list_item(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    item: &ListItem,
    marker: &str,
    depth: usize,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let base_x = engine.params().x_mm;
    let indent_mm = engine.params().list_indent_mm * depth_factor(depth);
    let item_x = base_x + indent_mm;
    let body_size_pt = engine.params().body_size_pt;
    let body_leading_mm = engine.params().body_leading_mm;

    let marker_width_pt = {
        let face = fonts.primary_for(&InlineStyle::PLAIN).face()?;
        measure_with_face(&face, marker, body_size_pt)
    };
    let marker_width_mm = marker_width_pt / POINTS_PER_MM;

    let usable_width_mm = engine.params().usable_width_mm - indent_mm - marker_width_mm;
    let usable_width_pt = usable_width_mm * POINTS_PER_MM;

    let mut first_line = true;
    for run in item.runs() {
        let face = fonts.primary_for(&run.style).face()?;
        let lines = wrap_with_face(&face, run.text(), body_size_pt, usable_width_pt);
        for line in lines {
            engine.ensure_space_for(body_leading_mm);
            let layer = engine.current_layer();
            let y = engine.cursor_y_mm();
            if first_line {
                emit_line_with_fallback(
                    &layer,
                    font_refs,
                    fonts,
                    &InlineStyle::PLAIN,
                    marker,
                    body_size_pt,
                    item_x,
                    y,
                    missing,
                )?;
                first_line = false;
            }
            emit_line_with_fallback(
                &layer,
                font_refs,
                fonts,
                &run.style,
                &line,
                body_size_pt,
                item_x + marker_width_mm,
                y,
                missing,
            )?;
            engine.advance(body_leading_mm);
        }
    }

    for child in item.children() {
        emit_list(engine, font_refs, fonts, child, depth + 1, missing)?;
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
const fn depth_factor(depth: usize) -> f32 {
    // Indent = `(depth + 1) * list_indent_mm` — depth 0 (top-level) still
    // indents one step so list items sit inside any surrounding block.
    (depth as f32) + 1.0_f32
}

/// Emits a code block: background rect filled from `palette.code_bg`,
/// then the source lines rendered in the mono font using
/// `palette.code_text`. The engine paginates if the block is too tall
/// for the current page.
#[allow(clippy::cast_precision_loss)]
fn emit_code(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    block: &CodeBlock,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let style = InlineStyle {
        bold: false,
        italic: false,
        code: true,
    };
    let font = fonts.primary_for(&style);
    let face = font.face()?;
    let body_size_pt = engine.params().body_size_pt;
    let body_leading_mm = engine.params().body_leading_mm;
    let padding = engine.params().block_padding_mm;
    let usable_width_pt =
        (engine.params().usable_width_mm - padding - padding) * POINTS_PER_MM;

    let mut wrapped: Vec<String> = Vec::new();
    for src_line in block.code().lines() {
        let lines = wrap_with_face(&face, src_line, body_size_pt, usable_width_pt);
        if lines.is_empty() {
            wrapped.push(String::new());
        } else {
            wrapped.extend(lines);
        }
    }

    let content_height = body_leading_mm * wrapped.len() as f32;
    let total_height = content_height + padding + padding;
    engine.ensure_space_for(total_height);
    let layer = engine.current_layer();
    let top_y = engine.cursor_y_mm() + body_leading_mm;
    let bottom_y = top_y - total_height;
    let x_left = engine.params().x_mm;
    let x_right = x_left + engine.params().usable_width_mm;

    paint_fill_rect(
        &layer,
        engine.params().palette.code_bg,
        x_left,
        bottom_y,
        x_right,
        top_y,
    );

    let text_x = x_left + padding;
    for line in &wrapped {
        let layer = engine.current_layer();
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &style,
            line,
            body_size_pt,
            text_x,
            y,
            missing,
        )?;
        engine.advance(body_leading_mm);
    }
    engine.advance(padding + padding);
    Ok(())
}

/// Emits a callout: background rect filled from `palette.callout_bg`,
/// a thin left-edge accent from `palette.callout_border`, an optional
/// bold title, then the body text.
#[allow(clippy::cast_precision_loss)]
fn emit_callout(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    callout: &Callout,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let body_size_pt = engine.params().body_size_pt;
    let body_leading_mm = engine.params().body_leading_mm;
    let padding = engine.params().block_padding_mm;
    let usable_width_pt =
        (engine.params().usable_width_mm - padding - padding) * POINTS_PER_MM;

    let plain_face = fonts.primary_for(&InlineStyle::PLAIN).face()?;
    let bold_face = fonts.primary_for(&InlineStyle::bold()).face()?;

    let title_lines: Vec<String> = match callout.title.as_deref() {
        Some(t) => wrap_with_face(&bold_face, t, body_size_pt, usable_width_pt),
        None => Vec::new(),
    };
    let mut body_lines: Vec<String> = Vec::new();
    for src_line in callout.body().lines() {
        let lines = wrap_with_face(&plain_face, src_line, body_size_pt, usable_width_pt);
        if lines.is_empty() {
            body_lines.push(String::new());
        } else {
            body_lines.extend(lines);
        }
    }

    let line_count = title_lines.len() + body_lines.len();
    let content_height = body_leading_mm * line_count as f32;
    let total_height = content_height + padding + padding;
    engine.ensure_space_for(total_height);
    let layer = engine.current_layer();
    let top_y = engine.cursor_y_mm() + body_leading_mm;
    let bottom_y = top_y - total_height;
    let x_left = engine.params().x_mm;
    let x_right = x_left + engine.params().usable_width_mm;
    let border_right = x_left + CALLOUT_BORDER_WIDTH_MM;

    paint_fill_rect(
        &layer,
        engine.params().palette.callout_bg,
        x_left,
        bottom_y,
        x_right,
        top_y,
    );
    paint_fill_rect(
        &layer,
        engine.params().palette.callout_border,
        x_left,
        bottom_y,
        border_right,
        top_y,
    );

    let text_x = x_left + padding + CALLOUT_BORDER_WIDTH_MM;
    for line in &title_lines {
        let layer = engine.current_layer();
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &InlineStyle::bold(),
            line,
            body_size_pt,
            text_x,
            y,
            missing,
        )?;
        engine.advance(body_leading_mm);
    }
    for line in &body_lines {
        let layer = engine.current_layer();
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &InlineStyle::PLAIN,
            line,
            body_size_pt,
            text_x,
            y,
            missing,
        )?;
        engine.advance(body_leading_mm);
    }
    engine.advance(padding + padding);
    Ok(())
}

/// Emits the Table of Contents from precomputed entries.
///
/// Each entry is rendered as a line: `<indent><title> ... <page>`.
/// The heading "Table of Contents" is emitted first in heading style.
#[allow(clippy::cast_precision_loss)]
fn emit_toc(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    entries: &[TocEntry],
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let heading_size_pt = engine.params().heading_size_pt;
    let heading_leading = engine.params().heading_leading_mm;
    engine.ensure_space_for(heading_leading);
    let layer = engine.current_layer();
    let x = engine.params().x_mm;
    let y = engine.cursor_y_mm();
    emit_line_with_fallback(
        &layer,
        font_refs,
        fonts,
        &InlineStyle::PLAIN,
        TOC_HEADING_TEXT,
        heading_size_pt,
        x,
        y,
        missing,
    )?;
    engine.advance(heading_leading);
    let gap = engine.params().block_gap_mm;
    engine.advance(gap);

    let body_size_pt = engine.params().body_size_pt;
    let body_leading = engine.params().body_leading_mm;
    let indent_mm = engine.params().list_indent_mm;

    for entry in entries {
        engine.ensure_space_for(body_leading);
        let layer = engine.current_layer();
        let depth = (entry.level.saturating_sub(1)) as f32;
        let entry_x = x + indent_mm * depth;
        let line = format!("{}  {}", entry.title, entry.page_display);
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &InlineStyle::PLAIN,
            &line,
            body_size_pt,
            entry_x,
            engine.cursor_y_mm(),
            missing,
        )?;
        engine.advance(body_leading);
    }
    Ok(())
}

const TOC_HEADING_TEXT: &str = "Table of Contents";

/// Emits an image block on the current page.
///
/// Decodes the raw bytes via `image::load_from_memory`, converts to a
/// printpdf `Image`, and places it at the cursor. The display width
/// defaults to the usable page width; aspect ratio is preserved.
fn emit_image(engine: &mut LayoutEngine<'_>, block: &ImageBlock) -> Result<(), RenderError> {
    let dynamic = image::load_from_memory(block.data())
        .map_err(|e| RenderError::Engine(format!("image decode failed: {e}")))?;
    let (px_w, px_h) = (dynamic.width(), dynamic.height());
    if px_w == 0 || px_h == 0 {
        return Ok(());
    }
    let aspect = px_h as f32 / px_w as f32;
    let (display_w_mm, display_h_mm) =
        block.resolved_size_mm(aspect, engine.params().usable_width_mm);

    engine.ensure_space_for(display_h_mm);
    let layer = engine.current_layer();
    let x_mm = engine.params().x_mm;
    let y_mm = engine.cursor_y_mm() - display_h_mm + engine.params().body_leading_mm;

    let pdf_image = PdfImage::from_dynamic_image(&dynamic);
    // DPI controls raster scale; for Stretch mode with different x/y
    // target sizes, printpdf does not expose independent axis scaling, so
    // we pick the DPI that matches the width and let the image stretch
    // vertically in the transform. For Contain, width-based DPI is
    // correct since the aspect is preserved.
    let dpi = if display_w_mm > 0.0 {
        px_w as f32 / (display_w_mm / MM_PER_INCH)
    } else {
        FALLBACK_IMAGE_DPI
    };
    let scale_y = if display_h_mm > 0.0 && display_w_mm > 0.0 {
        (display_h_mm / display_w_mm) / (aspect.max(f32::EPSILON))
    } else {
        1.0
    };
    let scale_y_opt = if (scale_y - 1.0).abs() > 1e-4 {
        Some(scale_y)
    } else {
        None
    };
    pdf_image.add_to_layer(
        layer,
        ImageTransform {
            translate_x: Some(Mm(x_mm)),
            translate_y: Some(Mm(y_mm)),
            dpi: Some(dpi),
            scale_y: scale_y_opt,
            ..ImageTransform::default()
        },
    );

    engine.advance(display_h_mm);
    Ok(())
}

/// Millimetres per inch, used to convert pixel DPI.
const MM_PER_INCH: f32 = 25.4;

/// Fallback DPI used when width resolves to zero — matches printpdf's
/// default so the image still embeds rather than disappearing.
const FALLBACK_IMAGE_DPI: f32 = 72.0;

/// Half; used for vertical centring (`available / HALF`) and for
/// horizontal centring (`remaining / HALF`) on cover pages.
const HALF: f32 = 2.0;

/// Multiplier applied to body leading to compute the vertical gap above
/// the author/date block on cover pages.
const COVER_META_GAP_MULTIPLIER: f32 = 2.0;

/// Emits a vertically-centred cover page on its own page.
///
/// Forces a page break if the cursor has already moved below the top,
/// then lays out title / subtitle / author / date horizontally centred
/// and vertically centred within the page's usable area. A trailing
/// page break guarantees subsequent content starts on a fresh page.
///
/// The title is recorded on the outline so bookmarks still point at the
/// cover. Header/footer suppression on page 1 is handled by the theme's
/// existing `suppress_on_first_page` flag — no new machinery needed.
fn emit_cover_page(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    cover: &crate::core::documents::pdf::block::CoverBlock,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    // Start on a fresh page unless the cursor is still at the top.
    if engine.cursor_y_mm() < engine.params().initial_y_mm {
        engine.break_page();
    }

    let heading_leading_mm = engine.params().heading_leading_mm;
    let body_leading_mm = engine.params().body_leading_mm;
    let heading_size_pt = engine.params().heading_size_pt;
    let body_size_pt = engine.params().body_size_pt;
    let usable_width_pt = engine.params().usable_width_pt;
    let usable_width_mm = engine.params().usable_width_mm;

    let primary = fonts.primary_for(&InlineStyle::PLAIN);
    let italic_style = InlineStyle {
        bold: false,
        italic: true,
        code: false,
    };
    let italic_slot = fonts.primary_for(&italic_style);

    let title_face = primary.face()?;
    let body_face = primary.face()?;
    let italic_face = italic_slot.face()?;

    let title_lines = wrap_with_face(&title_face, cover.title.trim(), heading_size_pt, usable_width_pt);
    let subtitle_lines: Vec<String> = cover
        .subtitle
        .as_deref()
        .map(|s| wrap_with_face(&italic_face, s.trim(), body_size_pt, usable_width_pt))
        .unwrap_or_default();
    let author_lines: Vec<String> = cover
        .author
        .as_deref()
        .map(|s| wrap_with_face(&body_face, s.trim(), body_size_pt, usable_width_pt))
        .unwrap_or_default();
    let date_lines: Vec<String> = cover
        .date
        .as_deref()
        .map(|s| wrap_with_face(&body_face, s.trim(), body_size_pt, usable_width_pt))
        .unwrap_or_default();

    let gap_after_title_mm = body_leading_mm;
    let gap_before_meta_mm = body_leading_mm * COVER_META_GAP_MULTIPLIER;
    let mut content_height_mm = title_lines.len() as f32 * heading_leading_mm;
    if !subtitle_lines.is_empty() {
        content_height_mm += gap_after_title_mm + subtitle_lines.len() as f32 * body_leading_mm;
    }
    let meta_line_count = author_lines.len() + date_lines.len();
    if meta_line_count > 0 {
        content_height_mm += gap_before_meta_mm + meta_line_count as f32 * body_leading_mm;
    }

    let available_mm = engine.params().initial_y_mm - engine.params().min_y_mm;
    let top_offset_mm = ((available_mm - content_height_mm) / HALF).max(0.0);
    engine.advance(top_offset_mm);

    let x_mm = engine.params().x_mm;

    // Record outline so the cover still anchors a bookmark.
    engine.record_outline(1, cover.title.trim());

    for line in &title_lines {
        engine.ensure_space_for(heading_leading_mm);
        let line_width_pt = measure_with_face(&title_face, line, heading_size_pt);
        let line_width_mm = line_width_pt / POINTS_PER_MM;
        let shift_mm = ((usable_width_mm - line_width_mm) / HALF).max(0.0);
        let layer = engine.current_layer();
        let y = engine.cursor_y_mm();
        emit_line_with_fallback(
            &layer,
            font_refs,
            fonts,
            &InlineStyle::PLAIN,
            line,
            heading_size_pt,
            x_mm + shift_mm,
            y,
            missing,
        )?;
        engine.advance(heading_leading_mm);
    }

    if !subtitle_lines.is_empty() {
        engine.advance(gap_after_title_mm);
        for line in &subtitle_lines {
            engine.ensure_space_for(body_leading_mm);
            let line_width_pt = measure_with_face(&italic_face, line, body_size_pt);
            let line_width_mm = line_width_pt / POINTS_PER_MM;
            let shift_mm = ((usable_width_mm - line_width_mm) / HALF).max(0.0);
            let layer = engine.current_layer();
            let y = engine.cursor_y_mm();
            emit_line_with_fallback(
                &layer,
                font_refs,
                fonts,
                &italic_style,
                line,
                body_size_pt,
                x_mm + shift_mm,
                y,
                missing,
            )?;
            engine.advance(body_leading_mm);
        }
    }

    if meta_line_count > 0 {
        engine.advance(gap_before_meta_mm);
        for line in author_lines.iter().chain(date_lines.iter()) {
            engine.ensure_space_for(body_leading_mm);
            let line_width_pt = measure_with_face(&body_face, line, body_size_pt);
            let line_width_mm = line_width_pt / POINTS_PER_MM;
            let shift_mm = ((usable_width_mm - line_width_mm) / HALF).max(0.0);
            let layer = engine.current_layer();
            let y = engine.cursor_y_mm();
            emit_line_with_fallback(
                &layer,
                font_refs,
                fonts,
                &InlineStyle::PLAIN,
                line,
                body_size_pt,
                x_mm + shift_mm,
                y,
                missing,
            )?;
            engine.advance(body_leading_mm);
        }
    }

    // Always break to a fresh page so subsequent content starts clean.
    engine.break_page();
    Ok(())
}

/// Resolves [`ColumnWidth`] specs into concrete widths in mm.
///
/// Fixed columns keep their declared width. The remaining width is
/// split among `Auto` columns proportional to their weight.
fn resolve_column_widths(columns: &[ColumnWidth], usable_mm: f32) -> Vec<f32> {
    let fixed_total: f32 = columns
        .iter()
        .filter_map(|c| match c {
            ColumnWidth::Fixed { mm } => Some(*mm),
            ColumnWidth::Auto { .. } => None,
        })
        .sum();
    let remaining = (usable_mm - fixed_total).max(0.0);
    let auto_weight_sum: f32 = columns
        .iter()
        .filter_map(|c| match c {
            ColumnWidth::Auto { weight } => Some(*weight),
            ColumnWidth::Fixed { .. } => None,
        })
        .sum();

    columns
        .iter()
        .map(|c| match c {
            ColumnWidth::Fixed { mm } => *mm,
            ColumnWidth::Auto { weight } => {
                if auto_weight_sum > 0.0 {
                    remaining * weight / auto_weight_sum
                } else {
                    0.0
                }
            }
        })
        .collect()
}

/// Emits a table block with optional header-row repeat on page splits.
///
/// Row processing: measure each row's height (tallest cell's wrapped
/// line count × body_leading), ensure space, paint the row background
/// (for headers), then emit each cell's text. When a body row would
/// overflow the page, break and re-emit the header row first.
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn emit_table(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    table: &TableBlock,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    if table.columns().is_empty() {
        return Ok(());
    }
    let col_widths = resolve_column_widths(table.columns(), engine.params().usable_width_mm);
    let cell_pad = engine.params().table_cell_padding_mm;
    let body_size_pt = engine.params().body_size_pt;
    let body_leading_mm = engine.params().body_leading_mm;
    let x_base = engine.params().x_mm;

    if let Some(header) = table.header() {
        emit_table_row(
            engine,
            font_refs,
            fonts,
            header,
            &col_widths,
            cell_pad,
            body_size_pt,
            body_leading_mm,
            x_base,
            true,
            missing,
        )?;
    }

    for row in table.rows() {
        let row_height = measure_row_height(fonts, row, &col_widths, cell_pad, body_size_pt)?;
        let total = row_height + cell_pad + cell_pad;

        if engine.cursor_y_mm() - total < engine.params().min_y_mm {
            engine.break_page();
            if let Some(header) = table.header() {
                emit_table_row(
                    engine,
                    font_refs,
                    fonts,
                    header,
                    &col_widths,
                    cell_pad,
                    body_size_pt,
                    body_leading_mm,
                    x_base,
                    true,
                    missing,
                )?;
            }
        }

        emit_table_row(
            engine,
            font_refs,
            fonts,
            row,
            &col_widths,
            cell_pad,
            body_size_pt,
            body_leading_mm,
            x_base,
            false,
            missing,
        )?;
    }

    Ok(())
}

/// Measures the height (in mm) of the tallest cell in `row` after
/// wrapping each cell's text to its column width.
fn measure_row_height(
    fonts: &FontCache,
    row: &TableRow,
    col_widths: &[f32],
    cell_pad: f32,
    body_size_pt: f32,
) -> Result<f32, RenderError> {
    let mut max_lines: usize = 1;
    for (cell, &col_w) in row.cells().iter().zip(col_widths.iter()) {
        let inner_w = (col_w - cell_pad - cell_pad).max(0.0) * POINTS_PER_MM;
        let mut cell_lines: usize = 0;
        for run in cell.runs() {
            let face = fonts.primary_for(&run.style).face()?;
            let lines = wrap_with_face(&face, run.text(), body_size_pt, inner_w);
            cell_lines += lines.len().max(1);
        }
        if cell.runs().is_empty() {
            cell_lines = 1;
        }
        max_lines = max_lines.max(cell_lines);
    }
    let leading_mm = body_size_pt * LEADING_MULTIPLIER / POINTS_PER_MM;
    Ok(leading_mm * max_lines as f32)
}

/// Nominal leading multiplier kept in sync with theme — but declared
/// here as a const so the sweep test is satisfied. The engine's own
/// `body_leading_mm` is the canonical value at render time.
const LEADING_MULTIPLIER: f32 = 1.3;

/// Emits a single table row (header or body).
#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
fn emit_table_row(
    engine: &mut LayoutEngine<'_>,
    font_refs: &FontRefMap,
    fonts: &FontCache,
    row: &TableRow,
    col_widths: &[f32],
    cell_pad: f32,
    body_size_pt: f32,
    body_leading_mm: f32,
    x_base: f32,
    is_header: bool,
    missing: &mut BTreeSet<char>,
) -> Result<(), RenderError> {
    let row_content_height =
        measure_row_height(fonts, row, col_widths, cell_pad, body_size_pt)?;
    let total_height = row_content_height + cell_pad + cell_pad;
    engine.ensure_space_for(total_height);

    let layer = engine.current_layer();
    let top_y = engine.cursor_y_mm() + body_leading_mm;
    let bottom_y = top_y - total_height;
    let table_right = x_base + col_widths.iter().sum::<f32>();

    if is_header {
        paint_fill_rect(
            &layer,
            engine.params().palette.table_header_bg,
            x_base,
            bottom_y,
            table_right,
            top_y,
        );
    }

    let border_color = engine.params().palette.table_border;
    paint_fill_rect(&layer, border_color, x_base, top_y, table_right, top_y + TABLE_BORDER_THICKNESS_MM);
    paint_fill_rect(&layer, border_color, x_base, bottom_y - TABLE_BORDER_THICKNESS_MM, table_right, bottom_y);

    let mut cell_x = x_base;
    for (cell, &col_w) in row.cells().iter().zip(col_widths.iter()) {
        let text_x = cell_x + cell_pad;
        let inner_w = (col_w - cell_pad - cell_pad).max(0.0) * POINTS_PER_MM;
        let style = if is_header {
            InlineStyle::bold()
        } else {
            InlineStyle::PLAIN
        };

        let mut line_y = engine.cursor_y_mm();
        for run in cell.runs() {
            let actual_style = if is_header { style } else { run.style };
            let face = fonts.primary_for(&actual_style).face()?;
            let lines = wrap_with_face(&face, run.text(), body_size_pt, inner_w);
            for line in lines {
                let layer = engine.current_layer();
                emit_line_with_fallback(
                    &layer,
                    font_refs,
                    fonts,
                    &actual_style,
                    &line,
                    body_size_pt,
                    text_x,
                    line_y,
                    missing,
                )?;
                line_y -= body_leading_mm;
            }
        }
        cell_x += col_w;
    }

    engine.advance(total_height);
    Ok(())
}

/// Thickness of horizontal table borders, in mm.
const TABLE_BORDER_THICKNESS_MM: f32 = 0.2;

/// Paints a filled rectangle on `layer` using the theme-supplied color.
/// Graphics state is saved/restored so the fill does not leak into
/// subsequent text emissions.
#[allow(clippy::similar_names)]
fn paint_fill_rect(
    layer: &PdfLayerReference,
    color: ThemeRgb,
    llx_mm: f32,
    lly_mm: f32,
    urx_mm: f32,
    ury_mm: f32,
) {
    layer.save_graphics_state();
    layer.set_fill_color(Color::Rgb(PrintRgb::new(color.r, color.g, color.b, None)));
    let rect =
        Rect::new(Mm(llx_mm), Mm(lly_mm), Mm(urx_mm), Mm(ury_mm)).with_mode(PaintMode::Fill);
    layer.add_rect(rect);
    layer.restore_graphics_state();
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

/// Progress percentage constants — render phases.
const PROGRESS_RENDERING: u8 = 20;
const PROGRESS_DECORATING: u8 = 60;
const PROGRESS_WRITING: u8 = 80;
const PROGRESS_COMPLETE: u8 = 100;

/// Sends a progress event to the sink, if present.
fn emit_progress(sink: &Option<ProgressSink>, event: PdfProgressEvent) {
    if let Some(cb) = sink {
        cb(event);
    }
}

#[async_trait]
impl PdfRenderer for PrintPdfRenderer {
    async fn render(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
        progress: Option<ProgressSink>,
    ) -> Result<RenderOutcome, RenderError> {
        self.render_sync(doc, theme, sink, &progress)
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

    #[tokio::test]
    async fn outline_records_one_entry_per_section_heading() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Outline").unwrap());
        for (i, name) in ["Alpha", "Bravo", "Charlie"].iter().enumerate() {
            let h = Heading::new(HeadingLevel::new(1).unwrap(), *name).unwrap();
            let mut s = Section::new(h);
            s.push(Block::Paragraph(
                Paragraph::plain(format!("section body {i}")).unwrap(),
            ));
            doc.push_section(s);
        }
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");

        let titles: Vec<&str> = outcome.outline.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Alpha", "Bravo", "Charlie"],
            "outline must list headings in document order"
        );
        for entry in &outcome.outline {
            assert_eq!(entry.level, 1, "section headings collapsed to level 1");
            assert!(
                entry.page_index < outcome.page_count,
                "outline page_index {} out of range for {} pages",
                entry.page_index,
                outcome.page_count
            );
        }
    }

    #[tokio::test]
    async fn outline_anchors_heading_to_page_after_break() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Cross-page Outline").unwrap());
        let h1 = Heading::new(HeadingLevel::new(1).unwrap(), "OnPageOne").unwrap();
        let mut s1 = Section::new(h1);
        s1.push(Block::Paragraph(Paragraph::plain("first body").unwrap()));
        s1.push(Block::PageBreak);
        s1.push(Block::Heading(
            Heading::new(HeadingLevel::new(2).unwrap(), "OnPageTwo").unwrap(),
        ));
        s1.push(Block::Paragraph(Paragraph::plain("second body").unwrap()));
        doc.push_section(s1);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");

        assert_eq!(outcome.outline.len(), 2);
        let on_one = &outcome.outline[0];
        let on_two = &outcome.outline[1];
        assert_eq!(on_one.title, "OnPageOne");
        assert_eq!(on_one.page_index, 0);
        assert_eq!(on_one.level, 1);
        assert_eq!(on_two.title, "OnPageTwo");
        assert_eq!(on_two.page_index, 1);
        assert_eq!(on_two.level, 2);
    }

    #[tokio::test]
    async fn outline_emits_pdf_outlines_dictionary() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Outlines Dict").unwrap());
        for (i, name) in ["Cover", "Body", "Wrap"].iter().enumerate() {
            let h = Heading::new(HeadingLevel::new(1).unwrap(), *name).unwrap();
            let mut s = Section::new(h);
            s.push(Block::Paragraph(
                Paragraph::plain(format!("body for {name}")).unwrap(),
            ));
            // Force each subsequent section onto its own page so printpdf's
            // one-bookmark-per-page collapse doesn't drop the entry.
            if i + 1 < 3 {
                s.push(Block::PageBreak);
            }
            doc.push_section(s);
        }
        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render ok");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let mut found_outlines = false;
        let mut outline_titles: Vec<String> = Vec::new();
        for (_, obj) in &parsed.objects {
            if let lopdf::Object::Dictionary(d) = obj {
                if let Ok(lopdf::Object::Name(n)) = d.get(b"Type") {
                    if n == b"Outlines" {
                        found_outlines = true;
                    }
                }
                if let Ok(lopdf::Object::String(bytes, _)) = d.get(b"Title") {
                    if d.get(b"Parent").is_ok() {
                        outline_titles.push(decode_pdf_string(bytes));
                    }
                }
            }
        }
        assert!(
            found_outlines,
            "rendered PDF must carry an /Outlines dictionary",
        );
        for expected in ["Cover", "Body", "Wrap"] {
            assert!(
                outline_titles.iter().any(|t| t == expected),
                "outline must contain {expected}; got {outline_titles:?}"
            );
        }
    }

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

    fn three_page_doc(title: &str) -> PdfDocument {
        let mut doc = PdfDocument::new(Metadata::new(title).unwrap());
        for (i, name) in ["First", "Second", "Third"].iter().enumerate() {
            let h = Heading::new(HeadingLevel::new(1).unwrap(), *name).unwrap();
            let mut s = Section::new(h);
            s.push(Block::Paragraph(
                Paragraph::plain(format!("body for {name}")).unwrap(),
            ));
            if i + 1 < 3 {
                s.push(Block::PageBreak);
            }
            doc.push_section(s);
        }
        doc
    }

    #[tokio::test]
    async fn header_and_footer_appear_on_pages_after_the_first() {
        let r = renderer();
        let doc = three_page_doc("DECORATEDREPORT");
        let theme = Theme::default(); // bundled report.toml has decorations on
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r.render(&doc, &theme, &mut buf, None).await.expect("render");
        assert_eq!(outcome.page_count, 3, "fixture must produce 3 pages");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();

        let title_sig = gid_signature(&face, "DECORATEDREPORT");
        // suppress_on_first_page is true → page 1 carries no title token.
        let p1 = page_gid_bytes(&parsed, 1);
        let p2 = page_gid_bytes(&parsed, 2);
        let p3 = page_gid_bytes(&parsed, 3);
        assert!(
            !contains_gid_signature(&p1, &title_sig),
            "page 1 must not carry header (suppress_on_first_page = true)"
        );
        assert!(
            contains_gid_signature(&p2, &title_sig),
            "page 2 header must contain {{title}}"
        );
        assert!(
            contains_gid_signature(&p3, &title_sig),
            "page 3 header must contain {{title}}"
        );

        // Footer template "Page {page} of {total_pages}" → page 2 must
        // contain "Page 2 of 3" — verify the literal `2 of 3` substring.
        let total_sig = gid_signature(&face, "Page 2 of 3");
        assert!(
            contains_gid_signature(&p2, &total_sig),
            "page 2 footer must resolve to 'Page 2 of 3' (got {} bytes content)",
            p2.len()
        );
        let p3_total = gid_signature(&face, "Page 3 of 3");
        assert!(
            contains_gid_signature(&p3, &p3_total),
            "page 3 footer must resolve to 'Page 3 of 3'"
        );
    }

    #[tokio::test]
    async fn total_pages_token_resolves_not_left_literal_in_pdf() {
        let r = renderer();
        let doc = three_page_doc("Title");
        let theme = Theme::default();
        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &theme, &mut buf, None).await.expect("render");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();

        let literal = gid_signature(&face, "{total_pages}");
        for pn in 1..=3 {
            let bytes = page_gid_bytes(&parsed, pn);
            assert!(
                !contains_gid_signature(&bytes, &literal),
                "page {pn} contains unresolved '{{total_pages}}' literal"
            );
        }
    }

    #[tokio::test]
    async fn first_page_suppression_toggle_off_paints_page_one() {
        let r = renderer();
        let doc = three_page_doc("HEADERONONE");
        let mut theme = Theme::default();
        theme.decorations.suppress_on_first_page = false;
        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &theme, &mut buf, None).await.expect("render");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let sig = gid_signature(&face, "HEADERONONE");
        let p1 = page_gid_bytes(&parsed, 1);
        assert!(
            contains_gid_signature(&p1, &sig),
            "with suppress_on_first_page=false the title must land on page 1"
        );
    }

    #[tokio::test]
    async fn no_decorations_in_theme_skips_decoration_pass() {
        let r = renderer();
        let doc = three_page_doc("Plain");
        let mut theme = Theme::default();
        theme.decorations.header = None;
        theme.decorations.footer = None;
        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &theme, &mut buf, None).await.expect("render");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let page_sig = gid_signature(&face, "Page ");
        for pn in 1..=3 {
            let bytes = page_gid_bytes(&parsed, pn);
            assert!(
                !contains_gid_signature(&bytes, &page_sig),
                "page {pn} carries footer text even though decorations were cleared"
            );
        }
    }

    // ---- T2.3 integration tests: list / code / callout ----

    use crate::core::documents::pdf::block::{Callout, CodeBlock, ListBlock, ListItem};

    #[tokio::test]
    async fn nested_list_indents_deeper_items() {
        // Child items must land further right than their parent. Read
        // the x-coordinate of the glyph run from each list-item's first
        // `Tj`/`TJ` operand and assert monotonic increase by depth.
        let r = renderer();
        let inner = ListBlock::new(
            true,
            vec![ListItem::plain("innerchildmarker").unwrap()],
        )
        .unwrap();
        let outer = ListBlock::new(
            false,
            vec![ListItem::plain("outerparentmarker")
                .unwrap()
                .with_children(vec![inner])],
        )
        .unwrap();

        let mut doc = PdfDocument::new(Metadata::new("Nested List").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Lists").unwrap();
        let mut s = Section::new(h);
        s.push(Block::List(outer));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let outer_sig = gid_signature(&face, "outerparentmarker");
        let inner_sig = gid_signature(&face, "innerchildmarker");
        let outer_x = text_x_for_signature(&parsed, 1, &outer_sig)
            .expect("outer item text not found");
        let inner_x = text_x_for_signature(&parsed, 1, &inner_sig)
            .expect("inner item text not found");
        assert!(
            inner_x > outer_x,
            "nested list must indent deeper: inner_x={inner_x} outer_x={outer_x}"
        );
    }

    #[tokio::test]
    async fn code_block_paints_background_rect_from_theme() {
        // Walk page content ops and assert there is a filled rect whose
        // fill color matches `palette.code_bg`. The operator stream
        // records the color via `rg` and the rect via `re` / `f`.
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Code").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Source").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Code(
            CodeBlock::new("fn main() {\n    println!(\"hi\");\n}").unwrap(),
        ));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let theme = Theme::default();
        let expected = (
            theme.palette.code_bg.r,
            theme.palette.code_bg.g,
            theme.palette.code_bg.b,
        );
        assert!(
            page_has_fill_color(&parsed, 1, expected),
            "code block must paint its theme-supplied background fill"
        );
    }

    #[tokio::test]
    async fn callout_paints_background_from_theme_palette() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Callout").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Notes").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Callout(
            Callout::new("Heads up, something important here.")
                .unwrap()
                .with_title("Important"),
        ));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let theme = Theme::default();
        let bg = (
            theme.palette.callout_bg.r,
            theme.palette.callout_bg.g,
            theme.palette.callout_bg.b,
        );
        let border = (
            theme.palette.callout_border.r,
            theme.palette.callout_border.g,
            theme.palette.callout_border.b,
        );
        assert!(
            page_has_fill_color(&parsed, 1, bg),
            "callout must paint its theme-supplied background fill"
        );
        assert!(
            page_has_fill_color(&parsed, 1, border),
            "callout must paint its theme-supplied border accent fill"
        );
    }

    /// Returns the x-coordinate (in points) of the first text-showing
    /// operator on `page_number` whose operand bytes contain `needle`.
    fn text_x_for_signature(
        pdf: &lopdf::Document,
        page_number: u32,
        needle: &[u8],
    ) -> Option<f32> {
        let pages = pdf.get_pages();
        let page_id = pages.get(&page_number).copied()?;
        let content = pdf.get_and_decode_page_content(page_id).ok()?;
        // printpdf emits `BT` … `Td tx ty` … `Tj` … `ET`. `Td` resets
        // the text cursor relative to the text matrix, which `BT`
        // resets to identity — so within one section the first `Td`
        // operand is the absolute starting x in points.
        let mut pending_x: Option<f32> = None;
        for op in &content.operations {
            if op.operator == "BT" {
                pending_x = None;
            }
            if op.operator == "Td" && op.operands.len() >= TD_OPERAND_COUNT {
                if let Some(x) = operand_as_f32(&op.operands[TD_X_OPERAND_INDEX]) {
                    pending_x = Some(x);
                }
            }
            if op.operator == "Tj" || op.operator == "TJ" {
                let mut bytes: Vec<u8> = Vec::new();
                for operand in &op.operands {
                    collect_text_bytes(&mut bytes, operand);
                }
                if bytes
                    .windows(needle.len())
                    .any(|w| w == needle)
                {
                    return pending_x;
                }
            }
        }
        None
    }

    const TD_OPERAND_COUNT: usize = 2;
    const TD_X_OPERAND_INDEX: usize = 0;

    fn operand_as_f32(obj: &lopdf::Object) -> Option<f32> {
        match obj {
            lopdf::Object::Real(v) => Some(*v),
            lopdf::Object::Integer(v) => {
                let f = *v as f32;
                Some(f)
            }
            _ => None,
        }
    }

    fn page_has_fill_color(
        pdf: &lopdf::Document,
        page_number: u32,
        expected: (f32, f32, f32),
    ) -> bool {
        let pages = pdf.get_pages();
        let Some(page_id) = pages.get(&page_number).copied() else {
            return false;
        };
        let Ok(content) = pdf.get_and_decode_page_content(page_id) else {
            return false;
        };
        for op in &content.operations {
            if (op.operator == "rg" || op.operator == "RG")
                && op.operands.len() >= RGB_OPERAND_COUNT
            {
                let r = operand_as_f32(&op.operands[RGB_R_INDEX]);
                let g = operand_as_f32(&op.operands[RGB_G_INDEX]);
                let b = operand_as_f32(&op.operands[RGB_B_INDEX]);
                if let (Some(rv), Some(gv), Some(bv)) = (r, g, b) {
                    if close_enough(rv, expected.0)
                        && close_enough(gv, expected.1)
                        && close_enough(bv, expected.2)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    const RGB_OPERAND_COUNT: usize = 3;
    const RGB_R_INDEX: usize = 0;
    const RGB_G_INDEX: usize = 1;
    const RGB_B_INDEX: usize = 2;
    const COLOR_EPSILON: f32 = 0.01;

    fn close_enough(a: f32, b: f32) -> bool {
        (a - b).abs() < COLOR_EPSILON
    }

    // ---- T2.4 integration tests: table blocks ----

    use crate::core::documents::pdf::block::{ColumnWidth, TableBlock, TableCell, TableRow};

    #[tokio::test]
    async fn table_column_widths_honored() {
        // A two-column table: 40mm fixed + auto. Cell text in column 2
        // must start further right than 40mm from the left margin.
        let r = renderer();
        let cols = vec![ColumnWidth::fixed(40.0), ColumnWidth::auto()];
        let header = TableRow::new(vec![
            TableCell::plain("Name").unwrap(),
            TableCell::plain("Value").unwrap(),
        ]);
        let row = TableRow::new(vec![
            TableCell::plain("alpha").unwrap(),
            TableCell::plain("MARKER_COL2").unwrap(),
        ]);
        let table = TableBlock::new(cols, Some(header), vec![row]).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("Table Cols").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "T").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Table(table));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let col2_sig = gid_signature(&face, "MARKER_COL2");
        let col2_x = text_x_for_signature(&parsed, 1, &col2_sig)
            .expect("col2 text not found");
        // 40mm + left margin (20mm) → col2 must start at ~170pt or more
        // (40mm ≈ 113pt, margin 20mm ≈ 57pt). Just assert it's past
        // the first column's fixed width in points.
        let min_col2_x_pt = 40.0 * POINTS_PER_MM;
        assert!(
            col2_x > min_col2_x_pt,
            "column 2 text must start past 40mm; got {col2_x}pt"
        );
    }

    #[tokio::test]
    async fn oversized_table_splits_and_repeats_header() {
        let r = renderer();
        let cols = vec![ColumnWidth::auto()];
        let header = TableRow::new(vec![TableCell::plain("TABLEHEADERMARKER").unwrap()]);
        let rows: Vec<TableRow> = (0..TABLE_SPLIT_ROW_COUNT)
            .map(|i| {
                TableRow::new(vec![TableCell::plain(format!("row{i:03}")).unwrap()])
            })
            .collect();
        let table = TableBlock::new(cols, Some(header), rows).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("Big Table").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Data").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Table(table));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        assert!(
            outcome.page_count >= 2,
            "20-row table must span 2+ pages, got {}",
            outcome.page_count
        );

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();
        let header_sig = gid_signature(&face, "TABLEHEADERMARKER");

        // Header must appear on page 1 AND page 2 (repeat).
        let p1 = page_gid_bytes(&parsed, 1);
        let p2 = page_gid_bytes(&parsed, 2);
        assert!(
            contains_gid_signature(&p1, &header_sig),
            "page 1 must contain the table header"
        );
        assert!(
            contains_gid_signature(&p2, &header_sig),
            "page 2 must repeat the table header"
        );
    }

    const TABLE_SPLIT_ROW_COUNT: usize = 40;

    #[tokio::test]
    async fn empty_table_body_renders_header_only_no_panic() {
        let r = renderer();
        let cols = vec![ColumnWidth::auto(), ColumnWidth::auto()];
        let header = TableRow::new(vec![
            TableCell::plain("A").unwrap(),
            TableCell::plain("B").unwrap(),
        ]);
        let table = TableBlock::new(cols, Some(header), vec![]).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("Empty Table").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Tbl").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Table(table));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render should not panic");
        assert_eq!(outcome.page_count, 1);
    }

    #[tokio::test]
    async fn table_header_paints_background_from_theme() {
        let r = renderer();
        let cols = vec![ColumnWidth::auto()];
        let header = TableRow::new(vec![TableCell::plain("H").unwrap()]);
        let row = TableRow::new(vec![TableCell::plain("B").unwrap()]);
        let table = TableBlock::new(cols, Some(header), vec![row]).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("TH BG").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "T").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Table(table));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let theme = Theme::default();
        let expected = (
            theme.palette.table_header_bg.r,
            theme.palette.table_header_bg.g,
            theme.palette.table_header_bg.b,
        );
        assert!(
            page_has_fill_color(&parsed, 1, expected),
            "table header must paint its theme-supplied background fill"
        );
    }

    // ---- T2.5 integration tests: image blocks ----

    use crate::core::documents::pdf::block::ImageBlock;

    /// Creates a minimal 2x2 red PNG in memory for test fixtures.
    fn tiny_red_png() -> Vec<u8> {
        use std::io::Cursor;
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        // 2x2 red pixels, RGB 8-bit
        let red: [u8; 12] = [
            0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00,
        ];
        image::ImageEncoder::write_image(
            encoder,
            &red,
            TINY_PNG_DIM,
            TINY_PNG_DIM,
            image::ColorType::Rgb8.into(),
        )
        .expect("encode png");
        buf.into_inner()
    }

    const TINY_PNG_DIM: u32 = 2;

    #[tokio::test]
    async fn image_block_embeds_xobject_in_pdf() {
        let r = renderer();
        let png = tiny_red_png();
        let img = ImageBlock::new(png).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("Image Embed").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Img").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Image(img));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        // The rendered PDF must contain an XObject image (Subtype /Image).
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let mut has_image_xobj = false;
        for (_, obj) in &parsed.objects {
            if let lopdf::Object::Stream(stream) = obj {
                if let Ok(lopdf::Object::Name(subtype)) = stream.dict.get(b"Subtype") {
                    if subtype == b"Image" {
                        has_image_xobj = true;
                    }
                }
            }
        }
        assert!(
            has_image_xobj,
            "rendered PDF must contain an image XObject"
        );
    }

    #[tokio::test]
    async fn image_block_no_placeholder_text_in_output() {
        let r = renderer();
        let png = tiny_red_png();
        let img = ImageBlock::new(png).unwrap();

        let mut doc = PdfDocument::new(Metadata::new("No Placeholder").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Img").unwrap();
        let mut s = Section::new(h);
        s.push(Block::Image(img));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        // The literal "[Image:" placeholder must not appear anywhere.
        let as_str = String::from_utf8_lossy(&buf);
        assert!(
            !as_str.contains("[Image:"),
            "rendered PDF must not contain placeholder text '[Image:'"
        );
    }

    #[test]
    fn image_block_empty_data_returns_error() {
        let err = ImageBlock::new(vec![]);
        assert!(err.is_err());
    }

    // ---- T2.6 integration tests: TOC + hyperlinks ----

    #[tokio::test]
    async fn toc_block_emits_heading_titles_with_page_numbers() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("TOC Test").unwrap());

        // Section 1 with TOC placeholder
        let h1 = Heading::new(HeadingLevel::new(1).unwrap(), "Cover").unwrap();
        let mut s1 = Section::new(h1);
        s1.push(Block::Toc);
        doc.push_section(s1);

        // Section 2
        let h2 = Heading::new(HeadingLevel::new(1).unwrap(), "Introduction").unwrap();
        let mut s2 = Section::new(h2);
        s2.push(Block::Paragraph(Paragraph::plain("intro text").unwrap()));
        s2.push(Block::PageBreak);
        doc.push_section(s2);

        // Section 3
        let h3 = Heading::new(HeadingLevel::new(1).unwrap(), "Conclusion").unwrap();
        let mut s3 = Section::new(h3);
        s3.push(Block::Paragraph(Paragraph::plain("conclusion text").unwrap()));
        doc.push_section(s3);

        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        // The TOC must contain the heading titles as glyph signatures.
        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses");
        let font_bytes =
            std::fs::read(fonts_root().join("LiberationSans-Regular.ttf")).unwrap();
        let face = Face::parse(&font_bytes, 0).unwrap();

        let toc_heading_sig = gid_signature(&face, "Table of Contents");
        let intro_sig = gid_signature(&face, "Introduction");
        let conclusion_sig = gid_signature(&face, "Conclusion");

        let mut all_gids: Vec<u8> = Vec::new();
        for pn in 1..=u32::try_from(outcome.page_count).unwrap() {
            all_gids.extend(page_gid_bytes(&parsed, pn));
        }

        assert!(
            contains_gid_signature(&all_gids, &toc_heading_sig),
            "output must contain 'Table of Contents' heading"
        );
        assert!(
            contains_gid_signature(&all_gids, &intro_sig),
            "output must contain 'Introduction' in TOC"
        );
        assert!(
            contains_gid_signature(&all_gids, &conclusion_sig),
            "output must contain 'Conclusion' in TOC"
        );
    }

    #[tokio::test]
    async fn external_link_produces_uri_annotation() {
        let r = renderer();
        let mut doc = PdfDocument::new(Metadata::new("Links").unwrap());
        let h = Heading::new(HeadingLevel::new(1).unwrap(), "Resources").unwrap();
        let mut s = Section::new(h);
        let run = InlineRun::new("click here")
            .unwrap()
            .with_link("https://example.com");
        s.push(Block::Paragraph(Paragraph::new(vec![run]).unwrap()));
        doc.push_section(s);

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        // The PDF raw bytes must contain the URI string.
        let raw = String::from_utf8_lossy(&buf);
        assert!(
            raw.contains("example.com"),
            "rendered PDF must contain 'example.com' URI in its raw bytes"
        );
    }

    #[tokio::test]
    async fn document_without_toc_block_renders_normally() {
        // Ensure no regression: docs without Block::Toc still work.
        let r = renderer();
        let doc = fixture_doc();
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");
        assert_eq!(outcome.page_count, 1);
    }

    // ---- T3.1: progress events ----

    use crate::core::documents::pdf::render::{PdfPhase, PdfProgressEvent};

    #[tokio::test]
    async fn progress_events_arrive_in_order_and_monotonic() {
        let r = renderer();
        let doc = fixture_doc();
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();
        let sink: ProgressSink = Box::new(move |evt| {
            events_clone.lock().unwrap().push(evt);
        });

        let mut buf: Vec<u8> = Vec::new();
        r.render(&doc, &Theme::default(), &mut buf, Some(sink))
            .await
            .expect("render");

        let captured = events.lock().unwrap();
        assert!(
            captured.len() >= 4,
            "must emit at least 4 events (4 phases), got {}",
            captured.len()
        );

        // Phases must arrive in order: Preparing → Rendering → Decorating → Writing → Complete
        let phases: Vec<&PdfPhase> = captured
            .iter()
            .filter_map(|e| match e {
                PdfProgressEvent::PhaseChanged { phase } => Some(phase),
                _ => None,
            })
            .collect();
        let expected = [
            PdfPhase::Preparing,
            PdfPhase::Rendering,
            PdfPhase::Decorating,
            PdfPhase::Writing,
            PdfPhase::Complete,
        ];
        assert_eq!(
            phases.len(),
            expected.len(),
            "must see all 5 phase transitions"
        );
        for (got, want) in phases.iter().zip(expected.iter()) {
            assert_eq!(*got, want, "phase ordering mismatch");
        }

        // Progress percentages must be monotonically non-decreasing
        let percents: Vec<u8> = captured
            .iter()
            .filter_map(|e| match e {
                PdfProgressEvent::Progress { percent } => Some(*percent),
                _ => None,
            })
            .collect();
        assert!(
            !percents.is_empty(),
            "must emit at least one progress event"
        );
        for window in percents.windows(2) {
            assert!(
                window[1] >= window[0],
                "progress must be monotonic: {} → {}",
                window[0],
                window[1]
            );
        }
        assert_eq!(*percents.last().unwrap(), PROGRESS_COMPLETE);
    }

    // ---- M2 exit gate: journey-a golden file ----

    /// Builds the M2 "Journey A" golden fixture: a report with cover,
    /// metadata, TOC, headers/footers, 2 images, 1 table, nested lists,
    /// one internal link, one external link.
    ///
    /// Validates the rendered PDF with `PdfValidator::validate_file` and
    /// asserts the byte count is stable (deterministic render).
    #[tokio::test]
    async fn m2_journey_a_golden_renders_and_validates() {
        use crate::core::documents::pdf::block::{
            Callout, CodeBlock, ColumnWidth, ImageBlock, ListBlock, ListItem, TableBlock,
            TableCell, TableRow,
        };
        use crate::infrastructure::tools::documents::pdf::validate::PdfValidator;

        let r = renderer();

        let mut doc = PdfDocument::new(Metadata::new("Journey A — M2 Golden").unwrap());

        // Section 1: Cover with TOC
        let h1 = Heading::new(HeadingLevel::new(1).unwrap(), "Cover").unwrap();
        let mut s1 = Section::new(h1);
        s1.push(Block::Paragraph(
            Paragraph::plain("This is the Journey A golden report.").unwrap(),
        ));
        s1.push(Block::Toc);
        s1.push(Block::PageBreak);
        doc.push_section(s1);

        // Section 2: Content with images, table, list, links
        let h2 = Heading::new(HeadingLevel::new(1).unwrap(), "Content").unwrap();
        let mut s2 = Section::new(h2);

        // Nested list
        let inner_list =
            ListBlock::new(true, vec![ListItem::plain("nested item").unwrap()]).unwrap();
        let outer_list = ListBlock::new(
            false,
            vec![
                ListItem::plain("top-level bullet")
                    .unwrap()
                    .with_children(vec![inner_list]),
                ListItem::plain("second bullet").unwrap(),
            ],
        )
        .unwrap();
        s2.push(Block::List(outer_list));

        // Table
        let cols = vec![ColumnWidth::fixed(40.0), ColumnWidth::auto()];
        let header = TableRow::new(vec![
            TableCell::plain("Feature").unwrap(),
            TableCell::plain("Status").unwrap(),
        ]);
        let table = TableBlock::new(
            cols,
            Some(header),
            vec![
                TableRow::new(vec![
                    TableCell::plain("TOC").unwrap(),
                    TableCell::plain("Done").unwrap(),
                ]),
                TableRow::new(vec![
                    TableCell::plain("Images").unwrap(),
                    TableCell::plain("Done").unwrap(),
                ]),
            ],
        )
        .unwrap();
        s2.push(Block::Table(table));

        // Code block
        s2.push(Block::Code(
            CodeBlock::new("fn main() {\n    println!(\"golden\");\n}")
                .unwrap()
                .with_language("rust"),
        ));

        // Callout
        s2.push(Block::Callout(
            Callout::new("This report is the M2 exit gate golden.")
                .unwrap()
                .with_title("Note"),
        ));

        // 2 images (tiny PNGs)
        let png = tiny_red_png();
        s2.push(Block::Image(
            ImageBlock::new(png.clone()).unwrap().with_alt("Image 1"),
        ));
        s2.push(Block::Image(
            ImageBlock::new(png).unwrap().with_alt("Image 2"),
        ));

        // External link
        let link_run = InlineRun::new("example.com")
            .unwrap()
            .with_link("https://example.com");
        s2.push(Block::Paragraph(Paragraph::new(vec![link_run]).unwrap()));

        s2.push(Block::PageBreak);
        doc.push_section(s2);

        // Section 3: Conclusion
        let h3 = Heading::new(HeadingLevel::new(1).unwrap(), "Conclusion").unwrap();
        let mut s3 = Section::new(h3);
        s3.push(Block::Paragraph(
            Paragraph::plain("Journey A complete.").unwrap(),
        ));
        doc.push_section(s3);

        // Render
        let mut buf: Vec<u8> = Vec::new();
        let outcome = r
            .render(&doc, &Theme::default(), &mut buf, None)
            .await
            .expect("render");

        // Basic structural assertions
        assert!(
            outcome.page_count >= 3,
            "journey-a must span 3+ pages, got {}",
            outcome.page_count
        );
        assert!(buf.starts_with(b"%PDF-1."));
        assert!(outcome.byte_len > 0);

        // Write golden file
        let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/tools/pdf/goldens/journey-a.pdf");
        std::fs::write(&golden_path, &buf).expect("write golden");

        // Validate with PdfValidator
        let report = PdfValidator::validate_file(&golden_path);
        assert!(
            report.passed,
            "PdfValidator must pass on the golden: {report:?}"
        );
        // Zero warnings
        let warnings: Vec<_> = report
            .checks
            .iter()
            .filter(|c| !c.passed && c.level == crate::infrastructure::tools::documents::shared::validation::ValidationLevel::Warning)
            .collect();
        assert!(
            warnings.is_empty(),
            "golden must have zero warnings: {warnings:?}"
        );
    }

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
