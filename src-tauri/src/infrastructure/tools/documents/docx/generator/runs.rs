//! Convert [`ParagraphText`] and [`TextRun`] into `docx_rust` runs.

use std::borrow::Cow;

use docx_rust::document::{Hyperlink, ParagraphContent, Run, RunContent, Text, TextSpace};
use docx_rust::formatting::{
    Bold, CharacterProperty, Color, Fonts, Italics, Shading, ShadingStyle, Size, Strike, Underline,
    UnderlineStyle,
};

use crate::core::documents::docx::theme::DocxTheme;
use crate::infrastructure::tools::documents::shared::{ParagraphText, TextRun};

use super::plan::Plan;
use super::units::pt_to_half_pt;

/// Emit runs for a paragraph, using the document theme for default
/// font, color, and code-block shading.
pub fn emit(
    text: &ParagraphText,
    theme: &DocxTheme,
    plan: &Plan,
    out: &mut Vec<ParagraphContent<'static>>,
) {
    match text {
        ParagraphText::Plain(s) => {
            if !s.is_empty() {
                out.push(ParagraphContent::Run(plain_run(s.clone(), theme)));
            }
        }
        ParagraphText::Rich(runs) => {
            for run in runs {
                if let Some(link) = run.link.as_deref() {
                    out.push(ParagraphContent::Link(hyperlink_run(run, link, theme, plan)));
                } else {
                    out.push(ParagraphContent::Run(styled_run(run, theme)));
                }
            }
        }
    }
}

fn plain_run(text: String, theme: &DocxTheme) -> Run<'static> {
    let mut props = CharacterProperty::default();
    props.fonts = Some(font_set(&theme.fonts.body));
    props.color = Some(Color {
        value: Cow::Owned(theme.palette.text.hex.clone()),
    });
    props.size = Some(Size::from(pt_to_half_pt(theme.typography.body_font_size_pt) as isize));
    Run {
        property: Some(props),
        content: vec![RunContent::Text(preserved_text(text))],
        ..Run::default()
    }
}

fn styled_run(run: &TextRun, theme: &DocxTheme) -> Run<'static> {
    let mut props = CharacterProperty::default();
    let family = if run.code {
        theme.fonts.mono.clone()
    } else {
        theme.fonts.body.clone()
    };
    props.fonts = Some(font_set(&family));
    let color = run
        .color
        .as_deref()
        .map(|c| c.trim_start_matches('#').to_ascii_lowercase())
        .unwrap_or_else(|| theme.palette.text.hex.clone());
    props.color = Some(Color {
        value: Cow::Owned(color),
    });
    props.size = Some(Size::from(pt_to_half_pt(theme.typography.body_font_size_pt) as isize));
    if run.bold {
        props.bold = Some(Bold { value: Some(true) });
    }
    if run.italic {
        props.italics = Some(Italics { value: Some(true) });
    }
    if run.strike {
        props.strike = Some(Strike { value: Some(true) });
    }
    if run.underline {
        props.underline = Some(Underline {
            color: None,
            val: Some(UnderlineStyle::Single),
        });
    }
    if run.code {
        props.shading = Some(Shading {
            style: Some(ShadingStyle::Clear),
            color: Some(Cow::Owned(theme.palette.code_text.hex.clone())),
            theme_color: None,
            theme_tint: None,
            theme_shade: None,
            fill: Some(Cow::Owned(theme.palette.code_bg.hex.clone())),
            theme_fill: None,
            theme_fill_tint: None,
            theme_fill_shade: None,
        });
    }
    Run {
        property: Some(props),
        content: vec![RunContent::Text(preserved_text(run.text.clone()))],
        ..Run::default()
    }
}

fn hyperlink_run(
    run: &TextRun,
    _link: &str,
    theme: &DocxTheme,
    _plan: &Plan,
) -> Hyperlink<'static> {
    // External hyperlinks require a relationship id in
    // `document.xml.rels`. Allocating those requires a mutable pass
    // alongside the media plan; for this iteration we render the
    // visible text in the link color so the document still reads
    // correctly, and defer live-link rendering to a follow-up once
    // the rels-allocator is shared across images and links.
    let mut styled = styled_run(run, theme);
    if let Some(props) = styled.property.as_mut() {
        if run.color.is_none() {
            if let Some(color) = props.color.as_mut() {
                color.value = Cow::Owned(theme.palette.link.hex.clone());
            }
        }
        if !run.underline {
            props.underline = Some(Underline {
                color: None,
                val: Some(UnderlineStyle::Single),
            });
        }
    }
    Hyperlink {
        id: None,
        anchor: None,
        content: Some(styled),
        bidirectional_embedding: None,
    }
}

fn font_set(family: &str) -> Fonts {
    Fonts::default()
        .ascii(family.to_string())
        .east_asia(family.to_string())
        .h_ansi(family.to_string())
        .custom(family.to_string())
}

fn preserved_text(s: String) -> Text<'static> {
    Text::from((s, TextSpace::Preserve))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn theme() -> DocxTheme {
        DocxTheme::report_default()
    }

    #[test]
    fn plain_text_emits_one_run() {
        let mut out = Vec::new();
        emit(
            &ParagraphText::Plain("hello".into()),
            &theme(),
            &Plan::default(),
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0], ParagraphContent::Run(_)));
    }

    #[test]
    fn rich_text_emits_one_run_per_span() {
        let text = ParagraphText::Rich(vec![
            TextRun { text: "a".into(), bold: true, ..Default::default() },
            TextRun { text: "b".into(), italic: true, ..Default::default() },
        ]);
        let mut out = Vec::new();
        emit(&text, &theme(), &Plan::default(), &mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn hyperlink_run_wraps_text_in_link() {
        let text = ParagraphText::Rich(vec![TextRun {
            text: "docs".into(),
            link: Some("https://example.com".into()),
            ..Default::default()
        }]);
        let mut out = Vec::new();
        emit(&text, &theme(), &Plan::default(), &mut out);
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0], ParagraphContent::Link(_)));
    }

    #[test]
    fn empty_plain_string_produces_no_run() {
        let mut out = Vec::new();
        emit(
            &ParagraphText::Plain(String::new()),
            &theme(),
            &Plan::default(),
            &mut out,
        );
        assert!(out.is_empty());
    }
}
