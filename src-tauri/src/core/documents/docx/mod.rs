//! DOCX domain module.
//!
//! Pure value objects for describing a Word document's style (fonts,
//! palette, spacing, page geometry, decorations) plus a patch type that
//! the LLM can supply through the tool boundary. No docx-rust coupling.

pub mod theme;
pub mod theme_patch;

pub use theme::{
    Decorations, DocxTheme, Fonts, PageDefaults, Palette, PaperSize, Rgb, Spacing, Typography,
};
pub use theme_patch::{DocxThemeError, DocxThemePatch, DoubleOption};
