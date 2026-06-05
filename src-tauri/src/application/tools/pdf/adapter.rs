//! Maps the legacy [`PdfPageContent`] input surface onto typed blocks.
//!
//! The draft store speaks the string-blob enum that the tool JSON schema
//! advertises (`Markdown`, `Text`, `Image`), but the renderer only
//! understands the typed [`Block`] model from the core. This module is
//! the single place where that translation happens.
//!
//! The Markdown parser recognises:
//! - `#`–`######` headings (all 6 levels)
//! - Blank-line-separated paragraphs
//! - Unordered lists (`-`, `*`, `+`) with nesting via indentation
//! - Ordered lists (`1.`, `2.`) with nesting via indentation
//! - Fenced code blocks (`` ``` ``) with optional language tag
//! - Blockquotes (`>`) → [`Callout`] blocks
//! - Horizontal rules (`---`, `***`, `___`) → [`Block::PageBreak`]
//! - Inline formatting: `**bold**`, `*italic*`, `` `code` ``,
//!   `[text](url)` links
//! - Pipe tables (`| A | B |` with header separator)

use crate::core::documents::pdf::block::{
    Block, Callout, CodeBlock, ColumnWidth, CoverBlock, ImageBlock, ImageFit, InlineRun,
    InlineStyle, ListBlock, ListItem, Paragraph, TableBlock, TableCell, TableRow,
};
use crate::core::documents::pdf::error::PdfModelError;
use crate::core::documents::pdf::section::{Heading, HeadingLevel};
use crate::infrastructure::tools::documents::shared::PdfPageContent;

/// Translates a single page of draft content into typed blocks.
///
/// # Errors
/// Returns [`PdfModelError`] if the resulting blocks fail core
/// validation (e.g. an empty paragraph that somehow slips through).
pub fn page_to_blocks(content: &PdfPageContent) -> Result<Vec<Block>, PdfModelError> {
    match content {
        PdfPageContent::Text { content } => text_to_blocks(content),
        PdfPageContent::Markdown { content } => markdown_to_blocks(content),
        PdfPageContent::Image {
            path,
            caption,
            width_mm,
            height_mm,
            fit,
        } => image_to_blocks(path, caption.as_deref(), *width_mm, *height_mm, *fit),
        PdfPageContent::Cover {
            title,
            subtitle,
            author,
            date,
        } => cover_to_blocks(
            title,
            subtitle.as_deref(),
            author.as_deref(),
            date.as_deref(),
        ),
    }
}

// ── Text ──────────────────────────────────────────────────────────

fn text_to_blocks(text: &str) -> Result<Vec<Block>, PdfModelError> {
    let mut out: Vec<Block> = Vec::new();
    let mut buffer: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            flush_paragraph_plain(&mut buffer, &mut out)?;
        } else {
            buffer.push(line);
        }
    }
    flush_paragraph_plain(&mut buffer, &mut out)?;
    ensure_non_empty(&mut out)?;
    Ok(out)
}

// ── Image ─────────────────────────────────────────────────────────

const MAX_IMAGE_FILE_BYTES: u64 = 50 * 1024 * 1024;

fn image_to_blocks(
    path: &str,
    caption: Option<&str>,
    width_mm: Option<f32>,
    height_mm: Option<f32>,
    fit: ImageFit,
) -> Result<Vec<Block>, PdfModelError> {
    let image_path = std::path::Path::new(path);
    if image_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(PdfModelError::InvalidImagePath(path.to_string()));
    }
    if let Ok(meta) = std::fs::metadata(image_path) {
        if meta.len() > MAX_IMAGE_FILE_BYTES {
            return Err(PdfModelError::ImageTooLarge {
                path: path.to_string(),
                size: meta.len(),
                max: MAX_IMAGE_FILE_BYTES,
            });
        }
    }
    let data = std::fs::read(path).unwrap_or_default();
    if data.is_empty() {
        // Fall back to placeholder text when the file is missing or
        // unreadable — better than crashing the whole render.
        let mut blocks = vec![Block::Paragraph(Paragraph::plain(format!(
            "[Image: {path}]"
        ))?)];
        if let Some(text) = caption {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                blocks.push(Block::Paragraph(Paragraph::plain(trimmed)?));
            }
        }
        return Ok(blocks);
    }
    let mut img = ImageBlock::new(data)?;
    if let Some(text) = caption {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            img = img.with_alt(trimmed);
        }
    }
    if let Some(w) = width_mm {
        img = img.with_width_mm(w);
    }
    if let Some(h) = height_mm {
        img = img.with_height_mm(h);
    }
    img = img.with_fit(fit);
    Ok(vec![Block::Image(img)])
}

