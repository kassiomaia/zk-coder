//! `printpdf`-backed implementation of
//! [`crate::core::documents::pdf::PdfRenderer`].
//!
//! Keep this module thin — the renderer is the only place the rest of
//! the backend touches `printpdf`. Layout, font metrics, and engine
//! quirks all stay here.

pub mod printpdf_renderer;

pub use printpdf_renderer::PrintPdfRenderer;
