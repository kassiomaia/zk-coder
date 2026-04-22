//! Assemble a `docx_rust::Paragraph` with appropriate paragraph-level
//! properties (style, alignment, numbering reference, spacing, font
//! overrides for headings).

use std::borrow::Cow;

use docx_rust::document::{Paragraph, ParagraphContent, Run, RunContent};
use docx_rust::formatting::{
    Bold, CharacterProperty, Color, Fonts, IndentLevel, Justification, JustificationVal,
    NumberingId, NumberingProperty, ParagraphProperty, Size, Spacing,
};

use crate::core::documents::docx::theme::DocxTheme;
use crate::infrastructure::tools::documents::shared::{
    Alignment, ListItem, ParagraphRole, ParagraphText,
};

use super::plan::Plan;
use super::runs;
use super::units::{heading_size_pt, line_spacing_to_line_value, pt_to_half_pt, pt_to_twips};

/// Build a styled body paragraph.
#[must_use]
pub fn body(
    text: &ParagraphText,
    role: Option<ParagraphRole>,
    alignment: Option<Alignment>,
    theme: &DocxTheme,
    plan: &Plan,
) -> Paragraph<'static> {
    let mut contents: Vec<ParagraphContent<'static>> = Vec::new();
    runs::emit(text, theme, plan, &mut contents);
    let property = body_property(role, alignment, theme);
    assemble(contents, property)
}

/// Build a heading paragraph. The heading style id (`Heading1`..
/// `Heading9`) is set so Word generates an outline entry; the run
/// size and color come from the theme's typography scale.
#[must_use]
pub fn heading(
    level: u8,
    text: &ParagraphText,
    alignment: Option<Alignment>,
    page_break_before: bool,
    theme: &DocxTheme,
    _plan: &Plan,
) -> Paragraph<'static> {
    let mut contents: Vec<ParagraphContent<'static>> = Vec::new();
    let size_pt = heading_size_pt(
        theme.typography.heading_font_size_pt,
        theme.typography.heading_scale,
        level,
    );
    collect_heading_runs(text, size_pt, theme, &mut contents);

    let mut property = ParagraphProperty::default();
    property.style_id = Some(format!("Heading{}", level.max(1).min(9)).into());
    if page_break_before {
        property.page_break_before = Some(docx_rust::formatting::PageBreakBefore {
            value: Some(true),
        });
    }
    property.justification = alignment.map(alignment_to_justification);
    property.spacing = Some(theme_spacing(theme));
    assemble(contents, property)
}

/// Build one paragraph of a list item. The numbering-property links
/// it to the list's `num_id` via [`PlannedNumbering`](super::plan::PlannedNumbering).
#[must_use]
pub fn list_item(
    item: &ListItem,
    num_id: i32,
    theme: &DocxTheme,
    plan: &Plan,
) -> Paragraph<'static> {
    let mut contents: Vec<ParagraphContent<'static>> = Vec::new();
    runs::emit(&item.text, theme, plan, &mut contents);
    let mut property = body_property(None, None, theme);
    property.numbering = Some(NumberingProperty {
        level: Some(IndentLevel {
            value: i32::from(item.level.unwrap_or(0)) as isize,
        }),
        id: Some(NumberingId::from(num_id as isize)),
        numbering_change: None,
        ins: None,
    });
    property.style_id = Some("ListParagraph".to_string().into());
    assemble(contents, property)
}

/// Build a horizontal-rule paragraph (bottom-bordered empty
/// paragraph, the canonical OOXML idiom).
#[must_use]
pub fn horizontal_rule(theme: &DocxTheme) -> Paragraph<'static> {
    use docx_rust::formatting::{BorderStyle, Borders, BottomBorder};
    let mut borders = Borders::default();
    borders.bottom = Some(BottomBorder {
        color: Some(Cow::Owned(theme.palette.table_border.hex.clone())),
        style: BorderStyle::Single,
        size: Some(6),
        space: Some(1),
        shadow: None,
        frame: None,
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
    });
    let mut property = body_property(None, None, theme);
    property.border = Some(borders);
    assemble(Vec::new(), property)
}

