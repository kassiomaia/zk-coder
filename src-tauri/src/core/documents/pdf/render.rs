//! Renderer trait plus its outcome / warning / error types.
//!
//! The trait is the only handle a use case has on the rendering engine.
//! Concrete implementations live in `infrastructure/tools/documents/pdf/`
//! and are free to pick any PDF library; callers see nothing but this
//! module.

use std::io::Write;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::document::PdfDocument;
pub use super::theme::Theme;

/// Progress notifications emitted while a document renders.
///
/// T1.2 does not wire progress events to the UI yet (T3.1 does). The
/// type is a placeholder so the trait signature is forward-compatible.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProgressSink;

/// Record of a font actually embedded in the output file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontRef {
    /// Family name as declared by the font `name` table.
    pub family: String,
    /// Number of raw bytes embedded.
    pub bytes_len: usize,
}

/// Non-fatal condition noted by the renderer.
///
/// Structural warnings surface to the UI so the user sees what was
/// degraded rather than discovering it at reading time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RenderWarning {
    /// One or more characters had no glyph in any loaded font.
    MissingGlyph {
        /// Snippet of the offending text.
        text: String,
    },
    /// An image was downscaled to fit the page.
    ImageScaledDown {
        /// Source pixel dimensions.
        original_px: (u32, u32),
        /// Rendered pixel dimensions.
        target_px: (u32, u32),
    },
    /// A table row did not fit on a single page and had to be clipped.
    LongTableRow {
        /// Zero-based row index.
        row_index: usize,
    },
}

/// Summary returned by a successful render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderOutcome {
    /// Number of pages written.
    pub page_count: usize,
    /// Fonts embedded in the output.
    pub embedded_fonts: Vec<FontRef>,
    /// Non-fatal warnings observed during layout.
    pub warnings: Vec<RenderWarning>,
    /// Total bytes written to the sink.
    pub byte_len: u64,
}

/// Fatal rendering error.
#[derive(Debug, Error)]
pub enum RenderError {
    /// I/O failure while writing to the sink.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// Font file could not be parsed or loaded.
    #[error("font error: {0}")]
    Font(String),
    /// Layout engine produced an unrecoverable inconsistency.
    #[error("layout error: {0}")]
    Layout(String),
    /// Underlying engine rejected the document.
    #[error("engine error: {0}")]
    Engine(String),
}

/// Contract for PDF renderers.
///
/// Implementations are injected into the application layer so use cases
/// are engine-agnostic. `sink` is a `dyn Write + Send` so the renderer
/// can be driven against a file, a buffer, or an in-memory cursor.
#[async_trait]
pub trait PdfRenderer: Send + Sync {
    /// Render `doc` using `theme` into `sink`.
    ///
    /// # Errors
    /// Returns a [`RenderError`] if fonts fail to load, the engine
    /// rejects the document, or writing to `sink` fails.
    async fn render(
        &self,
        doc: &PdfDocument,
        theme: &Theme,
        sink: &mut (dyn Write + Send),
        progress: Option<ProgressSink>,
    ) -> Result<RenderOutcome, RenderError>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_report_preset() {
        let t = Theme::default();
        assert!((t.typography.body_font_size_pt - 11.0).abs() < f32::EPSILON);
        assert!((t.typography.heading_font_size_pt - 18.0).abs() < f32::EPSILON);
        assert!(t.typography.line_leading > 1.0);
    }

    #[test]
    fn render_outcome_serde_roundtrip() {
        let o = RenderOutcome {
            page_count: 2,
            embedded_fonts: vec![FontRef {
                family: "LiberationSans".to_string(),
                bytes_len: 12_345,
            }],
            warnings: vec![RenderWarning::MissingGlyph {
                text: "☃".to_string(),
            }],
            byte_len: 42,
        };
        let json = serde_json::to_string(&o).unwrap();
        let back: RenderOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(o, back);
    }
}