// ── Cover ─────────────────────────────────────────────────────────

fn cover_to_blocks(
    title: &str,
    subtitle: Option<&str>,
    author: Option<&str>,
    date: Option<&str>,
) -> Result<Vec<Block>, PdfModelError> {
    let mut cover = CoverBlock::new(title.trim())?;
    if let Some(text) = subtitle {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            cover = cover.with_subtitle(trimmed);
        }
    }
    if let Some(text) = author {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            cover = cover.with_author(trimmed);
        }
    }
    if let Some(text) = date {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            cover = cover.with_date(trimmed);
        }
    }
    Ok(vec![Block::CoverPage(cover)])
}

// ── Markdown ──────────────────────────────────────────────────────

fn markdown_to_blocks(source: &str) -> Result<Vec<Block>, PdfModelError> {
    let mut out: Vec<Block> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let len = lines.len();
    let mut i = 0;

    while i < len {
        let line = lines[i];
        let trimmed = line.trim();

        // ── Blank line → flush ──
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // ── Horizontal rule → PageBreak ──
        if is_horizontal_rule(trimmed) {
            out.push(Block::PageBreak);
            i += 1;
            continue;
        }

        // ── Heading ──
        if let Some(heading) = parse_heading(trimmed)? {
            out.push(Block::Heading(heading));
            i += 1;
            continue;
        }

        // ── Fenced code block ──
        if trimmed.starts_with("```") {
            let lang = trimmed.strip_prefix("```").unwrap().trim();
            let lang = if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            };
            i += 1;
            let mut code_lines: Vec<&str> = Vec::new();
            while i < len && !lines[i].trim_start().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < len {
                i += 1; // skip closing ```
            }
            let code_text = code_lines.join("\n");
            if !code_text.trim().is_empty() {
                let mut cb = CodeBlock::new(code_text)?;
                if let Some(l) = lang {
                    cb = cb.with_language(l);
                }
                out.push(Block::Code(cb));
            }
            continue;
        }

        // ── Blockquote → Callout ──
        if trimmed.starts_with('>') {
            let mut quote_lines: Vec<String> = Vec::new();
            while i < len {
                let ql = lines[i].trim();
                if let Some(rest) = ql.strip_prefix('>') {
                    quote_lines.push(rest.trim_start().to_string());
                    i += 1;
                } else if ql.is_empty() {
                    break;
                } else {
                    break;
                }
            }
            let body = quote_lines.join("\n");
            if !body.trim().is_empty() {
                out.push(Block::Callout(Callout::new(body.trim())?));
            }
            continue;
        }

        // ── Pipe table ──
        if trimmed.contains('|') && i + 1 < len && is_table_separator(lines[i + 1].trim()) {
            let (table_block, consumed) = parse_table(&lines[i..]);
            if let Some(tb) = table_block {
                out.push(Block::Table(tb));
            }
            i += consumed;
            continue;
        }

        // ── List (ordered or unordered) ──
        if is_list_line(trimmed) {
            let (list_block, consumed) = parse_list(&lines[i..]);
            if let Some(lb) = list_block {
                out.push(Block::List(lb));
            }
            i += consumed;
            continue;
        }

        // ── Paragraph (default) ──
        let mut para_lines: Vec<&str> = Vec::new();
        while i < len {
            let pl = lines[i].trim();
            if pl.is_empty()
                || pl.starts_with('#')
                || pl.starts_with("```")
                || pl.starts_with('>')
                || is_horizontal_rule(pl)
                || is_list_line(pl)
                || (pl.contains('|') && i + 1 < len && is_table_separator(lines[i + 1].trim()))
            {
                break;
            }
            para_lines.push(lines[i]);
            i += 1;
        }
        if !para_lines.is_empty() {
            let text = para_lines.join(" ");
            let runs = parse_inline_runs(text.trim());
            if !runs.is_empty() {
                out.push(Block::Paragraph(Paragraph::new(runs)?));
            }
        }
    }

    ensure_non_empty(&mut out)?;
    Ok(out)
}

// ── Heading parser ────────────────────────────────────────────────

