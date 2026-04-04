//! Finalize a PDF document.

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PdfDraft, PdfPageContent, PdfPageSize},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use printpdf::{
    BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

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

                // Get and remove the draft (finalizing removes it from active drafts)
                store
                    .remove_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?
            };

            // Clone needed data before async call
            let filename = draft.filename.clone();
            let page_count = draft.pages.len();
            let page_size = draft.page_size;

            // Generate the PDF file
            let (path, size_bytes) = generate_pdf(draft).await?;

            // Create artifact reference
            let artifact_id = if args.auto_save {
                Some(format!("art_{}", uuid::Uuid::new_v4()))
            } else {
                None
            };

            let output = PdfFinalizeOutput {
                pdf_id: args.pdf_id,
                filename,
                artifact_id,
                path: path.to_str().unwrap().to_string(),
                page_count,
                page_size: format!("{page_size:?}"),
                size_bytes,
            };

            // Emit ArtifactCreated event for frontend display
            if let Some(ref artifact_id) = output.artifact_id {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ArtifactCreated {
                        session_id: "current".to_string(), // TODO: Get actual session_id
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

/// Generate a PDF file from the draft using printpdf.
async fn generate_pdf(draft: PdfDraft) -> Result<(PathBuf, u64), ToolError> {
    // Get the home directory for artifact storage
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| ToolError::failed("Could not determine home directory".to_string()))?;

    // Create artifacts directory in home/.coder/artifacts
    let artifacts_dir = PathBuf::from(home).join(".coder").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| ToolError::failed(format!("Failed to create artifacts directory: {e}")))?;

    // Create output file path
    let output_path = artifacts_dir.join(&draft.filename);

    // Get page dimensions
    let (page_width, page_height) = get_page_dimensions(&draft.page_size);

    // Handle empty draft - create a document with one blank page
    if draft.pages.is_empty() {
        let (doc, _page1, _layer1) =
            PdfDocument::new(&draft.filename, page_width, page_height, "Layer 1");
        let file = File::create(&output_path)
            .map_err(|e| ToolError::failed(format!("Failed to create PDF file: {e}")))?;
        let mut writer = BufWriter::new(file);
        doc.save(&mut writer)
            .map_err(|e| ToolError::failed(format!("Failed to save PDF: {e}")))?;

        let size_bytes = std::fs::metadata(&output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        return Ok((output_path, size_bytes));
    }

    // Create the PDF document with the first page
    let (doc, first_page_idx, first_layer_idx) =
        PdfDocument::new(&draft.filename, page_width, page_height, "Layer 1");

    // Add fonts once for reuse
    let font_regular = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ToolError::failed(format!("Failed to add font: {e}")))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| ToolError::failed(format!("Failed to add font: {e}")))?;

    // Process first page
    let first_page = doc.get_page(first_page_idx);
    let first_layer = first_page.get_layer(first_layer_idx);
    render_page_content(
        &doc,
        first_layer,
        &draft.pages[0].content,
        &font_regular,
        &font_bold,
        page_width,
        page_height,
    )?;

    // Add and render remaining pages
    for page_data in draft.pages.iter().skip(1) {
        let (page_idx, layer_idx) = doc.add_page(
            page_width,
            page_height,
            format!("Layer {}", page_data.index + 1),
        );
        let page = doc.get_page(page_idx);
        let layer = page.get_layer(layer_idx);
        render_page_content(
            &doc,
            layer,
            &page_data.content,
            &font_regular,
            &font_bold,
            page_width,
            page_height,
        )?;
    }

    // Save the document
    let file = File::create(&output_path)
        .map_err(|e| ToolError::failed(format!("Failed to create PDF file: {e}")))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)
        .map_err(|e| ToolError::failed(format!("Failed to save PDF: {e}")))?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((output_path, size_bytes))
}

/// Get page dimensions in millimeters for a given page size.
const fn get_page_dimensions(page_size: &PdfPageSize) -> (Mm, Mm) {
    match page_size {
        PdfPageSize::A4 => (Mm(210.0), Mm(297.0)),
        PdfPageSize::Letter => (Mm(216.0), Mm(279.0)),
        PdfPageSize::Legal => (Mm(216.0), Mm(356.0)),
        PdfPageSize::A3 => (Mm(297.0), Mm(420.0)),
    }
}

