//! Finalize a DOCX document.
//!
//! Reads the draft out of the store, runs the consolidated validator,
//! generates the `.docx` file, and emits an `ArtifactCreated` event
//! for the frontend.

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::documents::pdf::artifacts::resolve_artifacts_dir;
use crate::infrastructure::tools::documents::shared::validation::ValidationLevel;
use crate::infrastructure::tools::documents::shared::{DocxDraft, DraftStore, DraftStoreExt};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use super::generator::generate_docx;
use super::validate::{estimate_pages, DocxValidator};

/// Arguments for finalizing a DOCX document.
#[derive(Debug, Deserialize)]
pub struct DocxFinalizeArgs {
    /// Document ID from `docx_start`.
    pub document_id: String,
    /// Whether to automatically save to artifact store.
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
}

const fn default_auto_save() -> bool {
    true
}

/// Validation payload returned to the LLM.
#[derive(Debug, Serialize)]
pub struct ValidationReport {
    /// `true` when every `Error`-level check passed.
    pub passed: bool,
    /// Warning-level messages.
    pub warnings: Vec<String>,
    /// Error-level messages.
    pub errors: Vec<String>,
}

/// Output from finalizing a document.
#[derive(Debug, Serialize)]
pub struct DocxFinalizeOutput {
    /// Document ID.
    pub document_id: String,
    /// Filename.
    pub filename: String,
    /// Artifact ID (if `auto_saved`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Full path to the generated file.
    pub path: String,
    /// Validation report.
    pub validation: ValidationReport,
    /// Number of sections.
    pub section_count: usize,
    /// Estimated page count.
    pub estimated_page_count: usize,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Tool to finalize a DOCX document.
pub struct DocxFinalizeTool;

impl Tool for DocxFinalizeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("docx", "finalize"),
            description: "Finalize a DOCX document, generating the actual Word document file and saving it to the artifact store. After finalizing, no more sections can be added.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document ID from docx_start"
                    },
                    "auto_save": {
                        "type": "boolean",
                        "description": "Whether to automatically save to artifact store (default: true)",
                        "default": true
                    }
                },
                "required": ["document_id"]
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
            let args: DocxFinalizeArgs = parse_args("docx_finalize", arguments)?;

            // Clone the draft rather than removing it — validation or
            // generation can fail (missing image path, write error),
            // and a removed draft means the LLM must rebuild from
            // scratch instead of correcting one section and retrying.
            let draft_store = DraftStore::get_draft_store();
            let draft = {
                let store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;
                store.get_docx_draft(&args.document_id).cloned().ok_or_else(|| {
                    ToolError::failed(format!("Document not found: {}", args.document_id))
                })?
            };

            let filename = draft.filename.clone();
            let section_count = draft.sections.len();
            let estimated_page_count = estimate_pages(&draft);
            let validation = build_validation_report(&draft)?;

            let artifacts_dir = resolve_artifacts_dir(&app_handle)?;
            let output_path = artifacts_dir.join(&filename);
            let (path, size_bytes) = generate_docx(&draft, &output_path)?;

            // Validation and generation both succeeded — now it is
            // safe to drop the draft. We ignore a `None` return here
            // because a concurrent finalize (should not happen with
            // the LLM, but kept robust) may already have removed it.
            {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;
                store.remove_docx_draft(&args.document_id);
            }

            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = DocxFinalizeOutput {
                document_id: args.document_id,
                filename: filename.clone(),
                artifact_id,
                path: path.to_string_lossy().into_owned(),
                validation,
                section_count,
                estimated_page_count,
                size_bytes,
            };

            if let Some(ref artifact_id) = output.artifact_id {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ArtifactCreated {
                        session_id: "current".to_string(),
                        artifact_id: artifact_id.clone(),
                        artifact_type: "Docx".to_string(),
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

/// Run the consolidated validator and fail the tool call when any
/// error-level check trips. Warnings pass through as part of the
/// response payload so the LLM still gets the file.
fn build_validation_report(draft: &DocxDraft) -> Result<ValidationReport, ToolError> {
    let report = DocxValidator::validate_draft(draft);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    for check in &report.checks {
        if check.passed {
            continue;
        }
        match check.level {
            ValidationLevel::Error | ValidationLevel::Critical => {
                errors.push(check.message.clone())
            }
            ValidationLevel::Warning => warnings.push(check.message.clone()),
            ValidationLevel::Info => {}
        }
    }
    if !errors.is_empty() {
        return Err(ToolError::structured(
            format!("docx_finalize: {}", errors.join("; ")),
            serde_json::json!({ "validation": { "errors": errors, "warnings": warnings } }),
        ));
    }
    Ok(ValidationReport {
        passed: errors.is_empty(),
        warnings,
        errors,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{
        DocxContent, DraftSection, DraftStatus, ParagraphText,
    };
    use chrono::Utc;

    fn draft_with(sections: Vec<DraftSection>) -> DocxDraft {
        DocxDraft {
            document_id: "t".into(),
            filename: "t.docx".into(),
            title: "T".into(),
            sections,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_draft_fails_validation() {
        let err = build_validation_report(&draft_with(vec![])).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.to_lowercase().contains("validation") || msg.contains("Has Content"));
    }

    #[test]
    fn valid_draft_passes_validation() {
        let d = draft_with(vec![DraftSection {
            id: "s".into(),
            index: 0,
            content: DocxContent::Heading {
                level: 1,
                text: ParagraphText::Plain("x".into()),
                alignment: None,
                page_break_before: false,
            },
            theme_override: None,
        }]);
        let report = build_validation_report(&d).unwrap();
        assert!(report.passed);
    }

    #[test]
    fn validation_failure_preserves_draft_in_store() {
        // Simulates the finalize flow: we clone the draft, hit a
        // validation error, and must leave the original draft
        // untouched in the store so the LLM can fix and retry.
        let draft_store = DraftStore::get_draft_store();
        let id = {
            let mut store = draft_store.lock().unwrap();
            store.create_docx_draft(
                "preserve.docx".into(),
                "Preserve".into(),
                None,
            )
        };
        // Empty draft — validation will fail.
        let draft = {
            let store = draft_store.lock().unwrap();
            store.get_docx_draft(&id).cloned().unwrap()
        };
        assert!(build_validation_report(&draft).is_err());

        // The draft must still be in the store after the failed
        // validation; a successful finalize would have removed it.
        let still_present = {
            let store = draft_store.lock().unwrap();
            store.get_docx_draft(&id).is_some()
        };
        assert!(still_present, "draft must survive a failed finalize");

        // Clean up to avoid leaking into other tests.
        let _ = draft_store.lock().unwrap().remove_docx_draft(&id);
    }
}
