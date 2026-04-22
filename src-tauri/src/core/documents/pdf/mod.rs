//! Typed domain model for PDF documents.
//!
//! This module holds a pure, `serde`-able description of a PDF that callers
//! can build without depending on any rendering engine. The model makes
//! illegal states unrepresentable at the IPC boundary: coarse strings become
//! typed enums (`Align`, `PageSize`, `Block`), and invariants (non-empty
//! title, ordered runs) are enforced by constructors.
//!
//! The matching renderer lives in `infrastructure/tools/documents/pdf/` and
//! consumes these types through a trait — this module has zero knowledge of
//! `printpdf` or any other engine.

pub mod block;
pub mod document;
pub mod error;
pub mod geometry;
pub mod metadata;
pub mod render;
pub mod section;
pub mod theme;
pub mod theme_patch;

pub use block::{Align, Block, CoverBlock, ImageBlock, ImageFit, InlineRun, InlineStyle, Paragraph};
pub use document::PdfDocument;
pub use error::PdfModelError;
pub use geometry::{Margins, Orientation, PageGeometry, PageSize};
pub use metadata::{Lang, Metadata};
pub use render::{FontRef, PdfRenderer, ProgressSink, RenderError, RenderOutcome, RenderWarning};
pub use section::{Heading, HeadingLevel, Section};
pub use theme::{Decorations, Fonts, PageDefaults, Palette, Rgb, Spacing, Theme, TypographyScale};
pub use theme_patch::{
    DecorationsPatch, FontsPatch, PageDefaultsPatch, PalettePatch, RgbPatch, SpacingPatch,
    ThemeError, ThemePatch, TypographyPatch,
};
