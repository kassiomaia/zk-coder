//! Build a typed [`PdfDocument`] from a draft and render it.
//!
//! The `Tool` layer calls into [`render_draft`]; all orchestration lives
//! here so the tool future stays a thin adapter. Every byte of
//! presentation knowledge belongs to the renderer; every byte of agent
//! state belongs to the draft store; this module is the join.

use std::io::Write;

use crate::core::documents::pdf::block::Block;
use crate::core::documents::pdf::document::PdfDocument;
use crate::core::documents::pdf::error::PdfModelError;
use crate::core::documents::pdf::geometry::{Margins, Orientation, PageGeometry, PageSize};
use crate::core::documents::pdf::metadata::Metadata;
use crate::core::documents::pdf::render::{PdfRenderer, RenderError, RenderOutcome, Theme};
use crate::core::documents::pdf::section::{Heading, HeadingLevel, Section};
use crate::infrastructure::tools::documents::shared::{PdfDraft, PdfPageSize};

use super::adapter::page_to_blocks;

/// Failure mode for [`render_draft`].
#[derive(Debug, thiserror::Error)]
pub enum FinalizeError {
    /// Could not build a valid core document from the draft.
    #[error("invalid draft: {0}")]
    Model(#[from] PdfModelError),
    /// The renderer failed to produce a valid PDF.
    #[error("render failed: {0}")]
    Render(#[from] RenderError),
}

/// Builds a [`PdfDocument`] from a draft without touching the renderer.
///
/// Exposed for tests; the production path goes through [`render_draft`].
///
/// # Errors
/// Returns a core [`PdfModelError`] if the draft's title or any
/// generated block violates a core invariant (e.g. an empty title).
pub fn build_document(draft: &PdfDraft, theme: &Theme) -> Result<PdfDocument, PdfModelError> {
    let metadata = Metadata::new(trim_or_fallback(&draft.title, &draft.filename))?;
    let geometry = page_geometry_for(draft.page_size, theme)?;
    let mut doc = PdfDocument::new(metadata).with_geometry(geometry);

    let mut blocks: Vec<Block> = Vec::new();
    for (idx, page) in draft.pages.iter().enumerate() {
        if idx > 0 {
            blocks.push(Block::PageBreak);
        }
        blocks.extend(page_to_blocks(&page.content)?);
    }

    // Wrap the accumulated blocks under one outline heading derived from
    // the title so the T1.2 renderer (which walks sections) has
    // something to emit. Real sectioning lands with T2.1.
    let heading = Heading::new(
        HeadingLevel::new(1)?,
        trim_or_fallback(&draft.title, &draft.filename),
    )?;
    let section = Section::new(heading).with_blocks(blocks);
    doc.push_section(section);
    Ok(doc)
}

/// Renders a draft through the given renderer and returns the result.
///
/// # Errors
/// Returns [`FinalizeError::Model`] on invalid draft metadata and
/// [`FinalizeError::Render`] on engine failures.
pub async fn render_draft<R>(
    draft: &PdfDraft,
    renderer: &R,
    theme: &Theme,
    sink: &mut (dyn Write + Send),
) -> Result<RenderOutcome, FinalizeError>
where
    R: PdfRenderer + ?Sized,
{
    let doc = build_document(draft, theme)?;
    let outcome = renderer.render(&doc, theme, sink, None).await?;
    Ok(outcome)
}

fn page_geometry_for(size: PdfPageSize, theme: &Theme) -> Result<PageGeometry, PdfModelError> {
    let page_size = match size {
        PdfPageSize::A4 => PageSize::A4,
        PdfPageSize::A3 => PageSize::A3,
        PdfPageSize::Letter => PageSize::Letter,
        PdfPageSize::Legal => PageSize::Legal,
    };
    Ok(PageGeometry::new(
        page_size,
        Orientation::Portrait,
        Margins::uniform(theme.page_defaults.margin_mm)?,
    ))
}

fn trim_or_fallback<'a>(primary: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = primary.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::draft_store::{
        DraftPage, DraftStatus, PdfPageContent,
    };
    use chrono::Utc;

