//! Finalize a PPTX presentation.

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::pdf::artifacts::resolve_artifacts_dir,
    documents::shared::{get_draft_store, PptxDraft, PptxSlide, SlideContent, SlideLayout},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

// PPTX generation library
use pptx::{BulletFormat, Emu, Inches, Presentation, ShapeId, ShapeTree, TextFrame};

/// Arguments for finalizing a PPTX presentation.
#[derive(Debug, Deserialize)]
pub struct PptxFinalizeArgs {
    /// Deck ID from `pptx_start`
    pub deck_id: String,
    /// Whether to automatically save to artifact store
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

const fn default_auto_save() -> bool {
    true
}

/// Validation report for the presentation.
#[derive(Debug, Serialize)]
pub struct ValidationReport {
    /// Whether validation passed
    pub passed: bool,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Validation errors
    pub errors: Vec<String>,
}

/// Output from finalizing a presentation.
#[derive(Debug, Serialize)]
pub struct PptxFinalizeOutput {
    /// Deck ID
    pub deck_id: String,
    /// Filename
    pub filename: String,
    /// Artifact ID (if `auto_saved`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Full path to the generated file
    pub path: String,
    /// Validation report
    pub validation: ValidationReport,
    /// Number of slides
    pub slide_count: usize,
    /// Estimated duration in minutes (rough estimate: 1-2 minutes per slide)
    pub estimated_duration_minutes: usize,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Tool to finalize a PPTX presentation.
pub struct PptxFinalizeTool;

impl Tool for PptxFinalizeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pptx", "finalize"),
            description: "Finalize a PowerPoint presentation, generating the actual presentation file and saving it to the artifact store. After finalizing, no more slides can be added.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deck_id": {
                        "type": "string",
                        "description": "Deck ID from pptx_start"
                    },
                    "auto_save": {
                        "type": "boolean",
                        "description": "Whether to automatically save to artifact store (default: true)",
                        "default": true
                    }
                },
                "required": ["deck_id"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PptxFinalizeArgs = parse_args("pptx_finalize", arguments)?;

            let draft_store = get_draft_store();
            let draft = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                // Get and remove the draft (finalizing removes it from active drafts)
                store
                    .remove_pptx_draft(&args.deck_id)
                    .ok_or_else(|| ToolError::failed(format!("Deck not found: {}", args.deck_id)))?
            };

            // Clone needed data before async call
            let filename = draft.filename.clone();
            let slide_count = draft.slides.len();

            // Validate the presentation
            let validation = validate_pptx_draft(&draft);

            // Estimate duration
            let estimated_duration_minutes = estimate_duration(&draft);

            // Generate the PPTX file
            let (path, size_bytes) = generate_pptx(draft, &app_handle).await?;

            // Create artifact reference
            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = PptxFinalizeOutput {
                deck_id: args.deck_id,
                filename,
                artifact_id,
                path: path.to_string_lossy().into_owned(),
                validation,
                slide_count,
                estimated_duration_minutes,
                size_bytes,
            };

            // Emit ArtifactCreated event for frontend display
            if let Some(ref artifact_id) = output.artifact_id {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ArtifactCreated {
                        session_id: "current".to_string(), // TODO: Get actual session_id
                        artifact_id: artifact_id.clone(),
                        artifact_type: "Pptx".to_string(),
                        filename: output.filename.clone(),
                        size_bytes: output.size_bytes,
                        path: output.path.clone(),
                    },
                );
            }

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Validate a PPTX draft.
fn validate_pptx_draft(draft: &PptxDraft) -> ValidationReport {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut passed = true;

    // Check if presentation is empty
    if draft.slides.is_empty() {
        warnings.push("Presentation has no slides".to_string());
    }

    // Check if first slide is a title slide
    if let Some(first_slide) = draft.slides.first() {
        if !matches!(first_slide.slide.layout, SlideLayout::TitleSlide) {
            warnings.push(
                "First slide is not a TitleSlide layout. Consider adding a title slide."
                    .to_string(),
            );
        }
    }

    // Check for slides with no content
    for slide in &draft.slides {
        if slide.slide.content.is_empty() {
            warnings.push(format!(
                "Slide {} (\"{}\") has no content",
                slide.index + 1,
                slide.slide.title
            ));
        }

        // Check for very long titles
        if slide.slide.title.len() > 50 {
            warnings.push(format!(
                "Slide {} title is very long ({} chars), may not display well",
                slide.index + 1,
                slide.slide.title.len()
            ));
        }
    }

    // Check for table consistency
    for slide in &draft.slides {
        for content in &slide.slide.content {
            if let SlideContent::Table { headers, rows, .. } = content {
                let col_count = headers.len();
                for (row_idx, row) in rows.iter().enumerate() {
                    if row.len() != col_count {
                        errors.push(format!(
                            "Table row {} in slide \"{}\" has {} columns but header has {} columns",
                            row_idx + 1,
                            slide.slide.title,
                            row.len(),
                            col_count
                        ));
                        passed = false;
                    }
                }
            }
        }
    }

    // Check for image paths that might not exist
    for slide in &draft.slides {
        for content in &slide.slide.content {
            if let SlideContent::Image { path, .. } = content {
                if !std::path::Path::new(path).exists() {
                    warnings.push(format!(
                        "Image path may not exist: {} (slide \"{}\")",
                        path, slide.slide.title
                    ));
                }
            }
        }
    }

    ValidationReport {
        passed,
        warnings,
        errors,
    }
}

