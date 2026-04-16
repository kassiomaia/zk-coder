//! Section and heading types.

use serde::{Deserialize, Serialize};

use super::block::Block;
use super::error::PdfModelError;

/// Outline level for a heading.
///
/// Constrained to 1..=6 (the same range as HTML and PDF outlines) through a
/// validating newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HeadingLevel(u8);

impl HeadingLevel {
    /// Builds a heading level.
    ///
    /// # Errors
    /// Returns [`PdfModelError::InvalidHeadingLevel`] if `level` is outside
    /// `1..=6`.
    pub fn new(level: u8) -> Result<Self, PdfModelError> {
        if !(1..=6).contains(&level) {
            return Err(PdfModelError::InvalidHeadingLevel(level));
        }
        Ok(Self(level))
    }

    /// Returns the numeric level.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Heading title plus outline level.
///
/// Drives both the visible section title and the PDF outline / bookmark
/// entry. `text` is guarded against empty strings because the renderer
/// needs a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    /// Outline level (1..=6).
    pub level: HeadingLevel,
    text: String,
}

impl Heading {
    /// Builds a heading.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyInlineRun`] if `text` is empty or
    /// whitespace-only.
    pub fn new(level: HeadingLevel, text: impl Into<String>) -> Result<Self, PdfModelError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(PdfModelError::EmptyInlineRun);
        }
        Ok(Self { level, text })
    }

    /// Returns the heading text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// A heading plus the ordered list of blocks that belong to it.
///
/// Sections are the unit the TOC is built from in M2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Heading attached to this section.
    pub heading: Heading,
    /// Blocks contained in the section, in layout order.
    #[serde(default)]
    pub blocks: Vec<Block>,
}

impl Section {
    /// Builds an empty section with the given heading.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(heading: Heading) -> Self {
        Self {
            heading,
            blocks: Vec::new(),
        }
    }

    /// Replaces the section's blocks.
    #[must_use]
    pub fn with_blocks(mut self, blocks: Vec<Block>) -> Self {
        self.blocks = blocks;
        self
    }

    /// Appends a block at the end of the section.
    pub fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::documents::pdf::block::Paragraph;

    #[test]
    fn heading_level_rejects_out_of_range() {
        assert!(matches!(
            HeadingLevel::new(0),
            Err(PdfModelError::InvalidHeadingLevel(0))
        ));
        assert!(matches!(
            HeadingLevel::new(7),
            Err(PdfModelError::InvalidHeadingLevel(7))
        ));
    }

    #[test]
    fn heading_level_accepts_1_through_6() {
        for level in 1..=6u8 {
            assert_eq!(HeadingLevel::new(level).unwrap().get(), level);
        }
    }

    #[test]
    fn heading_rejects_empty_text() {
        assert!(Heading::new(HeadingLevel::new(1).unwrap(), "   ").is_err());
    }

    #[test]
    fn section_preserves_block_order() {
        let mut s = Section::new(Heading::new(HeadingLevel::new(1).unwrap(), "Intro").unwrap());
        s.push(Block::Paragraph(Paragraph::plain("one").unwrap()));
        s.push(Block::PageBreak);
        s.push(Block::Paragraph(Paragraph::plain("two").unwrap()));
        assert_eq!(s.blocks.len(), 3);
        assert!(matches!(s.blocks[1], Block::PageBreak));
    }

    #[test]
    fn section_serde_roundtrip() {
        let s = Section::new(Heading::new(HeadingLevel::new(2).unwrap(), "H").unwrap())
            .with_blocks(vec![
                Block::Paragraph(Paragraph::plain("body").unwrap()),
                Block::PageBreak,
            ]);
        let json = serde_json::to_string(&s).unwrap();
        let back: Section = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