/// Build a page-break-only paragraph.
#[must_use]
pub fn page_break() -> Paragraph<'static> {
    use docx_rust::document::{Break, BreakType};
    let run = Run {
        property: None,
        content: vec![RunContent::Break(Break {
            ty: Some(BreakType::Page),
        })],
        ..Run::default()
    };
    Paragraph {
        property: None,
        content: vec![ParagraphContent::Run(run)],
        ..Paragraph::default()
    }
}

fn collect_heading_runs(
    text: &ParagraphText,
    size_pt: u16,
    theme: &DocxTheme,
    out: &mut Vec<ParagraphContent<'static>>,
) {
    let plain = match text {
        ParagraphText::Plain(s) => s.clone(),
        ParagraphText::Rich(runs) => runs.iter().map(|r| r.text.clone()).collect(),
    };
    if plain.is_empty() {
        return;
    }
    let mut props = CharacterProperty::default();
    props.fonts = Some(heading_font_set(&theme.fonts.heading));
    props.color = Some(Color {
        value: Cow::Owned(theme.palette.heading.hex.clone()),
    });
    props.size = Some(Size::from(pt_to_half_pt(size_pt) as isize));
    props.bold = Some(Bold { value: Some(true) });
    out.push(ParagraphContent::Run(Run {
        property: Some(props),
        content: vec![RunContent::Text(
            docx_rust::document::Text::from((plain, docx_rust::document::TextSpace::Preserve)),
        )],
        ..Run::default()
    }));
}

fn heading_font_set(family: &str) -> Fonts {
    Fonts::default()
        .ascii(family.to_string())
        .east_asia(family.to_string())
        .h_ansi(family.to_string())
        .custom(family.to_string())
}

fn body_property(
    role: Option<ParagraphRole>,
    alignment: Option<Alignment>,
    theme: &DocxTheme,
) -> ParagraphProperty<'static> {
    let mut property = ParagraphProperty::default();
    property.spacing = Some(theme_spacing(theme));
    property.justification = alignment.map(alignment_to_justification);
    match role {
        Some(ParagraphRole::Quote) => {
            property.style_id = Some("Quote".to_string().into());
        }
        Some(ParagraphRole::Code) => {
            property.style_id = Some("CodeBlock".to_string().into());
        }
        _ => {}
    }
    property
}

fn theme_spacing(theme: &DocxTheme) -> Spacing {
    Spacing {
        before_lines: None,
        before: Some(pt_to_twips(theme.spacing.paragraph_space_before_pt) as isize),
        before_auto_spacing: None,
        after_lines: None,
        after: Some(pt_to_twips(theme.spacing.paragraph_space_after_pt) as isize),
        after_auto_spacing: None,
        line: Some(line_spacing_to_line_value(theme.typography.line_spacing) as isize),
        line_rule: None,
    }
}

fn alignment_to_justification(alignment: Alignment) -> Justification {
    let val = match alignment {
        Alignment::Left => JustificationVal::Left,
        Alignment::Center => JustificationVal::Center,
        Alignment::Right => JustificationVal::Right,
        Alignment::Justify => JustificationVal::Both,
    };
    Justification::from(val)
}

fn assemble(
    content: Vec<ParagraphContent<'static>>,
    property: ParagraphProperty<'static>,
) -> Paragraph<'static> {
    Paragraph {
        property: Some(property),
        content,
        ..Paragraph::default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn heading_has_style_id_matching_level() {
        let para = heading(
            2,
            &ParagraphText::Plain("x".into()),
            None,
            false,
            &DocxTheme::report_default(),
            &Plan::default(),
        );
        let style = para.property.as_ref().and_then(|p| p.style_id.as_ref()).unwrap();
        assert_eq!(style.value, "Heading2");
    }

    #[test]
    fn body_applies_alignment() {
        let para = body(
            &ParagraphText::Plain("x".into()),
            None,
            Some(Alignment::Center),
            &DocxTheme::report_default(),
            &Plan::default(),
        );
        let jc = para
            .property
            .as_ref()
            .and_then(|p| p.justification.as_ref())
            .unwrap();
        matches!(jc.value, JustificationVal::Center);
    }

    #[test]
    fn horizontal_rule_sets_bottom_border() {
        let para = horizontal_rule(&DocxTheme::report_default());
        let borders = para.property.as_ref().and_then(|p| p.border.as_ref()).unwrap();
        assert!(borders.bottom.is_some());
    }
}
