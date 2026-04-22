//! Render a `DocxContent::Table` into `docx_rust::Table`.

use std::borrow::Cow;

use docx_rust::document::{
    Paragraph as DocxParagraph, ParagraphContent, Table, TableCell, TableRow,
};
use docx_rust::formatting::{
    BorderStyle, ParagraphProperty, TableBorders as WTableBorders, TableProperty, TableWidth,
    TableWidthUnit,
};

use crate::core::documents::docx::theme::DocxTheme;
use crate::infrastructure::tools::documents::shared::ParagraphText;

use super::plan::Plan;
use super::runs;

/// Build a table from the draft's headers and rows. The first row
/// (headers) is shaded with the theme's `table_header_bg` and bold.
#[must_use]
pub fn build(
    headers: &[ParagraphText],
    rows: &[Vec<ParagraphText>],
    _caption: Option<&str>,
    theme: &DocxTheme,
    plan: &Plan,
) -> Table<'static> {
    let mut table = Table::default();
    table.property = table_property(theme);
    table.grids = uniform_grid(headers.len());

    table.rows.push(row_from_cells(headers, true, theme, plan));
    for row in rows {
        table.rows.push(row_from_cells(row, false, theme, plan));
    }
    table
}

fn table_property(theme: &DocxTheme) -> TableProperty<'static> {
    let mut prop = TableProperty::default();
    prop.width = Some(TableWidth {
        value: Some(5000),
        unit: Some(TableWidthUnit::Pct),
    });
    prop.borders = Some(table_borders(theme));
    prop
}

fn table_borders(theme: &DocxTheme) -> WTableBorders<'static> {
    use docx_rust::formatting::{
        InsideHorizonBorder, InsideVerticalBorder, BottomBorder, LeftBorder, RightBorder,
        TopBorder,
    };
    let color: Cow<'static, str> = Cow::Owned(theme.palette.table_border.hex.clone());
    let make_top = || TopBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    let make_bottom = || BottomBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    let make_left = || LeftBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    let make_right = || RightBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    let make_inside_h = || InsideHorizonBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    let make_inside_v = || InsideVerticalBorder {
        color: Some(color.clone()),
        style: BorderStyle::Single,
        size: Some(4),
        space: Some(0),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    };
    WTableBorders {
        top: Some(make_top()),
        bottom: Some(make_bottom()),
        left: Some(make_left()),
        right: Some(make_right()),
        inside_horizon: Some(make_inside_h()),
        inside_vertical: Some(make_inside_v()),
    }
}

fn uniform_grid(columns: usize) -> docx_rust::document::TableGrid {
    let cols: Vec<isize> = (0..columns).map(|_| 2500).collect();
    docx_rust::document::TableGrid::from(cols)
}

fn row_from_cells(
    cells: &[ParagraphText],
    header: bool,
    theme: &DocxTheme,
    plan: &Plan,
) -> TableRow<'static> {
    let mut row = TableRow::default();
    for cell_text in cells {
        row.cells
            .push(build_cell(cell_text, header, theme, plan).into());
    }
    row
}

fn build_cell(
    text: &ParagraphText,
    header: bool,
    theme: &DocxTheme,
    plan: &Plan,
) -> TableCell<'static> {
    let mut contents: Vec<ParagraphContent<'static>> = Vec::new();
    runs::emit(text, theme, plan, &mut contents);
    let mut property = ParagraphProperty::default();
    if header {
        property.style_id = Some("TableHeader".to_string().into());
    }
    let paragraph = DocxParagraph {
        property: Some(property),
        content: contents,
        ..DocxParagraph::default()
    };
    TableCell::paragraph(paragraph)
}
