//! Emit a `.docx` file from a finalized [`DocxDraft`].
//!
//! Rendering is a two-phase pipeline:
//!
//! 1. **Plan**: walk the draft, resolve the theme, and collect every
//!    block that needs a relationship id (images, headers, footers,
//!    hyperlinks) plus the numbering definitions that the lists in
//!    the document actually use.
//! 2. **Write**: translate each block into `docx_rust` types and hand
//!    the assembled [`Docx`] to the crate's ZIP writer.
//!
//! The intermediate [`Plan`] owns every byte slice and owned string so
//! the `Docx<'a>` we borrow from it at write time has a stable
//! lifetime.

use std::path::{Path, PathBuf};

use docx_rust::Docx;

use crate::core::documents::docx::theme::DocxTheme;
use crate::infrastructure::tools::documents::shared::DocxDraft;
use crate::infrastructure::tools::error::ToolError;

mod blocks;
mod content_types;
mod decorations;
mod images;
mod lists;
mod paragraphs;
mod plan;
mod runs;
mod sectpr;
mod styles;
mod tables;
mod units;

pub use plan::Plan;

/// Resolve the effective theme by merging the draft's patch onto the
/// bundled report default.
#[must_use]
pub fn effective_theme(draft: &DocxDraft) -> DocxTheme {
    let base = DocxTheme::report_default();
    match draft.theme_patch.as_ref() {
        Some(patch) => base.with_patch(patch),
        None => base,
    }
}

/// Generate the `.docx` file and return its path and size in bytes.
///
/// # Errors
/// Returns [`ToolError`] if any image fails to load, the draft is
/// inconsistent, or the ZIP write fails.
pub fn generate_docx(draft: &DocxDraft, output_path: &Path) -> Result<(PathBuf, u64), ToolError> {
    let theme = effective_theme(draft);
    let plan = Plan::build(draft, &theme)?;
    let path = output_path.to_path_buf();
    write_docx(draft, &plan, &theme, &path)?;
    let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok((path, size_bytes))
}

fn write_docx(
    draft: &DocxDraft,
    plan: &Plan,
    theme: &DocxTheme,
    output_path: &Path,
) -> Result<(), ToolError> {
    let mut docx = Docx::default();

    styles::install(&mut docx, theme);
    if !plan.numbering.is_empty() {
        lists::install_numbering(&mut docx, &plan.numbering);
    }
    images::install_media(&mut docx, &plan.images);
    decorations::install_header_footer(&mut docx, plan, theme);
    content_types::populate(&mut docx, plan);

    let image_lookup = blocks::image_lookup(draft);
    for block in &plan.blocks {
        blocks::emit(&mut docx, block, draft, plan, theme, &image_lookup);
    }

    docx.document
        .body
        .push(docx_rust::document::BodyContent::SectionProperty(
            sectpr::document_level(theme, plan),
        ));

    // Pre-populate document rels with the correct schema for footers.
    // The crate's `write()` then sees the target is already registered
    // and skips its buggy SCHEMA_HEADER registration for footer files.
    decorations::prepopulate_footer_rels(&mut docx, plan);

    docx.write_file(output_path)
        .map_err(|e| ToolError::failed(format!("Failed to write DOCX file: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::core::documents::docx::theme_patch::{
        DocxThemePatch, PagePatch, TypographyPatch,
    };
    use crate::infrastructure::tools::documents::shared::{
        DocxContent, DraftSection, DraftStatus, ParagraphText,
    };
    use chrono::Utc;

    fn trivial_draft(patch: Option<DocxThemePatch>) -> DocxDraft {
        DocxDraft {
            document_id: "t".into(),
            filename: "t.docx".into(),
            title: "Title".into(),
            sections: vec![DraftSection {
                id: "s".into(),
                index: 0,
                content: DocxContent::Heading {
                    level: 1,
                    text: ParagraphText::Plain("Hello".into()),
                    alignment: None,
                    page_break_before: false,
                },
                theme_override: None,
            }],
            theme_patch: patch,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn effective_theme_without_patch_is_report_default() {
        let theme = effective_theme(&trivial_draft(None));
        assert_eq!(theme, DocxTheme::report_default());
    }

    #[test]
    fn effective_theme_applies_patch_on_top_of_default() {
        let patch = DocxThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(14),
                ..Default::default()
            }),
            page: Some(PagePatch {
                margin_top_mm: Some(30.0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let theme = effective_theme(&trivial_draft(Some(patch)));
        assert_eq!(theme.typography.body_font_size_pt, 14);
        assert!((theme.page.margin_top_mm - 30.0).abs() < 1e-6);
    }

    #[test]
    fn generate_docx_produces_a_zip_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out.docx");
        let draft = trivial_draft(None);
        let (p, size) = generate_docx(&draft, &out).unwrap();
        assert!(p.exists());
        assert!(size > 0);
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[0..4], b"PK\x03\x04");
    }
}
