//! Planning pass: walk the draft and collect everything the writer
//! needs to own (image bytes, numbering ids, header/footer parts,
//! section breaks) before emitting OOXML.

use std::path::PathBuf;

use crate::core::documents::docx::theme::{Decorations, DocxTheme};
use crate::core::documents::docx::theme_patch::DocxThemePatch;
use crate::infrastructure::tools::documents::shared::{DocxContent, DocxDraft};
use crate::infrastructure::tools::error::ToolError;

/// Unique relationship id namespace.
const REL_BASE: usize = 100;

/// One image referenced by the document, resolved to its on-disk
/// bytes and pixel dimensions.
#[derive(Debug, Clone)]
pub struct PlannedImage {
    /// `rId` assigned in `word/_rels/document.xml.rels`.
    pub rel_id: String,
    /// Media part filename (e.g. `image1.png`).
    pub filename: String,
    /// Image bytes. Owned so the `&Vec<u8>` handed to `Docx.media`
    /// lives as long as the plan.
    pub bytes: Vec<u8>,
    /// Natural pixel width (for aspect-ratio preservation).
    pub width_px: u32,
    /// Natural pixel height.
    pub height_px: u32,
    /// Requested render width in millimetres.
    pub width_mm: f32,
}

/// Numbering definition actually used by the document. Every list
/// block allocates a fresh `num_id` so the LLM can trust that
/// sequential lists restart.
#[derive(Debug, Clone)]
pub struct PlannedNumbering {
    /// `<w:num w:numId="..">`.
    pub num_id: i32,
    /// `<w:abstractNum w:abstractNumId="..">`.
    pub abstract_id: i32,
    /// `true` for bullets, `false` for arabic numerals.
    pub bullet: bool,
    /// First number, ignored when `bullet` is `true`.
    pub start_at: u32,
}

/// A block to render. Carries pre-resolved indices (image plan index,
/// numbering id) so the writer does not have to revisit the plan.
#[derive(Debug, Clone)]
pub enum PlannedBlock {
    /// Regular draft section, keyed back to its index.
    Section(usize),
    /// A new OOXML section break starts here, with the resolved theme
    /// override. Inserted before the section whose `theme_override`
    /// introduced the break.
    SectionBreakBefore(usize, DocxThemePatch),
}

/// Planned header or footer part, pre-rendered with tokens kept
/// unsubstituted (token substitution happens at write time so
/// `{page}` / `{total}` can use the live page-number field).
#[derive(Debug, Clone)]
pub struct PlannedDecoration {
    /// Part filename inside the zip (e.g. `header1.xml`).
    pub filename: String,
    /// Rel id in `word/_rels/document.xml.rels`.
    pub rel_id: String,
    /// Raw template text as supplied by the theme.
    pub template: String,
}

/// All per-document planning output.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Blocks to emit, in order.
    pub blocks: Vec<PlannedBlock>,
    /// Images in the order they appear in the draft.
    pub images: Vec<PlannedImage>,
    /// Numbering definitions used by lists in the draft.
    pub numbering: Vec<PlannedNumbering>,
    /// Per-section-index lookup into `numbering`.
    pub numbering_for_section: std::collections::HashMap<usize, i32>,
    /// Header part, if the theme has a header template.
    pub header: Option<PlannedDecoration>,
    /// Footer part, if the theme has a footer template.
    pub footer: Option<PlannedDecoration>,
    /// First-page-only empty header, written only when
    /// `Decorations::different_first_page` is set.
    pub first_page_header: Option<PlannedDecoration>,
    /// First-page-only empty footer.
    pub first_page_footer: Option<PlannedDecoration>,
    /// Title, copied off the draft for convenience.
    pub title: String,
}

