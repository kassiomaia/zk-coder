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

/// One entry in a [`ListBlock`].
///
/// Carries its own inline runs and an optional ordered list of nested
/// `ListBlock`s — the renderer recurses on the children so arbitrarily
/// deep nesting works without changes to the block enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    runs: Vec<InlineRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    children: Vec<ListBlock>,
}

impl ListItem {
    /// Builds a list item from its inline runs.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyListItem`] if `runs` is empty.
    pub fn new(runs: Vec<InlineRun>) -> Result<Self, PdfModelError> {
        if runs.is_empty() {
            return Err(PdfModelError::EmptyListItem);
        }
        Ok(Self {
            runs,
            children: Vec::new(),
        })
    }

    /// Convenience: a single plain run.
    ///
    /// # Errors
    /// Propagates [`PdfModelError::EmptyInlineRun`] / [`PdfModelError::EmptyListItem`].
    pub fn plain(text: impl Into<String>) -> Result<Self, PdfModelError> {
        Self::new(vec![InlineRun::new(text)?])
    }

    /// Replaces the nested children list.
    #[must_use]
    pub fn with_children(mut self, children: Vec<ListBlock>) -> Self {
        self.children = children;
        self
    }

    /// Returns the ordered inline runs.
    #[must_use]
    pub fn runs(&self) -> &[InlineRun] {
        &self.runs
    }

    /// Returns the nested child lists, if any.
    #[must_use]
    pub fn children(&self) -> &[ListBlock] {
        &self.children
    }
}

/// Ordered or unordered list of [`ListItem`]s.
///
/// Nesting is structural (each item carries its own children) rather
/// than indentation-based: the model never has to guess whether two
/// adjacent items are siblings or parent/child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBlock {
    /// `true` for ordered (numbered) lists, `false` for bulleted.
    pub ordered: bool,
    items: Vec<ListItem>,
}

impl ListBlock {
    /// Builds a list block.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyList`] if `items` is empty.
    pub fn new(ordered: bool, items: Vec<ListItem>) -> Result<Self, PdfModelError> {
        if items.is_empty() {
            return Err(PdfModelError::EmptyList);
        }
        Ok(Self { ordered, items })
    }

    /// Returns the ordered list items.
    #[must_use]
    pub fn items(&self) -> &[ListItem] {
        &self.items
    }
}

/// Code block rendered in a monospaced font with a background fill.
///
/// `language` is informational (carried through serde for future syntax
/// highlighting); the M2 renderer ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Source language hint (e.g. `"rust"`); reserved for future use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    code: String,
}

impl CodeBlock {
    /// Builds a code block.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyCodeBlock`] if `code` is empty or
    /// whitespace-only.
    pub fn new(code: impl Into<String>) -> Result<Self, PdfModelError> {
        let code = code.into();
        if code.trim().is_empty() {
            return Err(PdfModelError::EmptyCodeBlock);
        }
        Ok(Self {
            language: None,
            code,
        })
    }

    /// Attaches the language hint.
    #[must_use]
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Returns the source code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
}

/// Callout / admonition box rendered with a theme-supplied background.
///
/// The optional `title` is rendered in bold above the body text. Theme
/// colours come from the palette — never hard-coded — so future
/// "info / warn / note" variants can be styled without renderer
/// changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callout {
    /// Optional bold title rendered above the body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    body: String,
}

impl Callout {
    /// Builds a callout from its body text.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyCallout`] if `body` is empty or
    /// whitespace-only.
    pub fn new(body: impl Into<String>) -> Result<Self, PdfModelError> {
        let body = body.into();
        if body.trim().is_empty() {
            return Err(PdfModelError::EmptyCallout);
        }
        Ok(Self { title: None, body })
    }

    /// Attaches a title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns the body text.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Embedded image rendered inline.
///
/// The block carries the raw image bytes (PNG, JPEG, BMP) and an
/// optional display width in mm. When the display width is `None` the
/// renderer picks a default fitting the usable page width.
///
/// Serde skips the heavy `data` field so JSON round-trips describe the
/// _structure_ without carrying megabytes of pixel data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBlock {
    /// Optional alt-text / caption for accessibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Display width in mm. `None` → fit to usable page width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    /// Raw image bytes (PNG / JPEG / BMP).
    #[serde(skip)]
    data: Vec<u8>,
}

impl ImageBlock {
    /// Builds an image block from raw bytes.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyImageData`] if `data` is empty.
    pub fn new(data: Vec<u8>) -> Result<Self, PdfModelError> {
        if data.is_empty() {
            return Err(PdfModelError::EmptyImageData);
        }
        Ok(Self {
            alt: None,
            width_mm: None,
            data,
        })
    }

    /// Attaches alt-text.
    #[must_use]
    pub fn with_alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Overrides the display width.
    #[must_use]
    pub const fn with_width_mm(mut self, mm: f32) -> Self {
        self.width_mm = Some(mm);
        self
    }