    fn draft_with(title: &str, pages: Vec<PdfPageContent>) -> PdfDraft {
        let pages = pages
            .into_iter()
            .enumerate()
            .map(|(i, content)| DraftPage {
                id: format!("page_{i}"),
                index: i,
                content,
            })
            .collect();
        PdfDraft {
            pdf_id: "pdf_test".to_string(),
            filename: "test.pdf".to_string(),
            title: title.to_string(),
            pages,
            page_size: PdfPageSize::A4,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn build_document_carries_title_into_metadata() {
        let d = draft_with(
            "Quarterly Report",
            vec![PdfPageContent::Text {
                content: "Body".to_string(),
            }],
        );
        let doc = build_document(&d, &Theme::default()).unwrap();
        assert_eq!(doc.metadata.title(), "Quarterly Report");
    }

    #[test]
    fn build_document_falls_back_to_filename_when_title_blank() {
        let d = draft_with(
            "   ",
            vec![PdfPageContent::Text {
                content: "Body".to_string(),
            }],
        );
        let doc = build_document(&d, &Theme::default()).unwrap();
        assert_eq!(doc.metadata.title(), "test.pdf");
    }

    #[test]
    fn build_document_inserts_page_break_between_pages() {
        let d = draft_with(
            "T",
            vec![
                PdfPageContent::Text {
                    content: "one".to_string(),
                },
                PdfPageContent::Text {
                    content: "two".to_string(),
                },
            ],
        );
        let doc = build_document(&d, &Theme::default()).unwrap();
        assert_eq!(doc.sections.len(), 1);
        let blocks = &doc.sections[0].blocks;
        let break_count = blocks
            .iter()
            .filter(|b| matches!(b, Block::PageBreak))
            .count();
        assert_eq!(break_count, 1, "exactly one page break between two pages");
    }

    #[test]
    fn build_document_empty_draft_still_has_one_section() {
        let d = draft_with("Empty", vec![]);
        let doc = build_document(&d, &Theme::default()).unwrap();
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.metadata.title(), "Empty");
    }

    #[tokio::test]
    async fn render_draft_produces_pdf_with_matching_title() {
        use crate::infrastructure::tools::documents::pdf::render::{
            bundled_report_cache, PrintPdfRenderer,
        };

        let fonts = bundled_report_cache().expect("bundled fonts load");
        let renderer = PrintPdfRenderer::new(fonts);

        let draft = draft_with(
            "T1.3 Integration",
            vec![
                PdfPageContent::Markdown {
                    content: "# Hello\nbody line".to_string(),
                },
                PdfPageContent::Text {
                    content: "plain text page".to_string(),
                },
            ],
        );

        let mut buf: Vec<u8> = Vec::new();
        let theme = Theme::default();
        let outcome = render_draft(&draft, &renderer, &theme, &mut buf)
            .await
            .expect("render ok");

        assert!(outcome.byte_len > 0);
        assert!(buf.starts_with(b"%PDF-1."));

        let parsed = lopdf::Document::load_mem(&buf).expect("lopdf parses output");
        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(|o| parsed.dereference(o).map(|(_, v)| v))
            .expect("trailer has Info reference");
        let info_dict = info.as_dict().expect("Info is a dict");
        let title_obj = info_dict.get(b"Title").expect("/Title present");
        let title_bytes = match title_obj {
            lopdf::Object::String(s, _) => s.clone(),
            other => panic!("unexpected /Title object: {other:?}"),
        };
        let title_text = decode_pdf_string(&title_bytes);
        assert!(
            title_text.contains("T1.3 Integration"),
            "/Title must equal the draft title, got {title_text:?}"
        );
    }

    fn decode_pdf_string(bytes: &[u8]) -> String {
        if bytes.starts_with(&[0xFE, 0xFF]) {
            let u16s: Vec<u16> = bytes[2..]
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect();
            String::from_utf16_lossy(&u16s)
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}
