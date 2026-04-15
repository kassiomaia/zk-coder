//! Maps the legacy [`PdfPageContent`] input surface onto typed blocks.
//!
//! The draft store still speaks the string-blob enum that the tool JSON
//! schema advertises, but the renderer only understands the typed
//! [`Block`] model from the core. This module is the single place where
//! that translation happens so the core remains engine-free and the
//! renderer remains content-type-free.
//!
//! Markdown support is deliberately minimal — it recognizes `#`, `##`,
//! and `###` headings plus blank-line-separated paragraphs. That is
//! enough to let agents emit typed-looking structure through the
//! existing tool surface while richer blocks (lists, tables, code) ride
//! in on M2.

use crate::core::documents::pdf::block::{Block, Paragraph};
use crate::core::documents::pdf::error::PdfModelError;
use crate::core::documents::pdf::section::{Heading, HeadingLevel};
use crate::infrastructure::tools::documents::shared::PdfPageContent;

/// Translates a single page of draft content into typed blocks.
///
/// Output order matches source order, so the renderer lays blocks out
/// top-to-bottom as the agent emitted them. Empty input produces a
/// single non-breaking-space paragraph so every page has at least one
/// glyph — the typed model forbids empty paragraphs.
///
/// # Errors
/// Returns [`PdfModelError`] if the resulting paragraph or heading text
/// fails core-level validation. All failures are bugs here, not in the
/// caller — the core rejects strings the agent has no reason to emit.
pub fn page_to_blocks(content: &PdfPageContent) -> Result<Vec<Block>, PdfModelError> {
    match content {
        PdfPageContent::Text { content } => text_to_blocks(content),
        PdfPageContent::Markdown { content } => markdown_to_blocks(content),
        PdfPageContent::Image { path, caption } => image_placeholder_blocks(path, caption.as_deref()),
    }
}

fn text_to_blocks(text: &str) -> Result<Vec<Block>, PdfModelError> {
    paragraphs_from_text(text)
}

fn markdown_to_blocks(source: &str) -> Result<Vec<Block>, PdfModelError> {
    let mut out: Vec<Block> = Vec::new();
    let mut buffer: Vec<&str> = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim_end();
        if let Some(heading) = parse_heading(line)? {
            flush_paragraph(&mut buffer, &mut out)?;
            out.push(Block::Heading(heading));
            continue;
        }
        if line.trim().is_empty() {
            flush_paragraph(&mut buffer, &mut out)?;
            continue;
        }
        buffer.push(line);
    }
    flush_paragraph(&mut buffer, &mut out)?;

    if out.is_empty() {
        out.push(Block::Paragraph(Paragraph::plain(placeholder_text())?));
    }
    Ok(out)
}

fn image_placeholder_blocks(path: &str, caption: Option<&str>) -> Result<Vec<Block>, PdfModelError> {
    // T2.5 replaces the placeholder with a real `Block::Image`. Until
    // then we keep the legacy "[Image: …]" marker so nothing is silently
    // dropped, but the payload is typed text rather than raw bytes.
    let mut blocks = vec![Block::Paragraph(Paragraph::plain(format!(
        "[Image: {path}]"
    ))?)];
    if let Some(text) = caption {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::plain(trimmed)?));
        }
    }
    Ok(blocks)
}

fn paragraphs_from_text(text: &str) -> Result<Vec<Block>, PdfModelError> {
    let mut out: Vec<Block> = Vec::new();
    let mut buffer: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            flush_paragraph(&mut buffer, &mut out)?;
        } else {
            buffer.push(line);
        }
    }
    flush_paragraph(&mut buffer, &mut out)?;
    if out.is_empty() {
        out.push(Block::Paragraph(Paragraph::plain(placeholder_text())?));
    }
    Ok(out)
}

fn flush_paragraph(buffer: &mut Vec<&str>, out: &mut Vec<Block>) -> Result<(), PdfModelError> {
    if buffer.is_empty() {
        return Ok(());
    }
    let joined = buffer.join(" ");
    buffer.clear();
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    out.push(Block::Paragraph(Paragraph::plain(trimmed.to_string())?));
    Ok(())
}

