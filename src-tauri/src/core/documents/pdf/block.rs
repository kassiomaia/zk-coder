//! Block-level elements: paragraphs, inline runs, and the `Block` enum.

use serde::{Deserialize, Serialize};

use super::error::PdfModelError;
use super::section::Heading;

/// Horizontal alignment applied to a paragraph.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// Flush-left (the default).
    #[default]
    Left,
    /// Flush-right.
    Right,
    /// Centred.
    Center,
    /// Full justification.
    Justify,
}

/// Inline formatting flags.
///
/// A `struct` of booleans rather than `bitflags` so the serde output is
/// self-describing and tests read naturally.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineStyle {
    /// Bold weight.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    /// Italic slant.
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    /// Monospace / inline-code style.
    #[serde(default, skip_serializing_if = "is_false")]
    pub code: bool,
}

impl InlineStyle {
    /// Style with all flags disabled.
    pub const PLAIN: Self = Self {
        bold: false,
        italic: false,
        code: false,
    };

    /// Shorthand for a bold-only style.
    #[must_use]
    pub const fn bold() -> Self {
        Self {
            bold: true,
            italic: false,
            code: false,
        }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(v: &bool) -> bool {
    // Signature is dictated by serde's `skip_serializing_if`, which
    // requires `fn(&T) -> bool`.
    !*v
}

/// A contiguous span of text that shares one [`InlineStyle`].
///
/// An optional link target turns the run into a hyperlink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineRun {
    text: String,
    /// Formatting flags applied to this run.
    #[serde(default)]
    pub style: InlineStyle,
    /// Optional hyperlink target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl InlineRun {
    /// Builds a new inline run with no style and no link.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyInlineRun`] if `text` is empty.
    pub fn new(text: impl Into<String>) -> Result<Self, PdfModelError> {
        let text = text.into();
        if text.is_empty() {
            return Err(PdfModelError::EmptyInlineRun);
        }
        Ok(Self {
            text,
            style: InlineStyle::PLAIN,
            link: None,
        })
    }

    /// Overrides the inline style.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_style(mut self, style: InlineStyle) -> Self {
        self.style = style;
        self
    }

    /// Attaches a link target.
    #[must_use]
    pub fn with_link(mut self, href: impl Into<String>) -> Self {
        self.link = Some(href.into());
        self
    }

    /// Returns the run text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Paragraph block.
///
/// Runs are ordered — the renderer walks them left to right. Construction
/// fails on empty run lists so every paragraph has at least one glyph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    runs: Vec<InlineRun>,
    /// Alignment applied when laying out lines.
    #[serde(default)]
    pub align: Align,
}

impl Paragraph {
    /// Builds a paragraph from an ordered list of runs.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyParagraph`] if `runs` is empty.
    pub fn new(runs: Vec<InlineRun>) -> Result<Self, PdfModelError> {
        if runs.is_empty() {
            return Err(PdfModelError::EmptyParagraph);
        }
        Ok(Self {
            runs,
            align: Align::default(),
        })
    }

    /// Convenience constructor: a single plain-style run.
    ///
    /// # Errors
    /// Propagates [`PdfModelError::EmptyInlineRun`] / [`PdfModelError::EmptyParagraph`].
    pub fn plain(text: impl Into<String>) -> Result<Self, PdfModelError> {
        Self::new(vec![InlineRun::new(text)?])
    }

    /// Overrides the horizontal alignment.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Returns the ordered runs.
    #[must_use]
    pub fn runs(&self) -> &[InlineRun] {
        &self.runs
    }
}

/// Block-level element.
///
/// M1 only ships the three variants required by the execution plan; richer
/// blocks (lists, tables, images, ...) land in M2 and extend this enum in
/// lock-step with the renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Paragraph of inline runs.
    Paragraph(Paragraph),
    /// Outline heading, also used to drive the PDF bookmark tree.
    Heading(Heading),
    /// Forces the next block onto a new page.
    PageBreak,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::documents::pdf::section::HeadingLevel;

    #[test]
    fn inline_run_rejects_empty_text() {
        assert_eq!(InlineRun::new(""), Err(PdfModelError::EmptyInlineRun));
    }

    #[test]
    fn paragraph_rejects_empty_runs() {
        assert_eq!(Paragraph::new(vec![]), Err(PdfModelError::EmptyParagraph));
    }

    #[test]
    fn paragraph_preserves_run_order() {
        let p = Paragraph::new(vec![
            InlineRun::new("first ").unwrap(),
            InlineRun::new("second ").unwrap().with_style(InlineStyle::bold()),
            InlineRun::new("third").unwrap(),
        ])
        .unwrap();
        let texts: Vec<&str> = p.runs().iter().map(InlineRun::text).collect();
        assert_eq!(texts, vec!["first ", "second ", "third"]);
        assert!(p.runs()[1].style.bold);
    }

    #[test]
    fn paragraph_serde_roundtrip_preserves_order() {
        let p = Paragraph::new(vec![
            InlineRun::new("a").unwrap(),
            InlineRun::new("b").unwrap(),
            InlineRun::new("c").unwrap(),
        ])
        .unwrap()
        .with_align(Align::Justify);
        let json = serde_json::to_string(&p).unwrap();
        let back: Paragraph = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        let texts: Vec<&str> = back.runs().iter().map(InlineRun::text).collect();
        assert_eq!(texts, vec!["a", "b", "c"]);
    }

    #[test]
    fn page_break_serde_roundtrip() {
        let block = Block::PageBreak;
        let json = serde_json::to_string(&block).unwrap();
        assert_eq!(json, r#"{"type":"page_break"}"#);
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn block_paragraph_serde_roundtrip() {
        let block = Block::Paragraph(Paragraph::plain("hello").unwrap());
        let json = serde_json::to_string(&block).unwrap();
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn block_heading_serde_roundtrip() {
        let block = Block::Heading(Heading::new(HeadingLevel::new(2).unwrap(), "H2").unwrap());
        let json = serde_json::to_string(&block).unwrap();
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block, back);
    }

    #[test]
    fn inline_style_skips_defaults_in_serde() {
        let run = InlineRun::new("x").unwrap();
        let json = serde_json::to_string(&run).unwrap();
        assert!(!json.contains("bold"));
        assert!(!json.contains("italic"));
    }
}