impl Plan {
    /// Build the plan. Reads image files from disk.
    ///
    /// # Errors
    /// Returns [`ToolError`] when an image cannot be read or decoded.
    pub fn build(draft: &DocxDraft, theme: &DocxTheme) -> Result<Self, ToolError> {
        let mut plan = Plan {
            title: draft.title.clone(),
            ..Plan::default()
        };
        plan_decorations(&mut plan, &theme.decorations);

        let mut next_rel = REL_BASE;
        let mut next_abstract = 0_i32;
        let mut next_num = 1_i32;

        for (idx, section) in draft.sections.iter().enumerate() {
            if section.theme_override.is_some() && idx > 0 {
                plan.blocks.push(PlannedBlock::SectionBreakBefore(
                    idx,
                    section.theme_override.clone().unwrap_or_default(),
                ));
            }

            match &section.content {
                DocxContent::Image { path, width_mm, .. } => {
                    let planned = read_image(
                        next_rel,
                        PathBuf::from(path),
                        width_mm.unwrap_or(120.0),
                        plan.images.len(),
                    )?;
                    next_rel += 1;
                    plan.images.push(planned);
                }
                DocxContent::BulletList { .. } => {
                    plan.numbering.push(PlannedNumbering {
                        num_id: next_num,
                        abstract_id: next_abstract,
                        bullet: true,
                        start_at: 1,
                    });
                    plan.numbering_for_section.insert(idx, next_num);
                    next_num += 1;
                    next_abstract += 1;
                }
                DocxContent::NumberedList { start_at, .. } => {
                    plan.numbering.push(PlannedNumbering {
                        num_id: next_num,
                        abstract_id: next_abstract,
                        bullet: false,
                        start_at: start_at.unwrap_or(1),
                    });
                    plan.numbering_for_section.insert(idx, next_num);
                    next_num += 1;
                    next_abstract += 1;
                }
                _ => {}
            }

            plan.blocks.push(PlannedBlock::Section(idx));
        }

        Ok(plan)
    }

}

fn plan_decorations(plan: &mut Plan, decorations: &Decorations) {
    let mut next = 200_usize;
    if let Some(template) = decorations.header.as_ref() {
        let rel_id = format!("rId{next}");
        next += 1;
        plan.header = Some(PlannedDecoration {
            filename: "header1.xml".to_string(),
            rel_id,
            template: template.clone(),
        });
    }
    if let Some(template) = decorations.footer.as_ref() {
        let rel_id = format!("rId{next}");
        next += 1;
        plan.footer = Some(PlannedDecoration {
            filename: "footer1.xml".to_string(),
            rel_id,
            template: template.clone(),
        });
    }
    if decorations.different_first_page
        && (decorations.header.is_some() || decorations.footer.is_some())
    {
        if decorations.header.is_some() {
            let rel_id = format!("rId{next}");
            next += 1;
            plan.first_page_header = Some(PlannedDecoration {
                filename: "header2.xml".to_string(),
                rel_id,
                template: String::new(),
            });
        }
        if decorations.footer.is_some() {
            let rel_id = format!("rId{next}");
            plan.first_page_footer = Some(PlannedDecoration {
                filename: "footer2.xml".to_string(),
                rel_id,
                template: String::new(),
            });
        }
    }
}

fn read_image(
    rel_seed: usize,
    path: PathBuf,
    width_mm: f32,
    plan_index: usize,
) -> Result<PlannedImage, ToolError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ToolError::failed(format!(
            "Failed to read image '{}': {e}",
            path.display()
        ))
    })?;
    let decoded = image::load_from_memory(&bytes).map_err(|e| {
        ToolError::failed(format!(
            "Failed to decode image '{}': {e}",
            path.display()
        ))
    })?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());
    let ext = match extension.as_str() {
        "jpg" | "jpeg" => "jpeg",
        _ => "png",
    };
    let filename = format!("media/image{}.{}", plan_index + 1, ext);
    Ok(PlannedImage {
        rel_id: format!("rId{rel_seed}"),
        filename,
        width_px: decoded.width(),
        height_px: decoded.height(),
        width_mm,
        bytes,
    })
}