fn parse_heading(line: &str) -> Result<Option<Heading>, PdfModelError> {
    let mut level = 0u8;
    let mut rest = line;
    while rest.starts_with('#') && level < 6 {
        level += 1;
        rest = &rest[1..];
    }
    if level == 0 || !rest.starts_with(' ') {
        return Ok(None);
    }
    let text = rest.trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(Heading::new(HeadingLevel::new(level)?, text)?))
}

// ── Horizontal rule ───────────────────────────────────────────────

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.as_bytes()[0];
    (first == b'-' || first == b'*' || first == b'_')
        && trimmed.bytes().all(|b| b == first || b == b' ')
        && trimmed.bytes().filter(|&b| b == first).count() >= 3
}

// ── List parser ───────────────────────────────────────────────────

fn is_list_line(line: &str) -> bool {
    is_unordered_list_line(line) || is_ordered_list_line(line)
}

fn is_unordered_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ")
}

fn is_ordered_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    // Match `1. `, `2. `, etc.
    if let Some(dot_pos) = trimmed.find(". ") {
        trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) && dot_pos > 0
    } else {
        false
    }
}

fn line_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn strip_list_marker(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return rest;
    }
    // Ordered: strip "N. "
    if let Some(dot_pos) = trimmed.find(". ") {
        if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) && dot_pos > 0 {
            return &trimmed[dot_pos + 2..];
        }
    }
    trimmed
}

fn parse_list(lines: &[&str]) -> (Option<ListBlock>, usize) {
    if lines.is_empty() {
        return (None, 0);
    }
    let base_indent = line_indent(lines[0]);
    let ordered = is_ordered_list_line(lines[0]);
    let mut items: Vec<ListItem> = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            consumed += 1;
            continue;
        }
        let indent = line_indent(line);
        if indent < base_indent {
            break;
        }
        if indent > base_indent {
            // Nested list — parse recursively
            let (child, child_consumed) = parse_list(&lines[consumed..]);
            if let Some(child_list) = child {
                if let Some(last_item) = items.last_mut() {
                    // Attach as child of last item
                    let children = vec![child_list];
                    // We need to rebuild the item with children — use a temporary
                    let runs = last_item.runs().to_vec();
                    let existing_children = last_item.children().to_vec();
                    let mut all_children = existing_children;
                    all_children.extend(children);
                    if let Ok(new_item) = ListItem::new(runs) {
                        *last_item = new_item.with_children(all_children);
                    }
                }
            }
            consumed += child_consumed;
            continue;
        }
        // Same indent — a list item at this level
        if !is_list_line(trimmed) {
            break;
        }
        let text = strip_list_marker(line);
        let runs = parse_inline_runs(text.trim());
        if let Ok(item) = ListItem::new(if runs.is_empty() {
            vec![InlineRun::new(text.trim()).unwrap_or_else(|_| InlineRun::new("\u{00A0}").unwrap())]
        } else {
            runs
        }) {
            items.push(item);
        }
        consumed += 1;
    }

    if items.is_empty() {
        return (None, consumed.max(1));
    }
    match ListBlock::new(ordered, items) {
        Ok(lb) => (Some(lb), consumed),
        Err(_) => (None, consumed),
    }
}

// ── Table parser ──────────────────────────────────────────────────

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    !trimmed.is_empty()
        && trimmed.split('|').all(|cell| {
            let c = cell.trim();
            !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
        })
}

fn parse_table_row_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_table(lines: &[&str]) -> (Option<TableBlock>, usize) {
    if lines.len() < 2 {
        return (None, 0);
    }
    let header_cells = parse_table_row_cells(lines[0]);
    // lines[1] is the separator — skip it
    let col_count = header_cells.len();
    let columns: Vec<ColumnWidth> = (0..col_count).map(|_| ColumnWidth::auto()).collect();

    let header_row = TableRow::new(
        header_cells
            .into_iter()
            .map(|c| {
                if c.is_empty() {
                    TableCell::new(vec![])
                } else {
                    let runs = parse_inline_runs(&c);
                    TableCell::new(if runs.is_empty() {
                        vec![InlineRun::new(&c)
                            .unwrap_or_else(|_| InlineRun::new("\u{00A0}").unwrap())]
                    } else {
                        runs
                    })
                }
            })
            .collect(),
    );

    let mut body_rows: Vec<TableRow> = Vec::new();
    let mut consumed = 2; // header + separator

    while consumed < lines.len() {
        let line = lines[consumed].trim();
        if line.is_empty() || !line.contains('|') {
            break;
        }
        let cells = parse_table_row_cells(lines[consumed]);
        // Pad or truncate to match column count
        let mut row_cells: Vec<TableCell> = cells
            .into_iter()
            .take(col_count)
            .map(|c| {
                if c.is_empty() {
                    TableCell::new(vec![])
                } else {
                    let runs = parse_inline_runs(&c);
                    TableCell::new(if runs.is_empty() {
                        vec![InlineRun::new(&c)
                            .unwrap_or_else(|_| InlineRun::new("\u{00A0}").unwrap())]
                    } else {
                        runs
                    })
                }
            })
            .collect();
        // Pad with empty cells if needed
        while row_cells.len() < col_count {
            row_cells.push(TableCell::new(vec![]));
        }
        body_rows.push(TableRow::new(row_cells));
        consumed += 1;
    }

    match TableBlock::new(columns, Some(header_row), body_rows) {
        Ok(tb) => (Some(tb), consumed),
        Err(_) => (None, consumed),
    }
}

