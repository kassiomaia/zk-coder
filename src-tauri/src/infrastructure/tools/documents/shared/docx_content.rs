//! DOCX-specific draft content types.
//!
//! Extracted from `draft_store.rs` so the styled/run-aware content
//! model does not bloat the shared store file.

use serde::{Deserialize, Serialize};

/// Text content of a paragraph or heading. Either a plain string (the
/// whole thing takes the paragraph-level style) or a sequence of
/// [`TextRun`]s when the LLM needs inline bold / italic / color / link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParagraphText {
    /// Single run, unstyled beyond the paragraph's role.
    Plain(String),
    /// Explicit run list. Produces one `<w:r>` per entry.
    Rich(Vec<TextRun>),
}

impl ParagraphText {
    /// Total character count across all runs. Used for quick previews
    /// and validation (e.g. empty-paragraph checks).
    #[must_use]
    pub fn text_len(&self) -> usize {
        match self {
            ParagraphText::Plain(s) => s.chars().count(),
            ParagraphText::Rich(runs) => runs.iter().map(|r| r.text.chars().count()).sum(),
        }
    }

    /// Flat string preview for list/summary endpoints.
    #[must_use]
    pub fn as_plain_string(&self) -> String {
        match self {
            ParagraphText::Plain(s) => s.clone(),
            ParagraphText::Rich(runs) => runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>().join(""),
        }
    }
}

/// Inline text span with optional formatting marks.
///
/// All formatting fields default to off. When `link` is set the run
/// renders as a hyperlink and adopts the palette's link color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TextRun {
    /// Literal text content.
    pub text: String,
    /// Bold weight.
    #[serde(default)]
    pub bold: bool,
    /// Italic slant.
    #[serde(default)]
    pub italic: bool,
    /// Single underline.
    #[serde(default)]
    pub underline: bool,
    /// Single strikethrough.
    #[serde(default)]
    pub strike: bool,
    /// Inline monospace + code background shading.
    #[serde(default)]
    pub code: bool,
    /// `RRGGBB` hex foreground override. Takes precedence over the
    /// palette's text color for this run only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// External URL. When set, the run is wrapped in a `<w:hyperlink>`
    /// and picks up the link color from the palette (unless `color`
    /// overrides it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// Paragraph alignment. Omit to inherit from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// Flush left (start).
    Left,
    /// Centered.
    Center,
    /// Flush right (end).
    Right,
    /// Justified — distribute space between words.
    Justify,
}

/// List item body plus optional nesting level (0-8, matching the
/// numbering definition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListItem {
    /// Item text (plain or rich).
    pub text: ParagraphText,
    /// Nesting level. `None` and `Some(0)` are equivalent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
}

/// Content a DOCX section can carry. One variant per block type that
/// the generator can emit.
///
/// `Eq` is intentionally not derived because `Image { width_mm: f32 }`
/// cannot satisfy `Eq`; value equality still works through `PartialEq`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DocxContent {
    /// Heading, levels 1-9.
    Heading {
        /// Outline level, 1-9.
        level: u8,
        /// Heading text.
        text: ParagraphText,
        /// Optional override; falls back to theme defaults.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alignment: Option<Alignment>,
        /// Insert a page break before the heading paragraph.
        #[serde(default)]
        page_break_before: bool,
    },
    /// Regular paragraph.
    Paragraph {
        /// Paragraph text.
        text: ParagraphText,
        /// Optional override; falls back to theme defaults.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alignment: Option<Alignment>,
        /// Optional semantic role: `body` (default), `quote`, or
        /// `code`. Each role picks up its own palette / padding.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<ParagraphRole>,
    },
    /// Bulleted list. Items can nest via [`ListItem::level`].
    BulletList {
        /// List items in order.
        items: Vec<ListItem>,
    },
    /// Numbered list. `start_at` biases the first item's number.
    NumberedList {
        /// List items in order.
        items: Vec<ListItem>,
        /// Starting number (default 1).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_at: Option<u32>,
    },
    /// Table with a header row.
    Table {
        /// Column headers.
        headers: Vec<ParagraphText>,
        /// Row cells. Every row must be `headers.len()` long.
        rows: Vec<Vec<ParagraphText>>,
        /// Optional caption rendered below the table.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    /// Embedded raster image (PNG / JPEG).
    Image {
        /// Absolute path to the image file on disk.
        path: String,
        /// Optional caption rendered below the image.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// Render width in millimetres. `None` falls back to 120 mm.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width_mm: Option<f32>,
    },
    /// Hard page break.
    PageBreak,
    /// Horizontal rule rendered as a bottom-bordered empty paragraph.
    HorizontalRule,
}

/// Semantic paragraph role. The generator maps this to palette /
/// shading / font choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphRole {
    /// Default body paragraph.
    Body,
    /// Indented, italic blockquote.
    Quote,
    /// Monospaced block with code-background shading.
    Code,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_serialises_as_string() {
        let pt = ParagraphText::Plain("hello".to_string());
        let j = serde_json::to_value(&pt).unwrap();
        assert_eq!(j, serde_json::json!("hello"));
    }

    #[test]
    fn rich_text_serialises_as_array() {
        let pt = ParagraphText::Rich(vec![TextRun {
            text: "hi".to_string(),
            bold: true,
            ..Default::default()
        }]);
        let j = serde_json::to_value(&pt).unwrap();
        assert_eq!(j[0]["text"], "hi");
        assert_eq!(j[0]["bold"], true);
    }

    #[test]
    fn text_len_covers_both_variants() {
        assert_eq!(ParagraphText::Plain("abc".to_string()).text_len(), 3);
        let rich = ParagraphText::Rich(vec![
            TextRun { text: "ab".to_string(), ..Default::default() },
            TextRun { text: "cd".to_string(), ..Default::default() },
        ]);
        assert_eq!(rich.text_len(), 4);
    }

    #[test]
    fn heading_content_deserialises_with_plain_text() {
        let j = serde_json::json!({
            "type": "Heading",
            "level": 1,
            "text": "Intro",
            "page_break_before": false
        });
        let c: DocxContent = serde_json::from_value(j).unwrap();
        match c {
            DocxContent::Heading { level, text, .. } => {
                assert_eq!(level, 1);
                assert_eq!(text.as_plain_string(), "Intro");
            }
            _ => panic!("expected Heading"),
        }
    }

    #[test]
    fn paragraph_with_rich_runs_deserialises() {
        let j = serde_json::json!({
            "type": "Paragraph",
            "text": [
                { "text": "Bold ", "bold": true },
                { "text": "and italic", "italic": true }
            ]
        });
        let c: DocxContent = serde_json::from_value(j).unwrap();
        match c {
            DocxContent::Paragraph { text, .. } => {
                assert_eq!(text.text_len(), "Bold and italic".len());
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn unknown_run_field_rejected() {
        let j = serde_json::json!({
            "text": "x",
            "bld": true
        });
        let r: Result<TextRun, _> = serde_json::from_value(j);
        assert!(r.is_err());
    }
}
