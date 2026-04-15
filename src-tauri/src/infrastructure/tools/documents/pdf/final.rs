//! Finalize a PDF document.
//!
//! This module is the thin Tool adapter: it pops a draft, hands it to
//! the application service, and emits the artifact-created event. All
//! layout, font loading, and block translation lives under
//! `application/tools/pdf/` and `infrastructure/tools/.../pdf/render/`.

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::documents::pdf::artifacts::{
    migrate_legacy_artifacts, resolve_artifacts_dir,
};
use crate::infrastructure::tools::documents::pdf::write::{
    render_to_artifact, RenderArtifactError,
};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PdfDraft},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

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
        context: ToolExecutionContext,
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

            let (path, size_bytes) = write_pdf(draft, &app_handle).await?;

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
                let session_id = context
                    .session_id
                    .clone()
                    .unwrap_or_else(|| context.conversation_id.clone());
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ArtifactCreated {
                        session_id,
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
/// Resolves the artifacts directory under Tauri's `app_local_data_dir`,
/// migrates any pre-T1.7 files from the legacy `$HOME/.coder/artifacts`
/// location on a best-effort basis, and delegates the render to
/// [`render_to_artifact`] — which runs on `spawn_blocking`, writes to a
/// `.tmp` sibling, and atomically renames on success.
///
/// The [`CancellationToken`] slot is built fresh here because there is
/// no UI channel wired in yet. T3.1 will replace this with a token
/// supplied by the card so the user can stop a long render.
async fn write_pdf(
    draft: PdfDraft,
    app_handle: &AppHandle,
) -> Result<(PathBuf, u64), ToolError> {
    let artifacts_dir = resolve_artifacts_dir(app_handle)?;
    migrate_from_legacy_home(&artifacts_dir);
    let output_path = artifacts_dir.join(&draft.filename);

    let cancel = CancellationToken::new();
    render_to_artifact(draft, output_path, cancel)
        .await
        .map_err(|e| match e {
            RenderArtifactError::Cancelled => ToolError::failed("PDF render cancelled"),
            RenderArtifactError::Render(msg) => {
                ToolError::failed(format!("PDF render failed: {msg}"))
            }
            RenderArtifactError::Io(inner) => {
                ToolError::failed(format!("Failed to write PDF artifact: {inner}"))
            }
            RenderArtifactError::Join(msg) => {
                ToolError::failed(format!("PDF render task crashed: {msg}"))
            }
        })
}

/// Best-effort one-shot migration of any files left under the pre-T1.7
/// `$HOME/.coder/artifacts` location. Silent on any error: this runs on
/// every finalize, and we never want migration trouble to mask a
/// successful render.
fn migrate_from_legacy_home(current: &std::path::Path) {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return;
    };
    let legacy = PathBuf::from(home).join(".coder").join("artifacts");
    if legacy == current {
        return;
    }
    let _ = migrate_legacy_artifacts(&legacy, current);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod sweep_tests {
    /// The artifact path must resolve through Tauri's `app_local_data_dir`.
    /// Anchoring on `HOME` would reintroduce the pre-T1.7 bug of writing
    /// outside the Tauri sandbox.
    #[test]
    fn write_pdf_does_not_anchor_on_home_env() {
        let src = include_str!("final.rs");
        let signature = "async fn write_pdf(";
        let start = src.find(signature).expect("write_pdf must exist");
        let body_start = src[start..].find('{').expect("write_pdf body opens") + start;
        let mut depth = 0_i32;
        let mut end = body_start;
        for (idx, ch) in src[body_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = body_start + idx + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert!(end > body_start, "write_pdf body must be balanced");
        let body = &src[start..end];
        assert!(
            !body.contains(r#"env::var("HOME")"#),
            "write_pdf must not read HOME directly; use resolve_artifacts_dir",
        );
        assert!(
            !body.contains(r#"env::var("USERPROFILE")"#),
            "write_pdf must not read USERPROFILE directly",
        );
        assert!(
            !body.contains(".coder"),
            "write_pdf must not hard-code legacy .coder directory",
        );
    }

    /// T1.8 invariant: `write_pdf` must route through `render_to_artifact`,
    /// which is the only path that gives us the atomic-write + cancellation
    /// guarantees.
    #[test]
    fn write_pdf_routes_through_render_to_artifact() {
        let src = include_str!("final.rs");
        let body = write_pdf_body(src);
        assert!(
            body.contains("render_to_artifact("),
            "write_pdf must call render_to_artifact"
        );
        assert!(
            !body.contains("File::create"),
            "write_pdf must not open the final artifact file directly",
        );
        assert!(
            !body.contains("render_draft("),
            "write_pdf must delegate the render to the atomic write helper",
        );
    }

    fn write_pdf_body(src: &str) -> &str {
        let start = src
            .find("async fn write_pdf(")
            .expect("write_pdf must exist");
        let body_start = src[start..].find('{').expect("body opens") + start;
        let mut depth = 0_i32;
        for (idx, ch) in src[body_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &src[start..=body_start + idx];
                    }
                }
                _ => {}
            }
        }
        panic!("write_pdf body must be balanced");
    }

    /// `ArtifactCreated` events must carry the real `session_id` from
    /// `ToolExecutionContext`. A literal `"current"` would tie every
    /// artifact to the same stream on the frontend.
    #[test]
    fn finalize_does_not_hardcode_session_id() {
        let src = include_str!("final.rs");
        let execute_start = src
            .find("fn execute(")
            .expect("Tool::execute must exist");
        let sweep_start = src
            .find("mod sweep_tests")
            .expect("sweep_tests must exist");
        let exec_region = &src[execute_start..sweep_start];
        assert!(
            !exec_region.contains(r#""current".to_string()"#),
            "execute must not fabricate a 'current' session_id literal",
        );
        let normalized: String = exec_region
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(
            normalized.contains("context.session_id"),
            "execute must thread context.session_id into ArtifactCreated",
        );
    }
}