// ── Inline formatting parser ──────────────────────────────────────

/// Parses inline markdown formatting into [`InlineRun`]s.
///
/// Recognises: `**bold**`, `*italic*`, `` `code` ``, `[text](url)`.
fn parse_inline_runs(text: &str) -> Vec<InlineRun> {
    let mut runs: Vec<InlineRun> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the next formatting marker
        if let Some(pos) = find_next_marker(remaining) {
            // Emit plain text before the marker
            if pos > 0 {
                if let Ok(run) = InlineRun::new(&remaining[..pos]) {
                    runs.push(run);
                }
            }
            remaining = &remaining[pos..];

            // Try to parse the marker
            if let Some((run, advance)) = try_parse_inline_marker(remaining) {
                runs.push(run);
                remaining = &remaining[advance..];
            } else {
                // Not a valid marker — emit the character as plain text
                let ch_len = remaining.chars().next().map_or(1, char::len_utf8);
                if let Ok(run) = InlineRun::new(&remaining[..ch_len]) {
                    runs.push(run);
                }
                remaining = &remaining[ch_len..];
            }
        } else {
            // No more markers — rest is plain text
            if let Ok(run) = InlineRun::new(remaining) {
                runs.push(run);
            }
            break;
        }
    }

    runs
}

fn find_next_marker(text: &str) -> Option<usize> {
    text.find(|c: char| c == '*' || c == '`' || c == '[')
}

fn try_parse_inline_marker(text: &str) -> Option<(InlineRun, usize)> {
    // Bold: **text**
    if text.starts_with("**") {
        if let Some(end) = text[2..].find("**") {
            let inner = &text[2..2 + end];
            if !inner.is_empty() {
                if let Ok(run) = InlineRun::new(inner) {
                    return Some((run.with_style(InlineStyle::bold()), 2 + end + 2));
                }
            }
        }
    }

    // Italic: *text*
    if text.starts_with('*') && !text.starts_with("**") {
        if let Some(end) = text[1..].find('*') {
            let inner = &text[1..1 + end];
            if !inner.is_empty() {
                if let Ok(run) = InlineRun::new(inner) {
                    let style = InlineStyle {
                        bold: false,
                        italic: true,
                        code: false,
                    };
                    return Some((run.with_style(style), 1 + end + 1));
                }
            }
        }
    }

    // Inline code: `text`
    if text.starts_with('`') && !text.starts_with("```") {
        if let Some(end) = text[1..].find('`') {
            let inner = &text[1..1 + end];
            if !inner.is_empty() {
                if let Ok(run) = InlineRun::new(inner) {
                    let style = InlineStyle {
                        bold: false,
                        italic: false,
                        code: true,
                    };
                    return Some((run.with_style(style), 1 + end + 1));
                }
            }
        }
    }

    // Link: [text](url)
    if text.starts_with('[') {
        if let Some(bracket_end) = text.find("](") {
            let link_text = &text[1..bracket_end];
            let after_paren = &text[bracket_end + 2..];
            if let Some(paren_end) = after_paren.find(')') {
                let url = &after_paren[..paren_end];
                if !link_text.is_empty() && !url.is_empty() {
                    if let Ok(run) = InlineRun::new(link_text) {
                        return Some((run.with_link(url), bracket_end + 2 + paren_end + 1));
                    }
                }
            }
        }
    }

    None
}

// ── Helpers ───────────────────────────────────────────────────────

