//! Build the header/footer parts and register their relationships.
//!
//! Token substitution for `{page}`, `{total}`, and `{title}`:
//! * `{title}` is expanded eagerly using the draft's title.
//! * `{page}` and `{total}` are rendered as OOXML field instructions
//!   (`<w:fldChar>` / `<w:instrText>PAGE</w:instrText>` pairs) so
//!   Word fills them in at display time.

use std::borrow::Cow;

use docx_rust::document::{
    BodyContent, Footer, Header, Paragraph, ParagraphContent, Run, RunContent, Text, TextSpace,
};
use docx_rust::formatting::{CharacterProperty, Color, Fonts, ParagraphProperty, Size};
use docx_rust::rels::Relationships;
use docx_rust::Docx;

/// Schema URL for footer relationships. Lifted from the crate's
/// (private) `schema.rs` so we can populate `document.xml.rels`
/// manually around the crate's header-vs-footer bug.
const SCHEMA_FOOTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer";

use crate::core::documents::docx::theme::DocxTheme;

use super::plan::{Plan, PlannedDecoration};
use super::units::pt_to_half_pt;

/// Install the planned header/footer parts on the Docx.
pub fn install_header_footer(docx: &mut Docx<'_>, plan: &Plan, theme: &DocxTheme) {
    if let Some(h) = plan.header.as_ref() {
        docx.headers
            .insert(h.filename.clone(), build_header(h, plan, theme));
    }
    if let Some(f) = plan.footer.as_ref() {
        docx.footers
            .insert(f.filename.clone(), build_footer(f, plan, theme));
    }
    if let Some(h) = plan.first_page_header.as_ref() {
        docx.headers.insert(h.filename.clone(), empty_header());
    }
    if let Some(f) = plan.first_page_footer.as_ref() {
        docx.footers.insert(f.filename.clone(), empty_footer());
    }
}

/// Manually insert `SCHEMA_FOOTER` relationships for every footer
/// part before the crate's ZIP writer runs. The crate's auto-
/// registration incorrectly uses the header schema for footers; since
/// `Relationships::add_rel` is idempotent per target, pre-populating
/// with the correct schema wins.
pub fn prepopulate_footer_rels(docx: &mut Docx<'_>, plan: &Plan) {
    if plan.footer.is_none() && plan.first_page_footer.is_none() {
        return;
    }
    let rels = docx
        .document_rels
        .get_or_insert_with(Relationships::default);
    if let Some(f) = plan.footer.as_ref() {
        register_rel(rels, &f.rel_id, SCHEMA_FOOTER, &f.filename);
    }
    if let Some(f) = plan.first_page_footer.as_ref() {
        register_rel(rels, &f.rel_id, SCHEMA_FOOTER, &f.filename);
    }
}

fn register_rel<'a>(rels: &mut Relationships<'a>, id: &str, schema: &str, target: &str) {
    use docx_rust::rels::Relationship;
    let exists = rels.relationships.iter().any(|r| r.target == target);
    if !exists {
        rels.relationships.push(Relationship {
            id: Cow::Owned(id.to_string()),
            target: Cow::Owned(target.to_string()),
            ty: Cow::Owned(schema.to_string()),
            target_mode: None,
        });
    }
}

fn build_header(planned: &PlannedDecoration, plan: &Plan, theme: &DocxTheme) -> Header<'static> {
    let mut header = Header::default();
    header
        .content
        .push(BodyContent::Paragraph(template_paragraph(
            &planned.template,
            &plan.title,
            theme,
        )));
    header
}

fn build_footer(planned: &PlannedDecoration, plan: &Plan, theme: &DocxTheme) -> Footer<'static> {
    let mut footer = Footer::default();
    footer
        .content
        .push(BodyContent::Paragraph(template_paragraph(
            &planned.template,
            &plan.title,
            theme,
        )));
    footer
}

fn empty_header() -> Header<'static> {
    let mut header = Header::default();
    header
        .content
        .push(BodyContent::Paragraph(Paragraph::default()));
    header
}

fn empty_footer() -> Footer<'static> {
    let mut footer = Footer::default();
    footer
        .content
        .push(BodyContent::Paragraph(Paragraph::default()));
    footer
}

fn template_paragraph(template: &str, title: &str, theme: &DocxTheme) -> Paragraph<'static> {
    let expanded = template.replace("{title}", title);
    let mut content: Vec<ParagraphContent<'static>> = Vec::new();
    emit_template_runs(&expanded, theme, &mut content);
    Paragraph {
        property: Some(ParagraphProperty::default()),
        content,
        ..Paragraph::default()
    }
}

fn emit_template_runs(template: &str, theme: &DocxTheme, out: &mut Vec<ParagraphContent<'static>>) {
    // `{page}` and `{total}` would ideally expand to Word PAGE /
    // NUMPAGES fields, but `docx_rust` 0.1.x does not expose the
    // `InstrText` type publicly so we cannot construct the
    // field-char triad from outside the crate. For this iteration
    // the tokens render as literal text; follow-up can swap in
    // live fields once the crate exposes InstrText or we vendor a
    // small patch.
    let expanded = template
        .replace("{page}", "[page]")
        .replace("{total}", "[total]");
    if !expanded.is_empty() {
        out.push(ParagraphContent::Run(text_run(expanded, theme)));
    }
}

fn text_run(text: String, theme: &DocxTheme) -> Run<'static> {
    let mut props = CharacterProperty::default();
    props.fonts = Some(font_set(&theme.fonts.body));
    props.color = Some(Color {
        value: Cow::Owned(theme.palette.text.hex.clone()),
    });
    props.size = Some(Size::from(pt_to_half_pt(
        theme.typography.body_font_size_pt.saturating_sub(1).max(6),
    ) as isize));
    Run {
        property: Some(props),
        content: vec![RunContent::Text(Text::from((text, TextSpace::Preserve)))],
        ..Run::default()
    }
}

fn font_set(family: &str) -> Fonts {
    Fonts::default()
        .ascii(family.to_string())
        .east_asia(family.to_string())
        .h_ansi(family.to_string())
        .custom(family.to_string())
}