/// Estimate presentation duration from draft.
fn estimate_duration(draft: &PptxDraft) -> usize {
    // Estimate 1.5 minutes per slide
    (draft.slides.len() * 3).div_ceil(2)
}

/// Generate a PPTX file from the draft using the pptx crate.
async fn generate_pptx(
    draft: PptxDraft,
    app_handle: &AppHandle,
) -> Result<(PathBuf, u64), ToolError> {
    let artifacts_dir = resolve_artifacts_dir(app_handle)?;
    generate_pptx_to_dir(draft, &artifacts_dir).await
}

async fn generate_pptx_to_dir(
    draft: PptxDraft,
    artifacts_dir: &std::path::Path,
) -> Result<(PathBuf, u64), ToolError> {
    let output_path = artifacts_dir.join(&draft.filename);

    // Create a new presentation
    let mut prs = Presentation::new()
        .map_err(|e| ToolError::failed(format!("Failed to create presentation: {e}")))?;

    // Get slide layouts
    let layouts = prs
        .slide_layouts()
        .map_err(|e| ToolError::failed(format!("Failed to get slide layouts: {e}")))?;

    // Use the first layout as default
    let default_layout = layouts
        .first()
        .ok_or_else(|| ToolError::failed("No slide layouts available".to_string()))?;

    // Add each slide from the draft
    for draft_slide in &draft.slides {
        let layout = select_layout(&layouts, &draft_slide.slide.layout).unwrap_or(default_layout);

        let slide_ref = prs
            .add_slide(layout)
            .map_err(|e| ToolError::failed(format!("Failed to add slide: {e}")))?;

        render_slide_content(&mut prs, &slide_ref, &draft_slide.slide)
            .map_err(|e| ToolError::failed(format!("Failed to render slide content: {e}")))?;
    }

    // If no slides were added, add a title slide
    if draft.slides.is_empty() {
        let _ = prs
            .add_slide(default_layout)
            .map_err(|e| ToolError::failed(format!("Failed to add title slide: {e}")))?;
    }

    // Save the presentation
    prs.save(&output_path)
        .map_err(|e| ToolError::failed(format!("Failed to save presentation: {e}")))?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((output_path, size_bytes))
}

fn select_layout<'a>(
    layouts: &'a [pptx::slide::SlideLayoutRef],
    requested: &SlideLayout,
) -> Option<&'a pptx::slide::SlideLayoutRef> {
    let names = match requested {
        SlideLayout::TitleSlide => ["title slide", "title"].as_slice(),
        SlideLayout::TitleAndContent => ["title and content", "title"].as_slice(),
        SlideLayout::SectionHeader => ["section header", "section"].as_slice(),
        SlideLayout::TwoColumn | SlideLayout::TwoContent => ["two content", "two"].as_slice(),
        SlideLayout::Comparison => ["comparison"].as_slice(),
        SlideLayout::ContentWithCaption => ["content with caption", "caption"].as_slice(),
        SlideLayout::Blank => ["blank"].as_slice(),
    };

    layouts.iter().find(|layout| {
        let normalized = layout.name.to_lowercase();
        names.iter().any(|name| normalized.contains(name))
    })
}