fn flush_paragraph_plain(
    buffer: &mut Vec<&str>,
    out: &mut Vec<Block>,
) -> Result<(), PdfModelError> {
    if buffer.is_empty() {
        return Ok(());
    }
    let joined = buffer.join(" ");
    buffer.clear();
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    out.push(Block::Paragraph(Paragraph::plain(trimmed)?));
    Ok(())
}

fn ensure_non_empty(out: &mut Vec<Block>) -> Result<(), PdfModelError> {
    if out.is_empty() {
        out.push(Block::Paragraph(Paragraph::plain(PLACEHOLDER)?));
    }
    Ok(())
}

/// `U+00A0 NO-BREAK SPACE`. Used as the single glyph on pages that
/// otherwise have no content, so the typed-paragraph invariant holds.
const PLACEHOLDER: &str = "\u{00A0}";

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn assert_is_paragraph(block: &Block) {
        assert!(
            matches!(block, Block::Paragraph(_)),
            "expected Paragraph, got {block:?}"
        );
    }

    fn assert_paragraph_text(block: &Block, expected: &str) {
        match block {
            Block::Paragraph(p) => {
                let joined: String = p.runs().iter().map(|r| r.text()).collect();
                assert_eq!(joined, expected, "paragraph text mismatch");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // -- Headings --

    #[test]
    fn markdown_recognises_all_six_heading_levels() {
        let md = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6\nBody";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 7);
        for (i, level) in [1u8, 2, 3, 4, 5, 6].iter().enumerate() {
            assert!(matches!(&blocks[i], Block::Heading(h) if h.level.get() == *level));
        }
        assert_paragraph_text(&blocks[6], "Body");
    }

    // -- Lists --

    #[test]
    fn unordered_list_parsed() {
        let md = "- first\n- second\n- third";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::List(l) if !l.ordered && l.items().len() == 3));
    }

    #[test]
    fn ordered_list_parsed() {
        let md = "1. alpha\n2. bravo";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::List(l) if l.ordered && l.items().len() == 2));
    }

    // -- Code blocks --

    #[test]
    fn fenced_code_block_with_language() {
        let md = "```rust\nfn main() {}\n```";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Code(cb) => {
                assert_eq!(cb.code(), "fn main() {}");
                assert_eq!(cb.language.as_deref(), Some("rust"));
            }
            other => panic!("expected Code, got {other:?}"),
        }
    }

    // -- Blockquotes → Callouts --

    #[test]
    fn blockquote_becomes_callout() {
        let md = "> This is important\n> really important";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Callout(c) => assert!(c.body().contains("important")),
            other => panic!("expected Callout, got {other:?}"),
        }
    }

    // -- Horizontal rule → PageBreak --

    #[test]
    fn horizontal_rule_becomes_page_break() {
        let md = "Before\n\n---\n\nAfter";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_paragraph_text(&blocks[0], "Before");
        assert!(matches!(&blocks[1], Block::PageBreak));
        assert_paragraph_text(&blocks[2], "After");
    }

    // -- Tables --

    #[test]
    fn pipe_table_parsed() {
        let md = "| Name | Value |\n|------|-------|\n| foo | 42 |\n| bar | 99 |";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table(t) => {
                assert!(t.header().is_some());
                assert_eq!(t.rows().len(), 2);
                assert_eq!(t.columns().len(), 2);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    // -- Inline formatting --

    #[test]
    fn bold_text_parsed() {
        let md = "This is **bold** text";
        let blocks = markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 1);
        if let Block::Paragraph(p) = &blocks[0] {
            let bold_run = p.runs().iter().find(|r| r.style.bold);
            assert!(bold_run.is_some(), "should have a bold run");
            assert_eq!(bold_run.unwrap().text(), "bold");
        }
    }

    #[test]
    fn italic_text_parsed() {
        let md = "This is *italic* text";
        let blocks = markdown_to_blocks(md).unwrap();
        if let Block::Paragraph(p) = &blocks[0] {
            let italic_run = p.runs().iter().find(|r| r.style.italic);
            assert!(italic_run.is_some(), "should have an italic run");
        }
    }

    #[test]
    fn inline_code_parsed() {
        let md = "Use `println!` macro";
        let blocks = markdown_to_blocks(md).unwrap();
        if let Block::Paragraph(p) = &blocks[0] {
            let code_run = p.runs().iter().find(|r| r.style.code);
            assert!(code_run.is_some(), "should have a code run");
            assert_eq!(code_run.unwrap().text(), "println!");
        }
    }

    #[test]
    fn link_parsed() {
        let md = "Visit [Example](https://example.com) today";
        let blocks = markdown_to_blocks(md).unwrap();
        if let Block::Paragraph(p) = &blocks[0] {
            let link_run = p.runs().iter().find(|r| r.link.is_some());
            assert!(link_run.is_some(), "should have a link run");
            let lr = link_run.unwrap();
            assert_eq!(lr.text(), "Example");
            assert_eq!(lr.link.as_deref(), Some("https://example.com"));
        }
    }

    // -- Image --

    #[test]
    fn image_with_missing_file_produces_placeholder() {
        let blocks = page_to_blocks(&PdfPageContent::Image {
            path: "/nonexistent/file.png".to_string(),
            caption: Some("cap".to_string()),
            width_mm: None,
            height_mm: None,
            fit: ImageFit::Contain,
        })
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_paragraph_text(&blocks[0], "[Image: /nonexistent/file.png]");
    }

    #[test]
    fn image_path_with_parent_dir_rejected() {
        let result = page_to_blocks(&PdfPageContent::Image {
            path: "../../etc/passwd".to_string(),
            caption: None,
            width_mm: None,
            height_mm: None,
            fit: ImageFit::Contain,
        });
        assert!(result.is_err(), "path with .. must be rejected");
        let err = result.unwrap_err();
        assert!(
            matches!(err, PdfModelError::InvalidImagePath(_)),
            "expected InvalidImagePath, got {err:?}"
        );
    }

    #[test]
    fn image_absolute_path_without_traversal_accepted() {
        let blocks = page_to_blocks(&PdfPageContent::Image {
            path: "/nonexistent/absolute/image.png".to_string(),
            caption: None,
            width_mm: None,
            height_mm: None,
            fit: ImageFit::Contain,
        })
        .unwrap();
        assert!(!blocks.is_empty());
    }

    // -- Cover --

    #[test]
    fn cover_produces_single_cover_block_with_all_fields() {
        let blocks = page_to_blocks(&PdfPageContent::Cover {
            title: "Quarterly Report".to_string(),
            subtitle: Some("Q1 2026".to_string()),
            author: Some("Ada Lovelace".to_string()),
            date: Some("2026-04-22".to_string()),
        })
        .unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CoverPage(cover) => {
                assert_eq!(cover.title, "Quarterly Report");
                assert_eq!(cover.subtitle.as_deref(), Some("Q1 2026"));
                assert_eq!(cover.author.as_deref(), Some("Ada Lovelace"));
                assert_eq!(cover.date.as_deref(), Some("2026-04-22"));
            }
            other => panic!("expected CoverPage, got {other:?}"),
        }
    }

    #[test]
    fn cover_trims_whitespace_and_drops_blank_optionals() {
        let blocks = page_to_blocks(&PdfPageContent::Cover {
            title: "  Report  ".to_string(),
            subtitle: Some("   ".to_string()),
            author: None,
            date: None,
        })
        .unwrap();
        match &blocks[0] {
            Block::CoverPage(cover) => {
                assert_eq!(cover.title, "Report");
                assert!(cover.subtitle.is_none(), "blank subtitle must be dropped");
                assert!(cover.author.is_none());
                assert!(cover.date.is_none());
            }
            other => panic!("expected CoverPage, got {other:?}"),
        }
    }

    #[test]
    fn cover_with_empty_title_is_rejected() {
        let result = page_to_blocks(&PdfPageContent::Cover {
            title: "   ".to_string(),
            subtitle: None,
            author: None,
            date: None,
        });
        assert!(result.is_err(), "empty title must be rejected");
    }

    // -- Edge cases --

    #[test]
    fn empty_markdown_produces_placeholder() {
        let blocks = markdown_to_blocks("").unwrap();
        assert_eq!(blocks.len(), 1);
        assert_is_paragraph(&blocks[0]);
    }

    #[test]
    fn mixed_content_roundtrip() {
        let md = r#"# Report

This is the **introduction** with a [link](https://example.com).

## Data

| Key | Value |
|-----|-------|
| CPU | 90% |

- Item one
- Item two

```python
print("hello")
```

> Note: this is important

---

## Conclusion

Done."#;
        let blocks = markdown_to_blocks(md).unwrap();
        // Should contain: heading, paragraph, heading, table, list, code, callout, page break, heading, paragraph
        assert!(
            blocks.len() >= 9,
            "mixed content should produce 9+ blocks, got {}",
            blocks.len()
        );
        assert!(matches!(&blocks[0], Block::Heading(_)));
    }
}
