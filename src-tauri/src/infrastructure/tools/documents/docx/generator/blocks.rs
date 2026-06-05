//! Dispatch a `PlannedBlock` to the correct emitter and append the
//! resulting paragraphs / tables to the Docx body.

use std::collections::HashMap;

use docx_rust::document::BodyContent;
use docx_rust::Docx;

use crate::core::documents::docx::theme::DocxTheme;
use crate::infrastructure::tools::documents::shared::{DocxContent, DocxDraft, DraftSection};

use super::plan::{Plan, PlannedBlock, PlannedImage};
use super::{images, paragraphs, sectpr, tables};

/// Emit one planned block into the Docx body.
///
/// `image_by_section` maps draft section index to the position in
/// `plan.images`; precomputing it keeps dispatch O(1).
pub fn emit(
    docx: &mut Docx<'_>,
    block: &PlannedBlock,
    draft: &DocxDraft,
    plan: &Plan,
    theme: &DocxTheme,
    image_by_section: &HashMap<usize, usize>,
) {
    match block {
        PlannedBlock::Section(idx) => {
            if let Some(section) = draft.sections.get(*idx) {
                dispatch(docx, section, *idx, plan, theme, image_by_section);
            }
        }
        PlannedBlock::SectionBreakBefore(_idx, patch) => {
            let section_property = sectpr::mid_document(theme, patch, plan);
            let mut para = docx_rust::document::Paragraph::default();
            let mut p_property = docx_rust::formatting::ParagraphProperty::default();
            p_property.section_property = Some(section_property);
            para.property = Some(p_property);
            docx.document.body.push(BodyContent::Paragraph(para));
        }
    }
}

/// Precompute section-index → image-plan-index lookup.
#[must_use]
pub fn image_lookup(draft: &DocxDraft) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    let mut pos = 0;
    for (idx, section) in draft.sections.iter().enumerate() {
        if matches!(section.content, DocxContent::Image { .. }) {
            map.insert(idx, pos);
            pos += 1;
        }
    }
    map
}

fn dispatch(
    docx: &mut Docx<'_>,
    section: &DraftSection,
    idx: usize,
    plan: &Plan,
    theme: &DocxTheme,
    image_by_section: &HashMap<usize, usize>,
) {
    match &section.content {
        DocxContent::Heading {
            level,
            text,
            alignment,
            page_break_before,
        } => {
            let para =
                paragraphs::heading(*level, text, *alignment, *page_break_before, theme, plan);
            docx.document.body.push(BodyContent::Paragraph(para));
        }
        DocxContent::Paragraph {
            text,
            alignment,
            role,
        } => {
            let para = paragraphs::body(text, *role, *alignment, theme, plan);
            docx.document.body.push(BodyContent::Paragraph(para));
        }
        DocxContent::BulletList { items } | DocxContent::NumberedList { items, .. } => {
            if let Some(num_id) = plan.numbering_for_section.get(&idx).copied() {
                for item in items {
                    let para = paragraphs::list_item(item, num_id, theme, plan);
                    docx.document.body.push(BodyContent::Paragraph(para));
                }
            }
        }
        DocxContent::Table {
            headers,
            rows,
            caption,
        } => {
            let table = tables::build(headers, rows, caption.as_deref(), theme, plan);
            docx.document.body.push(BodyContent::Table(table));
            if let Some(caption_text) = caption {
                let caption_para = paragraphs::body(
                    &crate::infrastructure::tools::documents::shared::ParagraphText::Plain(
                        format!("Table: {caption_text}"),
                    ),
                    None,
                    None,
                    theme,
                    plan,
                );
                docx.document
                    .body
                    .push(BodyContent::Paragraph(caption_para));
            }
        }
        DocxContent::Image { caption, .. } => {
            let Some(image_idx) = image_by_section.get(&idx) else {
                return;
            };
            let Some(planned) = plan.images.get(*image_idx) else {
                return;
            };
            let paragraphs_out =
                images::paragraph(planned, caption.as_deref(), *image_idx as i32, theme);
            for p in paragraphs_out {
                docx.document.body.push(BodyContent::Paragraph(p));
            }
        }
        DocxContent::PageBreak => {
            docx.document
                .body
                .push(BodyContent::Paragraph(paragraphs::page_break()));
        }
        DocxContent::HorizontalRule => {
            docx.document
                .body
                .push(BodyContent::Paragraph(paragraphs::horizontal_rule(theme)));
        }
    }
}

/// Unused accessor retained for potential planner callers.
#[allow(dead_code)]
fn _plan_image(plan: &Plan, position: usize) -> Option<&PlannedImage> {
    plan.images.get(position)
}