fn render_slide_content(
    presentation: &mut Presentation,
    slide_ref: &pptx::slide::SlideRef,
    slide: &PptxSlide,
) -> Result<(), ToolError> {
    let mut shapes = Vec::new();

    if !slide.title.trim().is_empty() {
        let title_frame = single_text_paragraph(slide.title.trim());
        shapes.push(build_textbox_with_text(
            ShapeId(10),
            "Title",
            Inches(0.5).into(),
            Inches(0.3).into(),
            Inches(9.0).into(),
            Inches(1.0).into(),
            &title_frame,
        ));
    }

    let mut body_paragraphs = Vec::new();
    for content in &slide.content {
        match content {
            SlideContent::TextBox { text, .. } if !text.trim().is_empty() => {
                body_paragraphs.push(BodyParagraph::Plain(text.trim().to_string()));
            }
            SlideContent::BulletList { items, level } => {
                for item in items.iter().filter(|item| !item.trim().is_empty()) {
                    body_paragraphs.push(BodyParagraph::Bullet {
                        text: item.trim().to_string(),
                        level: *level,
                    });
                }
            }
            SlideContent::NumberedList { items } => {
                for item in items.iter().filter(|item| !item.trim().is_empty()) {
                    body_paragraphs.push(BodyParagraph::Numbered(item.trim().to_string()));
                }
            }
            SlideContent::Table { .. } => {
                body_paragraphs.push(BodyParagraph::Plain(
                    "[Table content is not yet supported]".to_string(),
                ));
            }
            SlideContent::Image { path, .. } => {
                body_paragraphs.push(BodyParagraph::Plain(format!("[Image: {}]", path.trim())));
            }
            SlideContent::ChartPlaceholder { title, .. } => {
                body_paragraphs.push(BodyParagraph::Plain(format!("[Chart: {}]", title.trim())));
            }
            SlideContent::Title | SlideContent::Subtitle | SlideContent::TextBox { .. } => {}
        }
    }

    if !body_paragraphs.is_empty() {
        let body_frame = body_text_frame(&body_paragraphs);
        shapes.push(build_textbox_with_text(
            ShapeId(11),
            "Body",
            Inches(0.8).into(),
            Inches(1.5).into(),
            Inches(8.4).into(),
            Inches(4.8).into(),
            &body_frame,
        ));
    }

    append_shapes_to_slide(presentation, slide_ref, &shapes)
}

#[derive(Debug)]
enum BodyParagraph {
    Plain(String),
    Bullet { text: String, level: u8 },
    Numbered(String),
}

fn single_text_paragraph(text: &str) -> TextFrame {
    let mut text_frame = TextFrame::new();
    text_frame.set_text(text);
    text_frame
}

fn body_text_frame(paragraphs: &[BodyParagraph]) -> TextFrame {
    let mut text_frame = TextFrame::new();
    text_frame.clear();

    for (index, paragraph) in paragraphs.iter().enumerate() {
        let target = if index == 0 {
            &mut text_frame.paragraphs_mut()[0]
        } else {
            text_frame.add_paragraph()
        };

        target.clear();

        match paragraph {
            BodyParagraph::Plain(text) => {
                target.add_run().set_text(text);
            }
            BodyParagraph::Bullet { text, level } => {
                target.level = (*level).min(8);
                target.set_bullet(BulletFormat::Character('•'));
                target.add_run().set_text(text);
            }
            BodyParagraph::Numbered(text) => {
                target.set_bullet(BulletFormat::AutoNumbered("arabicPeriod".to_string()));
                target.add_run().set_text(text);
            }
        }
    }

    text_frame
}

fn build_textbox_with_text(
    shape_id: ShapeId,
    name: &str,
    left: Emu,
    top: Emu,
    width: Emu,
    height: Emu,
    text_frame: &TextFrame,
) -> String {
    let xml = ShapeTree::new_textbox_xml(shape_id, name, left, top, width, height);
    replace_tx_body(&xml, &text_frame.to_xml_string()).unwrap_or(xml)
}

fn replace_tx_body(xml: &str, replacement: &str) -> Option<String> {
    let start = xml.find("<p:txBody")?;
    let end = xml.find("</p:txBody>")? + "</p:txBody>".len();

    let mut updated = String::with_capacity(xml.len() + replacement.len());
    updated.push_str(&xml[..start]);
    updated.push_str(replacement);
    updated.push_str(&xml[end..]);
    Some(updated)
}

