//! Finalize a PDF document.
//!
//! This module is the thin Tool adapter: it pops a draft, hands it to
//! the application service, and emits the artifact-created event. All
//! layout, font loading, and block translation lives under
//! `application/tools/pdf/` and `infrastructure/tools/.../pdf/render/`.

use crate::application::tools::pdf::finalize::{render_draft, FinalizeError};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::documents::pdf::render::{load_report_theme, PrintPdfRenderer};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PdfDraft},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::io::BufWriter;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Bundled font used for T1.2/T1.3. T1.5 replaces this single-font load
/// with a cache and fallback chain driven by the theme.
const BUNDLED_FONT_BYTES: &[u8] =
    include_bytes!("../../../../../resources/pdf/fonts/LiberationSans-Regular.ttf");

/// Arguments for finalizing a PDF document.
#[derive(Debug, Deserialize)]
pub struct PdfFinalizeArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Whether to automatically save to artifact store
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

const fn default_auto_save() -> bool {
    true
}

/// Output from finalizing a PDF document.
#[derive(Debug, Serialize)]
pub struct PdfFinalizeOutput {
    /// PDF ID
    pub pdf_id: String,
    /// Filename
    pub filename: String,
    /// Artifact ID (if `auto_saved`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Full path to the generated file
    pub path: String,
    /// Number of pages
    pub page_count: usize,
    /// Page size
    pub page_size: String,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Tool to finalize a PDF document.
pub struct PdfFinalizeTool;

impl Tool for PdfFinalizeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "finalize"),
            description: "Finalize a PDF document, generating the actual PDF file and saving it to the artifact store. After finalizing, no more pages can be added.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID from pdf_start"
                    },
                    "auto_save": {
                        "type": "boolean",
                        "description": "Whether to automatically save to artifact store (default: true)",
                        "default": true
                    }
                },
                "required": ["pdf_id"]
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
            let args: PdfFinalizeArgs = parse_args("pdf_finalize", arguments)?;

            let draft_store = get_draft_store();
            let draft = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                store
                    .remove_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?
            };

            let filename = draft.filename.clone();
            let page_count = draft.pages.len();
            let page_size = draft.page_size;

            let (path, size_bytes) = write_pdf(&draft).await?;

            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = PdfFinalizeOutput {
                pdf_id: args.pdf_id,
                filename,
                artifact_id,
                path: path
                    .to_str()
                    .ok_or_else(|| ToolError::failed("artifact path is not valid UTF-8"))?
                    .to_string(),
                page_count,
                page_size: format!("{page_size:?}"),
                size_bytes,
            };

            if let Some(ref artifact_id) = output.artifact_id {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ArtifactCreated {
                        // T1.7 threads the real `session_id` through
                        // `ToolExecutionContext` and drops this literal.
                        session_id: "current".to_string(),
                        artifact_id: artifact_id.clone(),
                        artifact_type: "Pdf".to_string(),
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

/// Writes the PDF artifact to disk and returns its path and size.
///
/// T1.7 replaces the `HOME`-anchored location with the Tauri
/// `app_local_data_dir`, and T1.8 moves the write onto a background
/// task.
async fn write_pdf(draft: &PdfDraft) -> Result<(PathBuf, u64), ToolError> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| ToolError::failed("Could not determine home directory".to_string()))?;
    let artifacts_dir = PathBuf::from(home).join(".coder").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| ToolError::failed(format!("Failed to create artifacts directory: {e}")))?;
    let output_path = artifacts_dir.join(&draft.filename);

    let renderer = PrintPdfRenderer::new(BUNDLED_FONT_BYTES.to_vec())
        .map_err(|e| ToolError::failed(format!("Failed to initialise PDF renderer: {e}")))?;
    let file = std::fs::File::create(&output_path)
        .map_err(|e| ToolError::failed(format!("Failed to create PDF file: {e}")))?;
    let mut writer = BufWriter::new(file);
    let theme = load_report_theme();

    render_draft(draft, &renderer, &theme, &mut writer)
        .await
        .map_err(|e| match e {
            FinalizeError::Model(inner) => {
                ToolError::failed(format!("Invalid PDF draft: {inner}"))
            }
            FinalizeError::Render(inner) => {
                ToolError::failed(format!("PDF render failed: {inner}"))
            }
        })?;

    let size_bytes = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
    Ok((output_path, size_bytes))
}
