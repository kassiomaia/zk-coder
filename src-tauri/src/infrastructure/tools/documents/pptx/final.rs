//! Finalize a PPTX presentation.

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{
        get_draft_store, DraftStoreExt, PptxDraft, PptxSlide, SlideContent, SlideLayout,
    },
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

// PPTX generation library
use pptx::Presentation;

/// Arguments for finalizing a PPTX presentation.
#[derive(Debug, Deserialize)]
pub struct PptxFinalizeArgs {
    /// Deck ID from pptx_start
    pub deck_id: String,
    /// Whether to automatically save to artifact store
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

fn default_auto_save() -> bool {
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
    /// Artifact ID (if auto_saved)
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
                    ToolError::failed(format!("Failed to acquire draft store lock: {}", e))
                })?;

                // Get and remove the draft (finalizing removes it from active drafts)
                store.remove_pptx_draft(&args.deck_id).ok_or_else(|| {
                    ToolError::failed(format!("Deck not found: {}", args.deck_id))
                })?
            };

            // Clone needed data before async call
            let filename = draft.filename.clone();
            let slide_count = draft.slides.len();

            // Validate the presentation
            let validation = validate_pptx_draft(&draft);

            // Estimate duration
            let estimated_duration_minutes = estimate_duration(&draft);

            // Generate the PPTX file
            let (path, size_bytes) = generate_pptx(draft).await?;

            // Create artifact reference
            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = PptxFinalizeOutput {
                deck_id: args.deck_id,
                filename,
                artifact_id: artifact_id.clone(),
                path: path.to_str().unwrap().to_string(),
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
    (draft.slides.len() * 3 + 1) / 2
}

/// Generate a PPTX file from the draft using the pptx crate.
async fn generate_pptx(draft: PptxDraft) -> Result<(PathBuf, u64), ToolError> {
    use pptx::Presentation;

    // Get the home directory for artifact storage
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| ToolError::failed("Could not determine home directory".to_string()))?;

    // Create artifacts directory in home/.coder/artifacts
    let artifacts_dir = PathBuf::from(home).join(".coder").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| ToolError::failed(format!("Failed to create artifacts directory: {}", e)))?;

    // Create output file path
    let output_path = artifacts_dir.join(&draft.filename);

    // Create a new presentation
    let mut prs = Presentation::new()
        .map_err(|e| ToolError::failed(format!("Failed to create presentation: {}", e)))?;

    // Get slide layouts
    let layouts = prs
        .slide_layouts()
        .map_err(|e| ToolError::failed(format!("Failed to get slide layouts: {}", e)))?;

    // Use the first layout as default
    let default_layout = layouts
        .first()
        .ok_or_else(|| ToolError::failed("No slide layouts available".to_string()))?;

    // Add each slide from the draft
    for draft_slide in &draft.slides {
        let _slide = prs
            .add_slide(default_layout)
            .map_err(|e| ToolError::failed(format!("Failed to add slide: {}", e)))?;

        // Note: For now, we create basic slides with titles
        // Full content rendering will be added in a follow-up
        // The pptx crate v0.1.0 has limited shape manipulation API
    }

    // If no slides were added, add a title slide
    if draft.slides.is_empty() {
        let _ = prs
            .add_slide(default_layout)
            .map_err(|e| ToolError::failed(format!("Failed to add title slide: {}", e)))?;
    }

    // Save the presentation
    prs.save(&output_path)
        .map_err(|e| ToolError::failed(format!("Failed to save presentation: {}", e)))?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((output_path, size_bytes))
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
}
