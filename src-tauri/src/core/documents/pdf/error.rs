//! Domain errors produced while validating PDF model inputs.

use thiserror::Error;

/// Errors raised when constructing or mutating a typed PDF value.
#[derive(Debug, Error, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum PdfModelError {
    /// A `Metadata` was built with an empty or whitespace-only title.
    #[error("metadata title must not be empty")]
    EmptyTitle,

    /// A margin value was non-finite or negative.
    #[error("margin value must be finite and non-negative, got {0}")]
    InvalidMargin(f32),

    /// A page dimension was non-finite or non-positive.
    #[error("page dimension must be finite and positive, got {0}")]
    InvalidPageDimension(f32),

    /// A heading level was outside the 1..=6 range.
    #[error("heading level must be between 1 and 6, got {0}")]
    InvalidHeadingLevel(u8),

    /// A paragraph was built with no inline runs.
    #[error("paragraph must contain at least one inline run")]
    EmptyParagraph,

    /// An inline run text was empty.
    #[error("inline run text must not be empty")]
    EmptyInlineRun,

    /// A list block was constructed with no items.
    #[error("list block must contain at least one item")]
    EmptyList,

    /// A list item was constructed with no inline runs.
    #[error("list item must contain at least one inline run")]
    EmptyListItem,

    /// A code block was constructed with empty source.
    #[error("code block must not be empty")]
    EmptyCodeBlock,

    /// A callout was constructed with empty body text.
    #[error("callout body must not be empty")]
    EmptyCallout,

    /// A table was constructed with no columns.
    #[error("table must have at least one column")]
    EmptyTableColumns,

    /// An image block was constructed with empty data.
    #[error("image data must not be empty")]
    EmptyImageData,

    /// A table row had a cell count that differs from the column count.
    #[error("table row has {cells} cells but table defines {columns} columns")]
    TableCellCountMismatch {
        /// Number of cells the row provides.
        cells: usize,
        /// Number of columns the table declares.
        columns: usize,
    },
}
