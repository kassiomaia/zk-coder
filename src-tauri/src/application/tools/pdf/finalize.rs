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
use crate::core::documents::pdf::render::{
    PdfRenderer, ProgressSink, RenderError, RenderOutcome, Theme,
};
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
    let mut prev_had_override = false;
    for (idx, page) in draft.pages.iter().enumerate() {
        // Emit the theme switch BEFORE the page break so the new page's
        // cursor is initialised with the incoming params' leading. If the
        // switch fired after the break, `break_page()` would compute
        // `initial_y_mm` from the outgoing theme and then the switch
        // would silently shift leading on an already-placed cursor,
        // starting the overridden page at the wrong y-position.
        match page.theme_override.as_ref() {
            Some(patch) => {
                blocks.push(Block::ThemeSwitch(patch.clone()));
                prev_had_override = true;
            }
            None if prev_had_override => {
                // Restore the document's base theme for this page.
                blocks.push(Block::ThemeSwitch(
                    crate::core::documents::pdf::theme_patch::ThemePatch::default(),
                ));
                prev_had_override = false;
            }
            None => {}
        }
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
    progress: Option<ProgressSink>,
) -> Result<RenderOutcome, FinalizeError>
where
    R: PdfRenderer + ?Sized,
{
    let doc = build_document(draft, theme)?;
    let outcome = renderer.render(&doc, theme, sink, progress).await?;
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
                theme_override: None,
            })
            .collect();
        PdfDraft {
            pdf_id: "pdf_test".to_string(),
            filename: "test.pdf".to_string(),
            title: title.to_string(),
            pages,
            page_size: PdfPageSize::A4,
            theme_patch: None,
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
    fn build_document_inserts_theme_switch_only_for_overridden_page() {
        use crate::core::documents::pdf::theme_patch::{
            PalettePatch, RgbPatch, ThemePatch,
        };

        let pages = vec![
            DraftPage {
                id: "p0".to_string(),
                index: 0,
                content: PdfPageContent::Text {
                    content: "first".to_string(),
                },
                theme_override: None,
            },
            DraftPage {
                id: "p1".to_string(),
                index: 1,
                content: PdfPageContent::Text {
                    content: "second".to_string(),
                },
                theme_override: Some(ThemePatch {
                    palette: Some(PalettePatch {
                        heading: Some(RgbPatch {
                            r: Some(0.8),
                            g: Some(0.1),
                            b: Some(0.1),
                        }),
                        ..PalettePatch::default()
                    }),
                    ..ThemePatch::default()
                }),
            },
            DraftPage {
                id: "p2".to_string(),
                index: 2,
                content: PdfPageContent::Text {
                    content: "third".to_string(),
                },
                theme_override: None,
            },
        ];
        let draft = PdfDraft {
            pdf_id: "pdf_t".to_string(),
            filename: "t.pdf".to_string(),
            title: "T".to_string(),
            pages,
            page_size: PdfPageSize::A4,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let doc = build_document(&draft, &Theme::default()).unwrap();
        let blocks = &doc.sections[0].blocks;
        let switches: Vec<&Block> = blocks
            .iter()
            .filter(|b| matches!(b, Block::ThemeSwitch(_)))
            .collect();
        assert_eq!(
            switches.len(),
            2,
            "one switch to apply the override, one to reset"
        );
        // First switch carries the override's palette patch.
        match switches[0] {
            Block::ThemeSwitch(patch) => {
                assert!(patch.palette.is_some(), "first switch has palette patch");
            }
            _ => panic!("expected ThemeSwitch"),
        }
        // Second switch resets to an empty patch.
        match switches[1] {
            Block::ThemeSwitch(patch) => {
                assert!(
                    patch.palette.is_none(),
                    "reset switch has no palette override"
                );
                assert!(patch.typography.is_none());
            }
            _ => panic!("expected ThemeSwitch"),
        }
    }

    #[test]
    fn build_document_emits_no_theme_switch_when_no_overrides() {
        let pages = vec![
            DraftPage {
                id: "p0".to_string(),
                index: 0,
                content: PdfPageContent::Text {
                    content: "a".to_string(),
                },
                theme_override: None,
            },
            DraftPage {
                id: "p1".to_string(),
                index: 1,
                content: PdfPageContent::Text {
                    content: "b".to_string(),
                },
                theme_override: None,
            },
        ];
        let draft = PdfDraft {
            pdf_id: "pdf_n".to_string(),
            filename: "n.pdf".to_string(),
            title: "N".to_string(),
            pages,
            page_size: PdfPageSize::A4,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let doc = build_document(&draft, &Theme::default()).unwrap();
        let switches = doc.sections[0]
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::ThemeSwitch(_)))
            .count();
        assert_eq!(
            switches, 0,
            "no overrides → no ThemeSwitch blocks (backwards-compatible path)"
        );
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
        let outcome = render_draft(&draft, &renderer, &theme, &mut buf, None)
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

    /// M1 exit-gate fixture: a 4-page reference document that exercises
    /// headings, paragraphs, inline styles, and explicit page breaks.
    /// Used by the structural fingerprint test below.
    fn m1_reference_draft() -> PdfDraft {
        let pages = vec![
            PdfPageContent::Markdown {
                content: "# M1 Reference\n\nCover page for the M1 exit gate. \
                          This fixture exists to lock the structural shape \
                          of a finalized PDF: page count, embedded fonts, \
                          and the title round-trip.".to_string(),
            },
            PdfPageContent::Markdown {
                content: "## Body Page\n\nA second page with a body \
                          paragraph that uses **bold**, *italic*, and \
                          regular text in a single run. Layout must \
                          paginate cleanly and the bold/italic font \
                          slots must end up in the embedded fonts list."
                    .to_string(),
            },
            PdfPageContent::Markdown {
                content: "## Continuation\n\nA third page demonstrates \
                          that explicit page breaks (one per draft page) \
                          chain through the layout engine. Adding more \
                          words here verifies that the wrap logic still \
                          fires inside a single page.".to_string(),
            },
            PdfPageContent::Text {
                content: "Closing page (plain text). The renderer must \
                          treat plain text as a single paragraph block \
                          and end the document on page four exactly."
                    .to_string(),
            },
        ];
        draft_with("M1 Reference", pages)
    }

    /// Locks the structural fingerprint of the M1 reference render.
    ///
    /// Replaces the originally-planned SHA-256 byte comparison: `printpdf 0.7`
    /// stamps a random `document_id` / `instance_id` and the current
    /// timestamp into every output, so byte hashes flap. The structural
    /// assertions catch every regression a byte hash would (page count,
    /// font embedding, title round-trip, plausible byte budget) without
    /// the noise.
    #[tokio::test]
    async fn m1_exit_gate_four_page_reference_renders() {
        use crate::infrastructure::tools::documents::pdf::render::{
            bundled_report_cache, PrintPdfRenderer,
        };

        let fonts = bundled_report_cache().expect("bundled fonts load");
        let renderer = PrintPdfRenderer::new(fonts);
        let theme = Theme::default();

        let draft = m1_reference_draft();
        let mut buf = Vec::new();
        let outcome = render_draft(&draft, &renderer, &theme, &mut buf, None)
            .await
            .expect("M1 reference renders");

        assert!(buf.starts_with(b"%PDF-1."), "valid PDF magic header");
        let pdf = lopdf::Document::load_mem(&buf).expect("lopdf parses output");

        let lopdf_pages = pdf.page_iter().count();
        assert_eq!(
            lopdf_pages, 4,
            "M1 reference must produce exactly 4 pages (lopdf saw {lopdf_pages})"
        );
        assert_eq!(
            outcome.page_count, 4,
            "renderer page_count must agree with lopdf's count"
        );

        assert!(
            outcome.embedded_fonts.len() >= 4,
            "M1 reference must embed at least the four bundled font slots; got {} ({:?})",
            outcome.embedded_fonts.len(),
            outcome
                .embedded_fonts
                .iter()
                .map(|f| &f.family)
                .collect::<Vec<_>>()
        );

        let len = outcome.byte_len;
        // Empirical range: with 4 TTF fonts embedded the bulk is fonts,
        // not text — 4 pages of body land around ~2.5–3 MB on the
        // bundled cache. Floor catches "rendered nothing", ceiling
        // catches "embedded fonts twice" or "blew up content stream".
        assert!(
            (500_000..6_000_000).contains(&len),
            "M1 reference byte_len {len} outside plausible 4-page range"
        );

        let info = pdf
            .trailer
            .get(b"Info")
            .and_then(|o| pdf.dereference(o).map(|(_, v)| v))
            .expect("trailer carries /Info");
        let info_dict = info.as_dict().expect("/Info is a dict");
        let title_obj = info_dict.get(b"Title").expect("/Title present");
        let title_bytes = match title_obj {
            lopdf::Object::String(s, _) => s.clone(),
            other => panic!("unexpected /Title object: {other:?}"),
        };
        let title = decode_pdf_string(&title_bytes);
        assert!(
            title.contains("M1 Reference"),
            "/Title must round-trip the draft title; got {title:?}"
        );

        assert!(
            outcome.warnings.is_empty(),
            "M1 reference must render without structural warnings: {:?}",
            outcome.warnings
        );
    }

    /// Integration test: rich Markdown content (as the agent would emit it)
    /// passes through the adapter → build_document → renderer pipeline
    /// and produces a valid multi-page PDF with the expected structure.
    #[tokio::test]
    async fn rich_markdown_through_full_pipeline() {
        use crate::infrastructure::tools::documents::pdf::render::{
            bundled_report_cache, PrintPdfRenderer,
        };

        let fonts = bundled_report_cache().expect("bundled fonts load");
        let renderer = PrintPdfRenderer::new(fonts);
        let theme = Theme::default();

        let rich_md = r#"# Quarterly Report

This is the **executive summary** with a [link](https://example.com) and `inline code`.

## Key Metrics

| Metric | Q1 | Q2 |
|--------|----|----|
| Revenue | $10M | $12M |
| Users | 50k | 65k |

## Highlights

- Launched new **feature A**
- Improved *performance* by 40%
  - Backend optimizations
  - Frontend caching
1. First priority item
2. Second priority item

```rust
fn main() {
    println!("Hello, world!");
}
```

> **Important**: This quarter exceeded all targets.

---

## Conclusion

The quarter was *very* successful."#;

        let draft = draft_with(
            "Rich Markdown Test",
            vec![PdfPageContent::Markdown {
                content: rich_md.to_string(),
            }],
        );

        let mut buf: Vec<u8> = Vec::new();
        let outcome = render_draft(&draft, &renderer, &theme, &mut buf, None)
            .await
            .expect("rich markdown renders");

        assert!(buf.starts_with(b"%PDF-1."), "valid PDF header");
        assert!(outcome.page_count >= 1, "at least 1 page");

        let pdf = lopdf::Document::load_mem(&buf).expect("lopdf parses output");

        // The PDF must contain the URI annotation for the link.
        let raw = String::from_utf8_lossy(&buf);
        assert!(
            raw.contains("example.com"),
            "rendered PDF must contain the external link URI"
        );

        // The outline should contain the headings.
        assert!(
            !outcome.outline.is_empty(),
            "rendered PDF should have outline entries from headings"
        );
        let titles: Vec<&str> = outcome.outline.iter().map(|e| e.title.as_str()).collect();
        assert!(
            titles.iter().any(|t| t.contains("Quarterly")),
            "outline should contain 'Quarterly Report'"
        );
        assert!(
            titles.iter().any(|t| t.contains("Conclusion")),
            "outline should contain 'Conclusion'"
        );

        // Verify font dict has embedded fonts (not just Helvetica).
        let mut font_count = 0;
        for (_, obj) in &pdf.objects {
            if let lopdf::Object::Dictionary(d) = obj {
                if let Ok(t) = d.get(b"Type") {
                    if matches!(t, lopdf::Object::Name(name) if name == b"Font") {
                        font_count += 1;
                    }
                }
            }
        }
        assert!(
            font_count >= 3,
            "rendered PDF must embed multiple fonts, got {font_count}"
        );
    }
}
