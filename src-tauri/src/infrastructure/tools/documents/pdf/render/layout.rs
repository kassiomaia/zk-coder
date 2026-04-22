//! Page-break-aware layout engine.
//!
//! Owns the mutable state the [`PrintPdfRenderer`] needs while walking
//! blocks — current page/layer, cursor position, page count — and
//! knows how to start a new page when content overflows. Callers emit
//! content through the engine; the engine decides when a page break is
//! required so no caller needs to know about the bottom margin.
//!
//! All visual constants come from the loaded [`Theme`]; the sweep test
//! in `printpdf_renderer.rs` scans this file too and fails on any
//! numeric literal outside a named `const`.
//!
//! [`PrintPdfRenderer`]: super::PrintPdfRenderer

use printpdf::{Mm, PdfDocumentReference, PdfLayerReference, PdfPageIndex};

use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::render::Theme;
use crate::core::documents::pdf::theme::Palette;

/// PostScript points per millimetre (`72 / 25.4`).
pub(super) const POINTS_PER_MM: f32 = 2.834_645_7;

/// Layer-name prefix for pages added after the first. The initial page
/// is named by the renderer at document creation.
const LAYER_NAME_PREFIX: &str = "Layer ";

/// Precomputed layout dimensions in a mix of points and millimetres.
///
/// `printpdf` speaks millimetres at the API boundary (`Mm`) and points
/// for text sizes; we store both to avoid conversions in hot paths.
pub(super) struct LayoutParams {
    pub usable_width_pt: f32,
    pub body_leading_mm: f32,
    pub heading_leading_mm: f32,
    pub block_gap_mm: f32,
    pub min_y_mm: f32,
    pub x_mm: f32,
    pub initial_y_mm: f32,
    pub body_size_pt: f32,
    pub heading_size_pt: f32,
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub list_indent_mm: f32,
    pub block_padding_mm: f32,
    pub table_cell_padding_mm: f32,
    pub usable_width_mm: f32,
    pub palette: Palette,
}

impl LayoutParams {
    /// Builds layout parameters from the document geometry and the
    /// theme's typography/spacing knobs.
    pub fn from_doc(
        doc: &PdfDocument,
        theme: &Theme,
        page_width_mm: f32,
        page_height_mm: f32,
    ) -> Self {
        let margin_left = doc.geometry.margins.left_mm();
        let margin_right = doc.geometry.margins.right_mm();
        let margin_top = doc.geometry.margins.top_mm();
        let margin_bottom = doc.geometry.margins.bottom_mm();

        let body_size_pt = theme.typography.body_font_size_pt;
        let heading_size_pt = theme.typography.heading_font_size_pt;
        let line_leading = theme.typography.line_leading;
        let body_leading_mm = body_size_pt * line_leading / POINTS_PER_MM;
        let heading_leading_mm = heading_size_pt * line_leading / POINTS_PER_MM;
        let usable_width_mm = page_width_mm - margin_left - margin_right;
        Self {
            usable_width_pt: usable_width_mm * POINTS_PER_MM,
            body_leading_mm,
            heading_leading_mm,
            block_gap_mm: body_leading_mm * theme.spacing.block_gap,
            min_y_mm: margin_bottom,
            x_mm: margin_left,
            initial_y_mm: page_height_mm - margin_top - heading_leading_mm,
            body_size_pt,
            heading_size_pt,
            page_width_mm,
            page_height_mm,
            list_indent_mm: theme.spacing.list_indent_mm,
            block_padding_mm: theme.spacing.block_padding_mm,
            table_cell_padding_mm: theme.spacing.table_cell_padding_mm,
            usable_width_mm,
            palette: theme.palette,
        }
    }
}

/// One heading occurrence captured during layout.
///
/// The renderer translates these into PDF outline / bookmark entries
/// after layout completes. Capturing them in the engine — rather than
/// at the markdown/draft layer — guarantees the recorded `page` matches
/// the page the heading actually lands on after pagination.
#[derive(Debug, Clone)]
pub(super) struct OutlineMark {
    pub level: u8,
    pub title: String,
    /// printpdf's opaque handle for the page the heading lives on.
    pub page: PdfPageIndex,
    /// Zero-based human page number (matches `page_count - 1` at the
    /// moment of recording).
    pub page_number: usize,
}