    /// Returns the raw image bytes.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Column width specification for a [`TableBlock`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ColumnWidth {
    /// Fixed width in millimetres.
    Fixed {
        /// Width in mm.
        mm: f32,
    },
    /// Proportional share of the remaining width.
    ///
    /// After all `Fixed` columns are satisfied, the leftover width is
    /// divided among `Auto` columns in proportion to their `weight`.
    Auto {
        /// Relative weight (default `1.0`).
        #[serde(default = "default_auto_weight")]
        weight: f32,
    },
}

fn default_auto_weight() -> f32 {
    1.0
}

impl ColumnWidth {
    /// Shorthand for a fixed-width column.
    #[must_use]
    pub const fn fixed(mm: f32) -> Self {
        Self::Fixed { mm }
    }

    /// Shorthand for an auto-width column with weight `1.0`.
    #[must_use]
    pub fn auto() -> Self {
        Self::Auto {
            weight: default_auto_weight(),
        }
    }
}

/// A single cell in a [`TableRow`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    runs: Vec<InlineRun>,
}

impl TableCell {
    /// Builds a cell from its inline runs.
    ///
    /// An empty `runs` vec produces a blank cell — this is allowed so
    /// that sparse tables (with intentional empty cells) can be built.
    #[must_use]
    pub fn new(runs: Vec<InlineRun>) -> Self {
        Self { runs }
    }

    /// Convenience: a single plain-text cell.
    ///
    /// # Errors
    /// Propagates [`PdfModelError::EmptyInlineRun`] if `text` is empty.
    pub fn plain(text: impl Into<String>) -> Result<Self, PdfModelError> {
        Ok(Self::new(vec![InlineRun::new(text)?]))
    }

    /// Returns the ordered inline runs.
    #[must_use]
    pub fn runs(&self) -> &[InlineRun] {
        &self.runs
    }
}

/// A row of [`TableCell`]s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    cells: Vec<TableCell>,
}

impl TableRow {
    /// Builds a row from its cells.
    #[must_use]
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self { cells }
    }

    /// Returns the ordered cells.
    #[must_use]
    pub fn cells(&self) -> &[TableCell] {
        &self.cells
    }
}

/// Table block with optional header row and body rows.
///
/// Column widths drive the renderer's horizontal layout. Each row must
/// have exactly as many cells as the table has columns — construction
/// validates this so the renderer never panics on index-out-of-bounds.
///
/// The optional `header` is repeated by the renderer whenever the table
/// overflows onto a new page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableBlock {
    columns: Vec<ColumnWidth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    header: Option<TableRow>,
    rows: Vec<TableRow>,
}

impl TableBlock {
    /// Builds a table.
    ///
    /// # Errors
    /// - [`PdfModelError::EmptyTableColumns`] if `columns` is empty.
    /// - [`PdfModelError::TableCellCountMismatch`] if any row (header
    ///   or body) has a different cell count than `columns.len()`.
    pub fn new(
        columns: Vec<ColumnWidth>,
        header: Option<TableRow>,
        rows: Vec<TableRow>,
    ) -> Result<Self, PdfModelError> {
        if columns.is_empty() {
            return Err(PdfModelError::EmptyTableColumns);
        }
        let col_count = columns.len();
        if let Some(ref h) = header {
            if h.cells().len() != col_count {
                return Err(PdfModelError::TableCellCountMismatch {
                    cells: h.cells().len(),
                    columns: col_count,
                });
            }
        }
        for row in &rows {
            if row.cells().len() != col_count {
                return Err(PdfModelError::TableCellCountMismatch {
                    cells: row.cells().len(),
                    columns: col_count,
                });
            }
        }
        Ok(Self {
            columns,
            header,
            rows,
        })
    }

    /// Returns the column width specifications.
    #[must_use]
    pub fn columns(&self) -> &[ColumnWidth] {
        &self.columns
    }

    /// Returns the optional header row.
    #[must_use]
    pub fn header(&self) -> Option<&TableRow> {
        self.header.as_ref()
    }

    /// Returns the body rows.
    #[must_use]
    pub fn rows(&self) -> &[TableRow] {
        &self.rows
    }
}

