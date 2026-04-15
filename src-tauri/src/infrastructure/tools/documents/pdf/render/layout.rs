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

use printpdf::{Mm, PdfDocumentReference, PdfLayerReference};

use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::render::Theme;

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
        Self {
            usable_width_pt: (page_width_mm - margin_left - margin_right) * POINTS_PER_MM,
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
        }
    }
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
    cursor_y_mm: f32,
    page_count: usize,
}

impl<'a> LayoutEngine<'a> {
    /// Builds an engine seeded with the first page's layer.
    pub fn new(
        pdf: &'a PdfDocumentReference,
        first_layer: PdfLayerReference,
        params: LayoutParams,
    ) -> Self {
        let cursor = params.initial_y_mm;
        Self {
            pdf,
            params,
            layers: vec![first_layer],
            cursor_y_mm: cursor,
            page_count: 1,
        }
    }

    /// Exposes the layout parameters by reference.
    pub const fn params(&self) -> &LayoutParams {
        &self.params
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
