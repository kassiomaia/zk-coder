//! `printpdf`-backed implementation of
//! [`crate::core::documents::pdf::PdfRenderer`].
//!
//! Keep this module thin — the renderer is the only place the rest of
//! the backend touches `printpdf`. Layout, font metrics, and engine
//! quirks all stay here.

pub mod font_cache;
pub mod printpdf_renderer;
pub mod theme_loader;

pub use font_cache::{bundled_report_cache, FontCache, FontSlot, LoadedFont};
pub use printpdf_renderer::PrintPdfRenderer;
pub use theme_loader::{load_report_theme, parse_theme, ThemeLoadError};