/// Layout engine that owns the cursor and decides when to paginate.
///
/// A single engine is constructed per `render` call. The renderer gives
/// it the first page's layer; the engine appends pages to the same
/// [`PdfDocumentReference`] as needed.
pub(super) struct LayoutEngine<'a> {
    pdf: &'a PdfDocumentReference,
    params: LayoutParams,
    layers: Vec<PdfLayerReference>,
    pages: Vec<PdfPageIndex>,
    cursor_y_mm: f32,
    page_count: usize,
    outline: Vec<OutlineMark>,
}

impl<'a> LayoutEngine<'a> {
    /// Builds an engine seeded with the first page's layer and its
    /// printpdf page handle.
    pub fn new(
        pdf: &'a PdfDocumentReference,
        first_page: PdfPageIndex,
        first_layer: PdfLayerReference,
        params: LayoutParams,
    ) -> Self {
        let cursor = params.initial_y_mm;
        Self {
            pdf,
            params,
            layers: vec![first_layer],
            pages: vec![first_page],
            cursor_y_mm: cursor,
            page_count: 1,
            outline: Vec::new(),
        }
    }

    /// Records an outline mark anchored to the current page.
    ///
    /// `title` is trimmed for whitespace before storing; empty input
    /// after trimming is ignored so we never produce a blank bookmark.
    #[allow(clippy::expect_used)]
    pub fn record_outline(&mut self, level: u8, title: &str) {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return;
        }
        let page = *self
            .pages
            .last()
            .expect("LayoutEngine always holds at least one page handle");
        self.outline.push(OutlineMark {
            level,
            title: trimmed.to_string(),
            page,
            page_number: self.page_count - 1,
        });
    }

    /// Drains the collected outline marks in document order.
    pub fn take_outline(&mut self) -> Vec<OutlineMark> {
        std::mem::take(&mut self.outline)
    }

    /// Returns the layer for every page added so far, in page order.
    /// Used by the decorator pass to paint headers/footers after layout.
    pub fn layers(&self) -> &[PdfLayerReference] {
        &self.layers
    }

    /// Exposes the layout parameters by reference.
    pub const fn params(&self) -> &LayoutParams {
        &self.params
    }

    /// Replaces the active [`LayoutParams`] with a freshly-built set.
    ///
    /// Used by the renderer when it encounters a `Block::ThemeSwitch`:
    /// the incoming params reflect the merged (document theme + page
    /// patch) so typography, palette, spacing, and margins take effect
    /// from the next block onward. Cursor position and page list are
    /// untouched — only the layout knobs change.
    pub fn set_params(&mut self, params: LayoutParams) {
        self.params = params;
    }

    /// Current page's layer. A fresh clone is cheap (`PdfLayerReference`
    /// is a thin wrapper around indices). `layers` is seeded with one
    /// entry by [`Self::new`] and only ever grows, so `last()` is
    /// always `Some`.
    #[allow(clippy::expect_used)]
    pub fn current_layer(&self) -> PdfLayerReference {
        self.layers
            .last()
            .cloned()
            .expect("LayoutEngine always holds at least one layer")
    }

    /// y-coordinate where the next line of content is drawn, in mm.
    pub const fn cursor_y_mm(&self) -> f32 {
        self.cursor_y_mm
    }

    /// Number of pages added to the document so far.
    pub const fn page_count(&self) -> usize {
        self.page_count
    }

    /// Moves the cursor down by `dy_mm` (i.e. emits a line of height
    /// `dy_mm`). The caller is responsible for calling
    /// [`Self::ensure_space_for`] first when the line must not be split
    /// across pages.
    pub fn advance(&mut self, dy_mm: f32) {
        self.cursor_y_mm -= dy_mm;
    }

    /// Adds a new page, resets the cursor, and switches subsequent
    /// emissions to the new layer.
    pub fn break_page(&mut self) {
        let layer_name = format!("{LAYER_NAME_PREFIX}{}", self.page_count + 1);
        let (pi, li) = self.pdf.add_page(
            Mm(self.params.page_width_mm),
            Mm(self.params.page_height_mm),
            layer_name,
        );
        let layer = self.pdf.get_page(pi).get_layer(li);
        self.layers.push(layer);
        self.pages.push(pi);
        self.cursor_y_mm = self.params.initial_y_mm;
        self.page_count += 1;
    }

    /// Starts a new page when advancing by `dy_mm` would cross the
    /// bottom margin.
    pub fn ensure_space_for(&mut self, dy_mm: f32) {
        if self.cursor_y_mm - dy_mm < self.params.min_y_mm {
            self.break_page();
        }
    }
}
