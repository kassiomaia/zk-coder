//! Application-layer use cases for the PDF authoring tool.
//!
//! The three use cases mirror the tool surface:
//!
//! - **StartDocument** — creates a typed draft (handled directly by the
//!   `pdf.start` tool via the draft store; no orchestration needed).
//! - **AppendBlock** — maps a legacy [`PdfPageContent`] value onto a list
//!   of typed blocks. See [`adapter::page_to_blocks`].
//! - **Finalize** — builds a [`PdfDocument`] from a draft and renders it
//!   via a [`PdfRenderer`]. See [`finalize::build_document`] and
//!   [`finalize::render_draft`].
//!
//! This layer is the single boundary between the string-blob draft model
//! and the typed core. Keeping the adapter here (rather than inside the
//! renderer or inside the `Tool` impl) means the core stays engine-free
//! and the infrastructure `Tool` becomes a thin orchestrator.
//!
//! [`PdfPageContent`]: crate::infrastructure::tools::documents::shared::PdfPageContent
//! [`PdfDocument`]: crate::core::documents::pdf::PdfDocument
//! [`PdfRenderer`]: crate::core::documents::pdf::PdfRenderer

pub mod adapter;
pub mod finalize;
