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
}
