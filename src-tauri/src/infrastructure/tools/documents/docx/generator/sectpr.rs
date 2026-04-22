//! Build `<w:sectPr>` blocks from a resolved theme.

use std::borrow::Cow;

use docx_rust::document::{HeaderFooterReference, HeaderFooterReferenceType, FooterReference, HeaderReference};
use docx_rust::formatting::{PageMargin, PageSize, SectionProperty, TitlePage};

use crate::core::documents::docx::theme::{DocxTheme, PaperSize};

use super::plan::Plan;
use super::units::mm_to_twips;

/// Build the document-level section property. References any
/// planned header/footer parts and writes page geometry from the
/// theme.
#[must_use]
pub fn document_level(theme: &DocxTheme, plan: &Plan) -> SectionProperty<'static> {
    let mut section = base_from_theme(theme);
    section.header_footer_references = header_footer_refs(plan);
    if theme.decorations.different_first_page {
        section.title_page = Some(TitlePage {});
    }
    section
}

/// Build a section property for a mid-document break. Page geometry
/// is the base theme merged with the supplied override; header/footer
/// references still point to the document-level parts.
#[must_use]
pub fn mid_document(
    base: &DocxTheme,
    override_patch: &crate::core::documents::docx::theme_patch::DocxThemePatch,
    plan: &Plan,
) -> SectionProperty<'static> {
    let merged = base.clone().with_patch(override_patch);
    let mut section = base_from_theme(&merged);
    section.header_footer_references = header_footer_refs(plan);
    section
}

fn base_from_theme(theme: &DocxTheme) -> SectionProperty<'static> {
    let mut section = SectionProperty::default();
    section.page_size = Some(page_size(theme));
    section.page_margin = Some(page_margin(theme));
    section
}

fn page_size(theme: &DocxTheme) -> PageSize {
    let (mut w, mut h) = match theme.page.paper {
        PaperSize::A4 => PaperSize::A4.twips_portrait(),
        PaperSize::Letter => PaperSize::Letter.twips_portrait(),
        PaperSize::Legal => PaperSize::Legal.twips_portrait(),
    };
    if theme.page.landscape {
        std::mem::swap(&mut w, &mut h);
    }
    PageSize {
        weight: w as isize,
        height: h as isize,
    }
}

fn page_margin(theme: &DocxTheme) -> PageMargin {
    PageMargin {
        top: Some(mm_to_twips(theme.page.margin_top_mm) as isize),
        bottom: Some(mm_to_twips(theme.page.margin_bottom_mm) as isize),
        left: Some(mm_to_twips(theme.page.margin_left_mm) as isize),
        right: Some(mm_to_twips(theme.page.margin_right_mm) as isize),
        header: Some(720),
        footer: Some(720),
        gutter: Some(0),
    }
}

fn header_footer_refs(plan: &Plan) -> Vec<HeaderFooterReference<'static>> {
    let mut refs = Vec::new();
    if let Some(header) = plan.header.as_ref() {
        refs.push(HeaderFooterReference::Header(HeaderReference {
            ty: Some(HeaderFooterReferenceType::Default),
            id: Some(Cow::Owned(header.rel_id.clone())),
        }));
    }
    if let Some(footer) = plan.footer.as_ref() {
        refs.push(HeaderFooterReference::Footer(FooterReference {
            ty: Some(HeaderFooterReferenceType::Default),
            id: Some(Cow::Owned(footer.rel_id.clone())),
        }));
    }
    if let Some(header) = plan.first_page_header.as_ref() {
        refs.push(HeaderFooterReference::Header(HeaderReference {
            ty: Some(HeaderFooterReferenceType::First),
            id: Some(Cow::Owned(header.rel_id.clone())),
        }));
    }
    if let Some(footer) = plan.first_page_footer.as_ref() {
        refs.push(HeaderFooterReference::Footer(FooterReference {
            ty: Some(HeaderFooterReferenceType::First),
            id: Some(Cow::Owned(footer.rel_id.clone())),
        }));
    }
    refs
}
