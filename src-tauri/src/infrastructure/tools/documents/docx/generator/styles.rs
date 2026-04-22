//! Populate `word/styles.xml` with paragraph styles the generator
//! references (`Heading1`..`Heading9`, `ListParagraph`, `Quote`,
//! `CodeBlock`, `TableHeader`).

use std::borrow::Cow;

use docx_rust::formatting::{Bold, CharacterProperty, Color, Fonts, Italics, ParagraphProperty, Size, Spacing};
use docx_rust::styles::{Style, StyleType};
use docx_rust::Docx;

use crate::core::documents::docx::theme::DocxTheme;

use super::units::{heading_size_pt, line_spacing_to_line_value, pt_to_half_pt, pt_to_twips};

/// Append the full style set to `docx.styles`.
pub fn install(docx: &mut Docx<'_>, theme: &DocxTheme) {
    for level in 1..=9 {
        docx.styles.styles.push(heading_style(level, theme));
    }
    docx.styles.styles.push(list_paragraph_style(theme));
    docx.styles.styles.push(quote_style(theme));
    docx.styles.styles.push(code_block_style(theme));
    docx.styles.styles.push(table_header_style(theme));
}

fn heading_style(level: u8, theme: &DocxTheme) -> Style<'static> {
    let size = heading_size_pt(
        theme.typography.heading_font_size_pt,
        theme.typography.heading_scale,
        level,
    );
    let mut style = Style::new(StyleType::Paragraph, format!("Heading{level}"));
    style.name = Some(docx_rust::styles::StyleName {
        value: Cow::Owned(format!("heading {level}")),
    });
    style.default = Some(false);
    let mut char_props = CharacterProperty::default();
    char_props.fonts = Some(font_set(&theme.fonts.heading));
    char_props.color = Some(Color {
        value: Cow::Owned(theme.palette.heading.hex.clone()),
    });
    char_props.size = Some(Size::from(pt_to_half_pt(size) as isize));
    char_props.bold = Some(Bold { value: Some(true) });
    style.character = Some(char_props);

    let mut para_props = ParagraphProperty::default();
    para_props.spacing = Some(Spacing {
        before: Some((pt_to_twips(theme.spacing.paragraph_space_before_pt) + 120) as isize),
        after: Some((pt_to_twips(theme.spacing.paragraph_space_after_pt) + 80) as isize),
        line: Some(line_spacing_to_line_value(theme.typography.line_spacing) as isize),
        line_rule: None,
        ..Spacing::default()
    });
    style.paragraph = Some(para_props);
    style
}

fn list_paragraph_style(theme: &DocxTheme) -> Style<'static> {
    let mut style = Style::new(StyleType::Paragraph, "ListParagraph");
    style.name = Some(docx_rust::styles::StyleName {
        value: Cow::Borrowed("List Paragraph"),
    });
    let mut para_props = ParagraphProperty::default();
    para_props.spacing = Some(theme_spacing(theme));
    style.paragraph = Some(para_props);
    style
}

fn quote_style(theme: &DocxTheme) -> Style<'static> {
    let mut style = Style::new(StyleType::Paragraph, "Quote");
    style.name = Some(docx_rust::styles::StyleName {
        value: Cow::Borrowed("Quote"),
    });
    let mut char_props = CharacterProperty::default();
    char_props.italics = Some(Italics { value: Some(true) });
    char_props.fonts = Some(font_set(&theme.fonts.body));
    style.character = Some(char_props);
    let mut para_props = ParagraphProperty::default();
    para_props.spacing = Some(theme_spacing(theme));
    style.paragraph = Some(para_props);
    style
}

fn code_block_style(theme: &DocxTheme) -> Style<'static> {
    let mut style = Style::new(StyleType::Paragraph, "CodeBlock");
    style.name = Some(docx_rust::styles::StyleName {
        value: Cow::Borrowed("Code Block"),
    });
    let mut char_props = CharacterProperty::default();
    char_props.fonts = Some(font_set(&theme.fonts.mono));
    char_props.color = Some(Color {
        value: Cow::Owned(theme.palette.code_text.hex.clone()),
    });
    char_props.size = Some(Size::from(
        pt_to_half_pt(theme.typography.body_font_size_pt) as isize,
    ));
    style.character = Some(char_props);
    style
}

fn table_header_style(theme: &DocxTheme) -> Style<'static> {
    let mut style = Style::new(StyleType::Paragraph, "TableHeader");
    style.name = Some(docx_rust::styles::StyleName {
        value: Cow::Borrowed("Table Header"),
    });
    let mut char_props = CharacterProperty::default();
    char_props.fonts = Some(font_set(&theme.fonts.body));
    char_props.bold = Some(Bold { value: Some(true) });
    char_props.size = Some(Size::from(
        pt_to_half_pt(theme.typography.body_font_size_pt) as isize,
    ));
    style.character = Some(char_props);
    style
}

fn font_set(family: &str) -> Fonts {
    Fonts::default()
        .ascii(family.to_string())
        .east_asia(family.to_string())
        .h_ansi(family.to_string())
        .custom(family.to_string())
}

fn theme_spacing(theme: &DocxTheme) -> Spacing {
    Spacing {
        before: Some(pt_to_twips(theme.spacing.paragraph_space_before_pt) as isize),
        after: Some(pt_to_twips(theme.spacing.paragraph_space_after_pt) as isize),
        line: Some(line_spacing_to_line_value(theme.typography.line_spacing) as isize),
        line_rule: None,
        ..Spacing::default()
    }
}