/// Render content to a PDF layer.
fn render_page_content(
    _doc: &PdfDocumentReference,
    layer: PdfLayerReference,
    content: &PdfPageContent,
    font_regular: &IndirectFontRef,
    font_bold: &IndirectFontRef,
    page_width: Mm,
    page_height: Mm,
) -> Result<(), ToolError> {
    // Margins in mm
    let margin_left = Mm(20.0);
    let margin_right = Mm(20.0);
    let margin_top = Mm(20.0);
    let margin_bottom = Mm(20.0);

    // Usable width and height (Mm is a newtype around f32)
    let usable_width = page_width - margin_left - margin_right;
    let _usable_height = page_height - margin_top - margin_bottom;

    // Current y position (from top)
    let mut current_y = page_height - margin_top;

    // Font sizes (must be f32 for use_text)
    const FONT_SIZE_TITLE: f32 = 18.0;
    const FONT_SIZE_BODY: f32 = 12.0;
    const LINE_HEIGHT: f32 = 5.0;

    match content {
        PdfPageContent::Text { content } => {
            // Render text content with simple word wrapping
            for line in wrap_text(
                content,
                f64::from(usable_width.0),
                f64::from(FONT_SIZE_BODY),
            ) {
                if current_y < margin_bottom {
                    break; // No more space on page
                }
                layer.use_text(&line, FONT_SIZE_BODY, margin_left, current_y, font_regular);
                current_y -= Mm(FONT_SIZE_BODY + LINE_HEIGHT);
            }
        }
        PdfPageContent::Markdown { content } => {
            // Render markdown content (basic implementation)
            for line in content.lines() {
                if current_y < margin_bottom {
                    break;
                }

                if line.starts_with("# ") {
                    // Header level 1
                    layer.use_text(
                        &line[2..],
                        FONT_SIZE_TITLE,
                        margin_left,
                        current_y,
                        font_bold,
                    );
                    current_y -= Mm(FONT_SIZE_TITLE + LINE_HEIGHT);
                } else if line.starts_with("## ") {
                    // Header level 2
                    layer.use_text(
                        &line[3..],
                        FONT_SIZE_TITLE - 2.0,
                        margin_left,
                        current_y,
                        font_bold,
                    );
                    current_y -= Mm(FONT_SIZE_TITLE - 2.0 + LINE_HEIGHT);
                } else if line.starts_with("### ") {
                    // Header level 3
                    layer.use_text(
                        &line[4..],
                        FONT_SIZE_TITLE - 4.0,
                        margin_left,
                        current_y,
                        font_bold,
                    );
                    current_y -= Mm(FONT_SIZE_TITLE - 4.0 + LINE_HEIGHT);
                } else if !line.is_empty() {
                    // Regular paragraph
                    for wrapped_line in
                        wrap_text(line, f64::from(usable_width.0), f64::from(FONT_SIZE_BODY))
                    {
                        if current_y < margin_bottom {
                            break;
                        }
                        layer.use_text(
                            &wrapped_line,
                            FONT_SIZE_BODY,
                            margin_left,
                            current_y,
                            font_regular,
                        );
                        current_y -= Mm(FONT_SIZE_BODY + LINE_HEIGHT);
                    }
                } else {
                    // Empty line
                    current_y -= Mm(LINE_HEIGHT);
                }
            }
        }
        PdfPageContent::Html { content, styles: _ } => {
            // For HTML, strip tags and render as text (basic implementation)
            let text = html_to_plain_text(content);
            for line in wrap_text(&text, f64::from(usable_width.0), f64::from(FONT_SIZE_BODY)) {
                if current_y < margin_bottom {
                    break;
                }
                layer.use_text(&line, FONT_SIZE_BODY, margin_left, current_y, font_regular);
                current_y -= Mm(FONT_SIZE_BODY + LINE_HEIGHT);
            }
        }
        PdfPageContent::Image { path, caption } => {
            // Add a placeholder text for the image
            layer.use_text(
                format!("[Image: {path}]"),
                FONT_SIZE_BODY,
                margin_left,
                current_y,
                font_regular,
            );
            current_y -= Mm(FONT_SIZE_BODY + LINE_HEIGHT);

            // Add caption if present
            if let Some(caption_text) = caption {
                for line in wrap_text(
                    caption_text,
                    f64::from(usable_width.0),
                    f64::from(FONT_SIZE_BODY),
                ) {
                    if current_y < margin_bottom {
                        break;
                    }
                    layer.use_text(
                        &line,
                        FONT_SIZE_BODY - 2.0,
                        margin_left,
                        current_y,
                        font_regular,
                    );
                    current_y -= Mm(FONT_SIZE_BODY - 2.0 + LINE_HEIGHT);
                }
            }

            // Add note about image embedding
            layer.use_text(
                "(Image embedding will be implemented in a future update)",
                FONT_SIZE_BODY - 2.0,
                margin_left,
                current_y,
                font_regular,
            );
        }
    }

    Ok(())
}