/// Block-level element.
///
/// M2 extends the M1 trio (paragraph / heading / page break) with rich
/// blocks: lists, code, callouts, and tables. Adding a variant requires
/// matching renderer support — the layout engine `match` is exhaustive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Paragraph of inline runs.
    Paragraph(Paragraph),
    /// Outline heading, also used to drive the PDF bookmark tree.
    Heading(Heading),
    /// Forces the next block onto a new page.
    PageBreak,
    /// Ordered or unordered list with optional nested children.
    List(ListBlock),
    /// Monospaced code block with a background fill.
    Code(CodeBlock),
    /// Callout / admonition box.
    Callout(Callout),
    /// Table with optional repeating header.
    Table(TableBlock),
    /// Embedded image.
    Image(ImageBlock),
    /// Table of contents placeholder.
    ///
    /// The renderer replaces this marker with the actual TOC content
    /// after a first layout pass establishes heading page numbers.
    Toc,
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

    #[test]
    fn list_block_rejects_empty_items() {
        assert_eq!(
            ListBlock::new(false, vec![]),
            Err(PdfModelError::EmptyList)
        );
    }

    #[test]
    fn list_item_rejects_empty_runs() {
        assert_eq!(ListItem::new(vec![]), Err(PdfModelError::EmptyListItem));
    }

    #[test]
    fn nested_list_serde_roundtrip_preserves_structure() {
        let inner = ListBlock::new(true, vec![ListItem::plain("a").unwrap()]).unwrap();
        let outer = ListBlock::new(
            false,
            vec![ListItem::plain("top")
                .unwrap()
                .with_children(vec![inner])],
        )
        .unwrap();
        let json = serde_json::to_string(&outer).unwrap();
        let back: ListBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(back, outer);
        assert_eq!(back.items()[0].children().len(), 1);
        assert!(back.items()[0].children()[0].ordered);
    }

    #[test]
    fn code_block_rejects_empty_or_whitespace() {
        assert!(CodeBlock::new("").is_err());
        assert!(CodeBlock::new("   \n\t  ").is_err());
    }

    #[test]
    fn code_block_with_language_roundtrips() {
        let cb = CodeBlock::new("fn main(){}").unwrap().with_language("rust");
        let json = serde_json::to_string(&cb).unwrap();
        assert!(json.contains(r#""language":"rust""#));
        let back: CodeBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code(), "fn main(){}");
        assert_eq!(back.language.as_deref(), Some("rust"));
    }

    #[test]
    fn callout_rejects_empty() {
        assert!(Callout::new("").is_err());
        assert!(Callout::new("   ").is_err());
    }

    #[test]
    fn block_list_code_callout_serde_roundtrip() {
        let blocks = vec![
            Block::List(
                ListBlock::new(false, vec![ListItem::plain("one").unwrap()]).unwrap(),
            ),
            Block::Code(CodeBlock::new("let x = 1;").unwrap()),
            Block::Callout(Callout::new("Heads up.").unwrap().with_title("Note")),
        ];
        for block in &blocks {
            let json = serde_json::to_string(block).unwrap();
            let back: Block = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, block);
        }
    }

    #[test]
    fn table_rejects_empty_columns() {
        assert_eq!(
            TableBlock::new(vec![], None, vec![]).unwrap_err(),
            PdfModelError::EmptyTableColumns
        );
    }

    #[test]
    fn table_rejects_cell_count_mismatch_in_header() {
        let cols = vec![ColumnWidth::auto(), ColumnWidth::auto()];
        let header = TableRow::new(vec![TableCell::plain("only one").unwrap()]);
        let err = TableBlock::new(cols, Some(header), vec![]).unwrap_err();
        assert!(matches!(
            err,
            PdfModelError::TableCellCountMismatch { cells: 1, columns: 2 }
        ));
    }

    #[test]
    fn table_rejects_cell_count_mismatch_in_body() {
        let cols = vec![ColumnWidth::auto()];
        let row = TableRow::new(vec![
            TableCell::plain("a").unwrap(),
            TableCell::plain("b").unwrap(),
        ]);
        let err = TableBlock::new(cols, None, vec![row]).unwrap_err();
        assert!(matches!(
            err,
            PdfModelError::TableCellCountMismatch { cells: 2, columns: 1 }
        ));
    }

    #[test]
    fn table_with_header_serde_roundtrip() {
        let cols = vec![ColumnWidth::fixed(40.0), ColumnWidth::auto()];
        let header = TableRow::new(vec![
            TableCell::plain("Name").unwrap(),
            TableCell::plain("Value").unwrap(),
        ]);
        let body = vec![TableRow::new(vec![
            TableCell::plain("alpha").unwrap(),
            TableCell::plain("1").unwrap(),
        ])];
        let table = TableBlock::new(cols, Some(header), body).unwrap();
        let json = serde_json::to_string(&table).unwrap();
        let back: TableBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(back.columns().len(), 2);
        assert!(back.header().is_some());
        assert_eq!(back.rows().len(), 1);
    }

    #[test]
    fn table_allows_empty_body_rows() {
        let cols = vec![ColumnWidth::auto()];
        let table = TableBlock::new(cols, None, vec![]).unwrap();
        assert!(table.rows().is_empty());
    }

    #[test]
    fn image_block_rejects_empty_data() {
        assert_eq!(
            ImageBlock::new(vec![]).unwrap_err(),
            PdfModelError::EmptyImageData
        );
    }

    #[test]
    fn image_block_stores_data_and_alt() {
        let img = ImageBlock::new(vec![0xFF, 0xD8])
            .unwrap()
            .with_alt("logo")
            .with_width_mm(50.0);
        assert_eq!(img.data(), &[0xFF, 0xD8]);
        assert_eq!(img.alt.as_deref(), Some("logo"));
        assert_eq!(img.width_mm, Some(50.0));
    }

    #[test]
    fn block_table_serde_roundtrip() {
        let table = TableBlock::new(
            vec![ColumnWidth::auto()],
            None,
            vec![TableRow::new(vec![TableCell::plain("x").unwrap()])],
        )
        .unwrap();
        let block = Block::Table(table);
        let json = serde_json::to_string(&block).unwrap();
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(back, block);
    }
}
