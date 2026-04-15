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
pub mod section;

pub use block::{Align, Block, InlineRun, InlineStyle, Paragraph};
pub use document::PdfDocument;
pub use error::PdfModelError;
pub use geometry::{Margins, Orientation, PageGeometry, PageSize};
pub use metadata::{Lang, Metadata};
pub use section::{Heading, HeadingLevel, Section};