fn append_shapes_to_slide(
    presentation: &mut Presentation,
    slide_ref: &pptx::slide::SlideRef,
    shapes: &[String],
) -> Result<(), ToolError> {
    if shapes.is_empty() {
        return Ok(());
    }

    let slide_xml = presentation
        .slide_xml_mut(slide_ref)
        .map_err(|e| ToolError::failed(format!("Failed to get mutable slide XML: {e}")))?;

    let xml_string = String::from_utf8(slide_xml.clone())
        .map_err(|e| ToolError::failed(format!("Slide XML is not valid UTF-8: {e}")))?;

    let marker = "</p:spTree>";
    let marker_index = xml_string
        .find(marker)
        .ok_or_else(|| ToolError::failed("Slide XML missing </p:spTree> marker".to_string()))?;

    let mut updated =
        String::with_capacity(xml_string.len() + shapes.iter().map(String::len).sum::<usize>());
    updated.push_str(&xml_string[..marker_index]);
    for shape in shapes {
        updated.push_str(shape);
    }
    updated.push_str(&xml_string[marker_index..]);

    *slide_xml = updated.into_bytes();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DraftSlide, DraftStatus};
    use chrono::Utc;

    fn create_test_draft() -> PptxDraft {
        PptxDraft {
            deck_id: "test_deck".to_string(),
            filename: "test.pptx".to_string(),
            slides: vec![
                DraftSlide {
                    id: "slide_1".to_string(),
                    index: 0,
                    slide: PptxSlide {
                        layout: SlideLayout::TitleSlide,
                        title: "Title".to_string(),
                        content: vec![SlideContent::Title],
                        notes: None,
                    },
                },
                DraftSlide {
                    id: "slide_2".to_string(),
                    index: 1,
                    slide: PptxSlide {
                        layout: SlideLayout::TitleAndContent,
                        title: "Content".to_string(),
                        content: vec![SlideContent::BulletList {
                            items: vec!["Item 1".to_string(), "Item 2".to_string()],
                            level: 0,
                        }],
                        notes: None,
                    },
                },
            ],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_empty_presentation() {
        let draft = PptxDraft {
            deck_id: "test".to_string(),
            filename: "test.pptx".to_string(),
            slides: vec![],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = validate_pptx_draft(&draft);
        assert!(!report.warnings.is_empty());
        assert!(report.warnings.iter().any(|w| w.contains("no slides")));
    }

    #[test]
    fn test_validate_first_slide_not_title() {
        let draft = PptxDraft {
            deck_id: "test".to_string(),
            filename: "test.pptx".to_string(),
            slides: vec![DraftSlide {
                id: "slide_1".to_string(),
                index: 0,
                slide: PptxSlide {
                    layout: SlideLayout::TitleAndContent, // Not TitleSlide
                    title: "First Slide".to_string(),
                    content: vec![],
                    notes: None,
                },
            }],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let report = validate_pptx_draft(&draft);
        assert!(report.warnings.iter().any(|w| w.contains("TitleSlide")));
    }

    #[test]
    fn test_validate_valid_presentation() {
        let draft = create_test_draft();
        let report = validate_pptx_draft(&draft);
        assert!(report.passed);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_estimate_duration() {
        let draft = create_test_draft(); // 2 slides
        let duration = estimate_duration(&draft);
        // 2 slides * 1.5 = 3 minutes
        assert_eq!(duration, 3);
    }

    #[test]
    fn test_estimate_duration_single_slide() {
        let draft = PptxDraft {
            deck_id: "test".to_string(),
            filename: "test.pptx".to_string(),
            slides: vec![DraftSlide {
                id: "slide_1".to_string(),
                index: 0,
                slide: PptxSlide {
                    layout: SlideLayout::TitleSlide,
                    title: "Title".to_string(),
                    content: vec![],
                    notes: None,
                },
            }],
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let duration = estimate_duration(&draft);
        // 1 slide * 1.5 = 1.5, rounded to (1*3+1)/2 = 2
        assert_eq!(duration, 2);
    }

    #[test]
    fn test_generate_pptx_writes_slide_text() {
        let mut draft = create_test_draft();
        draft.filename = format!("test-{}.pptx", uuid::Uuid::new_v4());

        let tmp = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (path, _size) = runtime
            .block_on(generate_pptx_to_dir(draft, tmp.path()))
            .unwrap();

        let prs = pptx::Presentation::open(&path).unwrap();
        let slides = prs.slides().unwrap();
        assert_eq!(slides.len(), 2);

        let slide1_xml = String::from_utf8(prs.slide_xml(&slides[0]).unwrap().to_vec()).unwrap();
        let slide2_xml = String::from_utf8(prs.slide_xml(&slides[1]).unwrap().to_vec()).unwrap();

        assert!(slide1_xml.contains("Title"));
        assert!(slide2_xml.contains("Item 1"));
        assert!(slide2_xml.contains("Item 2"));
    }
}