fn parse_heading(line: &str) -> Result<Option<Heading>, PdfModelError> {
    let (level, rest) = if let Some(r) = line.strip_prefix("### ") {
        (3u8, r)
    } else if let Some(r) = line.strip_prefix("## ") {
        (2u8, r)
    } else if let Some(r) = line.strip_prefix("# ") {
        (1u8, r)
    } else {
        return Ok(None);
    };
    let text = rest.trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(Heading::new(HeadingLevel::new(level)?, text)?))
}

/// `U+00A0 NO-BREAK SPACE`. Used as the single glyph on pages that
/// otherwise have no content, so the typed-paragraph invariant holds.
const fn placeholder_text() -> &'static str {
    "\u{00A0}"
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn assert_is_paragraph_with(block: &Block, expected: &str) {
        match block {
            Block::Paragraph(p) => {
                let joined: String = p.runs().iter().map(|r| r.text()).collect();
                assert_eq!(joined, expected, "paragraph text mismatch");
            }
            other => panic!("expected Block::Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn text_becomes_one_paragraph_per_blank_line_group() {
        let blocks = page_to_blocks(&PdfPageContent::Text {
            content: "hello world\nline two\n\nnext paragraph".to_string(),
        })
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_is_paragraph_with(&blocks[0], "hello world line two");
        assert_is_paragraph_with(&blocks[1], "next paragraph");
    }

    #[test]
    fn empty_text_produces_single_placeholder_paragraph() {
        let blocks = page_to_blocks(&PdfPageContent::Text {
            content: "   \n\n".to_string(),
        })
        .unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], Block::Paragraph(_)));
    }

    #[test]
    fn markdown_recognises_three_heading_levels() {
        let blocks = page_to_blocks(&PdfPageContent::Markdown {
            content: "# H1\n## H2\n### H3\nplain body\n".to_string(),
        })
        .unwrap();
        assert_eq!(blocks.len(), 4, "3 headings + 1 paragraph");
        assert!(matches!(
            &blocks[0],
            Block::Heading(h) if h.level.get() == 1 && h.text() == "H1"
        ));
        assert!(matches!(
            &blocks[1],
            Block::Heading(h) if h.level.get() == 2 && h.text() == "H2"
        ));
        assert!(matches!(
            &blocks[2],
            Block::Heading(h) if h.level.get() == 3 && h.text() == "H3"
        ));
        assert_is_paragraph_with(&blocks[3], "plain body");
    }

    #[test]
    fn markdown_blank_lines_split_paragraphs() {
        let blocks = page_to_blocks(&PdfPageContent::Markdown {
            content: "first line\nsecond line\n\nthird line".to_string(),
        })
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_is_paragraph_with(&blocks[0], "first line second line");
        assert_is_paragraph_with(&blocks[1], "third line");
    }

    #[test]
    fn image_is_rendered_as_typed_placeholder() {
        let blocks = page_to_blocks(&PdfPageContent::Image {
            path: "/tmp/x.png".to_string(),
            caption: Some("diagram 1".to_string()),
        })
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_is_paragraph_with(&blocks[0], "[Image: /tmp/x.png]");
        assert_is_paragraph_with(&blocks[1], "diagram 1");
    }

    #[test]
    fn image_without_caption_has_single_block() {
        let blocks = page_to_blocks(&PdfPageContent::Image {
            path: "/tmp/x.png".to_string(),
            caption: None,
        })
        .unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn markdown_heading_with_empty_text_is_treated_as_paragraph() {
        let blocks = page_to_blocks(&PdfPageContent::Markdown {
            content: "# ".to_string(),
        })
        .unwrap();
        assert_eq!(blocks.len(), 1, "empty heading falls through to paragraph");
        assert!(matches!(blocks[0], Block::Paragraph(_)));
    }
}