/// Simple text wrapping function that breaks text into lines that fit within a given width.
fn wrap_text(text: &str, max_width_pt: f64, font_size: f64) -> Vec<String> {
    // Approximate character width (varies by font, using average for Helvetica)
    const AVG_CHAR_WIDTH_PT: f64 = 0.5;

    let max_chars = (max_width_pt / (font_size * AVG_CHAR_WIDTH_PT)) as usize;
    let max_chars = max_chars.max(20); // Minimum characters per line

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Handle explicit line breaks
    let mut result = Vec::new();
    for line in lines {
        for sub_line in line.split('\n') {
            result.push(sub_line.to_string());
        }
    }

    if result.is_empty() {
        result.push(text.to_string());
    }

    result
}

/// Convert HTML content to plain text (basic implementation).
fn html_to_plain_text(html: &str) -> String {
    let mut result = html.to_string();

    // Remove common HTML tags
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");
    result = result.replace("<p>", "\n");
    result = result.replace("</p>", "\n");
    result = result.replace("<div>", "\n");
    result = result.replace("</div>", "\n");
    result = result.replace("<h1>", "\n");
    result = result.replace("</h1>", "\n");
    result = result.replace("<h2>", "\n");
    result = result.replace("</h2>", "\n");
    result = result.replace("<h3>", "\n");
    result = result.replace("</h3>", "\n");

    // Remove remaining HTML tags (simple regex-like replacement)
    let mut cleaned = String::new();
    let mut in_tag = false;
    for c in result.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => cleaned.push(c),
            _ => {}
        }
    }

    // Clean up excessive whitespace
    cleaned.lines().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::{DraftPage, DraftStatus};
    use chrono::Utc;

    fn create_test_draft() -> PdfDraft {
        PdfDraft {
            pdf_id: "test_pdf".to_string(),
            filename: "test.pdf".to_string(),
            pages: vec![
                DraftPage {
                    id: "page_1".to_string(),
                    index: 0,
                    content: PdfPageContent::Text {
                        content: "First page".to_string(),
                    },
                },
                DraftPage {
                    id: "page_2".to_string(),
                    index: 1,
                    content: PdfPageContent::Markdown {
                        content: "# Second page".to_string(),
                    },
                },
            ],
            page_size: PdfPageSize::A4,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_get_page_dimensions() {
        let (w, h) = get_page_dimensions(&PdfPageSize::A4);
        assert_eq!(w.0, 210.0);
        assert_eq!(h.0, 297.0);

        let (w, h) = get_page_dimensions(&PdfPageSize::Letter);
        assert_eq!(w.0, 216.0);
        assert_eq!(h.0, 279.0);

        let (w, h) = get_page_dimensions(&PdfPageSize::Legal);
        assert_eq!(w.0, 216.0);
        assert_eq!(h.0, 356.0);

        let (w, h) = get_page_dimensions(&PdfPageSize::A3);
        assert_eq!(w.0, 297.0);
        assert_eq!(h.0, 420.0);
    }

    #[test]
    fn test_wrap_text() {
        let text = "Hello world this is a test";
        let wrapped = wrap_text(text, 100.0, 12.0);
        // Should split into multiple lines based on width
        assert!(!wrapped.is_empty());
        assert!(wrapped.iter().all(|l| l.len() <= text.len()));
    }

    #[test]
    fn test_html_to_plain_text() {
        let html = "<p>Hello <strong>world</strong></p>";
        let plain = html_to_plain_text(html);
        assert!(!plain.contains('<'));
        assert!(!plain.contains('>'));
        assert!(plain.contains("Hello"));
        assert!(plain.contains("world"));
    }

    #[test]
    fn test_html_to_plain_text_with_breaks() {
        let html = "Line1<br>Line2<br/>Line3";
        let plain = html_to_plain_text(html);
        assert!(plain.contains("Line1"));
        assert!(plain.contains("Line2"));
        assert!(plain.contains("Line3"));
    }
}
