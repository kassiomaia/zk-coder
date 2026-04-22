//! Start a new PDF document.

use crate::core::documents::pdf::theme_patch::ThemePatch;
use crate::infrastructure::tools::{
    documents::shared::{get_draft_store, sanitize_filename, PdfPageSize},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for starting a PDF document.
#[derive(Debug, Deserialize)]
pub struct PdfStartArgs {
    /// Output filename (should end in .pdf)
    pub filename: String,
    /// Document title
    pub title: String,
    /// Page size (default: A4)
    #[serde(default)]
    pub page_size: Option<String>,
    /// Optional styling patch overlaid on the bundled report theme.
    ///
    /// Raw JSON so invalid shapes produce a clear `ToolError` instead of
    /// a generic serde decode failure at tool-call parse time.
    #[serde(default)]
    pub theme: Option<JsonValue>,
}

/// Output from starting a PDF document.
#[derive(Debug, Serialize)]
pub struct PdfStartOutput {
    /// PDF ID for subsequent operations
    pub pdf_id: String,
    /// Filename
    pub filename: String,
    /// Title
    pub title: String,
    /// Page size
    pub page_size: String,
    /// Current status (always "building" on start)
    pub status: String,
    /// Page count (always 0 on start)
    pub page_count: usize,
    /// When the document was created
    pub created_at: String,
    /// Whether a caller-supplied theme patch was accepted and stored.
    pub theme_applied: bool,
}

/// Tool to start a new PDF document.
pub struct PdfStartTool;

impl Tool for PdfStartTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "start"),
            description: "Create a professional PDF document. \
              Workflow: (1) call pdf_start, (2) call pdf_add_page one or more times to add content, \
              (3) call pdf_finalize to render and save the PDF. \
              The rendered PDF supports: headings (H1-H6), paragraphs with **bold**, *italic*, \
              `code`, and [links](url), ordered/unordered/nested lists, fenced code blocks \
              (```lang), pipe tables (| A | B |), blockquotes (> text) rendered as callout boxes, \
              horizontal rules (---) as page breaks, and embedded images. \
              The output includes page headers/footers with page numbers, a font-embedded \
              professional layout, and PDF bookmarks from headings. \
              \
              STYLING: pass an optional `theme` object to override the bundled defaults. Every \
              field is optional — supply only what the user asked to change. Supported sections: \
              `typography` (body_font_size_pt, heading_font_size_pt, line_leading), \
              `palette` (text, heading, code_bg, code_text, callout_bg, callout_border, \
              table_header_bg, table_border — each an RGB triple with r/g/b in [0,1]), \
              `spacing` (block_gap, list_indent_mm, block_padding_mm, table_cell_padding_mm), \
              `page_defaults.margin_mm`, `decorations` (header, footer — templates with tokens \
              {title}, {page}, {total_pages}, {section}; suppress_on_first_page; font_size_pt; \
              margin_mm), and `fonts` (regular, bold, italic, mono — must reference bundled \
              filenames: LiberationSans-Regular.ttf, LiberationSans-Bold.ttf, \
              LiberationSans-Italic.ttf, LiberationMono-Regular.ttf, DejaVuSans.ttf). \
              Example: \"theme\": { \"palette\": { \"heading\": { \"r\": 0.1, \"g\": 0.2, \"b\": 0.8 } }, \
              \"typography\": { \"body_font_size_pt\": 13 } }. Set `decorations.header` or \
              `decorations.footer` to null to drop that strip entirely. \
              \
              DO NOT write scripts to generate PDFs — use this tool instead. \
              Never use pandoc, LaTeX, or shell commands to generate PDFs — this tool replaces all of those.",
            input_schema: build_input_schema(),
            requires_workspace: false,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: crate::infrastructure::tools::context::ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PdfStartArgs = parse_args("pdf_start", arguments)?;

            let filename = sanitize_filename(&args.filename, "pdf");
            let page_size = parse_page_size(args.page_size.as_deref().unwrap_or("A4"));
            let theme_patch = decode_theme_patch(args.theme)?;
            let theme_applied = theme_patch.is_some();

            let draft_store = get_draft_store();
            let pdf_id = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                store.create_pdf_draft(
                    filename.clone(),
                    args.title.clone(),
                    Some(page_size),
                    theme_patch,
                )
            };

            let created_at = chrono::Utc::now().to_rfc3339();
            let page_size_str = format!("{page_size:?}");

            let output = PdfStartOutput {
                pdf_id,
                filename,
                title: args.title,
                page_size: page_size_str,
                status: "building".to_string(),
                page_count: 0,
                created_at,
                theme_applied,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

/// Decodes and validates the LLM-supplied `theme` argument.
///
/// Returns `Ok(None)` when the argument is missing or `null`. Returns a
/// [`ToolError::invalid_args`] naming the offending field on any
/// deserialization or validation failure so the LLM can self-correct.
fn decode_theme_patch(raw: Option<JsonValue>) -> Result<Option<ThemePatch>, ToolError> {
    let Some(value) = raw else { return Ok(None) };
    if value.is_null() {
        return Ok(None);
    }
    let patch: ThemePatch = serde_json::from_value(value).map_err(|e| {
        ToolError::invalid_args(format!("pdf_start: invalid `theme` argument: {e}"))
    })?;
    patch
        .validate()
        .map_err(|e| ToolError::invalid_args(format!("pdf_start: {e}")))?;
    Ok(Some(patch))
}

/// Parse page size string to `PdfPageSize` enum.
fn parse_page_size(s: &str) -> PdfPageSize {
    match s.to_uppercase().as_str() {
        "LETTER" => PdfPageSize::Letter,
        "LEGAL" => PdfPageSize::Legal,
        "A3" => PdfPageSize::A3,
        _ => PdfPageSize::A4,
    }
}

/// Build the tool's input schema.
///
/// Kept as a function so the large `theme` sub-schema does not crowd
/// `Tool::meta`. The shape mirrors `ThemePatch` and doubles as
/// LLM-visible documentation.
fn build_input_schema() -> JsonValue {
    let rgb_patch = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Normalised RGB triple; each channel in [0.0, 1.0]. Omit channels to keep the base.",
        "properties": {
            "r": {"type": "number", "minimum": 0, "maximum": 1},
            "g": {"type": "number", "minimum": 0, "maximum": 1},
            "b": {"type": "number", "minimum": 0, "maximum": 1}
        }
    });

    let theme_schema = serde_json::json!({
        "type": ["object", "null"],
        "additionalProperties": false,
        "description": "Optional styling overlay on top of the bundled report theme. Every field is optional — supply only the knobs the user asked to change. Pass explicit JSON null (or omit the key) to keep all defaults. Field-level ranges are enforced; out-of-range values reject the call with a descriptive error. Unknown section or field names are also rejected, so typos fail loudly instead of being silently ignored.",
        "properties": {
            "typography": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "body_font_size_pt":    {"type": "number", "minimum": 6,  "maximum": 72, "description": "Body font size in points (default 11)."},
                    "heading_font_size_pt": {"type": "number", "minimum": 8,  "maximum": 96, "description": "Heading font size in points (default 18)."},
                    "line_leading":         {"type": "number", "minimum": 1.0, "maximum": 2.5, "description": "Line spacing multiplier (default 1.3). 2.0 ≈ double-spaced."}
                }
            },
            "palette": {
                "type": "object",
                "additionalProperties": false,
                "description": "Colors for text, headings, code blocks, callouts, and tables.",
                "properties": {
                    "text":            rgb_patch,
                    "heading":         rgb_patch,
                    "code_bg":         rgb_patch,
                    "code_text":       rgb_patch,
                    "callout_bg":      rgb_patch,
                    "callout_border":  rgb_patch,
                    "table_header_bg": rgb_patch,
                    "table_border":    rgb_patch
                }
            },
            "spacing": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "block_gap":             {"type": "number", "minimum": 0, "maximum": 5, "description": "Vertical gap between blocks as a multiplier of body leading (default 0.4)."},
                    "list_indent_mm":        {"type": "number", "minimum": 0, "maximum": 40, "description": "Indent per list nesting level, mm (default 6)."},
                    "block_padding_mm":      {"type": "number", "minimum": 0, "maximum": 20, "description": "Internal padding of code blocks and callouts, mm (default 2.5)."},
                    "table_cell_padding_mm": {"type": "number", "minimum": 0, "maximum": 20, "description": "Cell padding inside tables, mm (default 1.5)."}
                }
            },
            "page_defaults": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "margin_mm": {"type": "number", "minimum": 5, "maximum": 80, "description": "Uniform page margin in mm (default 20)."}
                }
            },
            "decorations": {
                "type": "object",
                "additionalProperties": false,
                "description": "Header/footer strips. Tokens: {title}, {page}, {total_pages}, {section}. Pass null to drop a strip.",
                "properties": {
                    "header": {"type": ["string", "null"], "description": "Header template, or null to omit."},
                    "footer": {"type": ["string", "null"], "description": "Footer template, or null to omit."},
                    "suppress_on_first_page": {"type": "boolean", "description": "Skip header/footer on page 1 (cover convention)."},
                    "font_size_pt": {"type": "number", "minimum": 6, "maximum": 20, "description": "Header/footer text size (default 9)."},
                    "margin_mm":    {"type": "number", "minimum": 2, "maximum": 30, "description": "Distance from page edge to header/footer baseline, mm (default 10)."}
                }
            },
            "fonts": {
                "type": "object",
                "additionalProperties": false,
                "description": "Font filenames. Must reference bundled TTF files: LiberationSans-Regular.ttf, LiberationSans-Bold.ttf, LiberationSans-Italic.ttf, LiberationMono-Regular.ttf, DejaVuSans.ttf.",
                "properties": {
                    "regular":   {"type": "string"},
                    "bold":      {"type": "string"},
                    "italic":    {"type": "string"},
                    "mono":      {"type": "string"},
                    "fallbacks": {"type": "array", "items": {"type": "string"}}
                }
            }
        }
    });

    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "filename": {
                "type": "string",
                "description": "Output filename (should end in .pdf)"
            },
            "title": {
                "type": "string",
                "description": "Document title"
            },
            "page_size": {
                "type": "string",
                "description": "Page size",
                "enum": ["A4", "Letter", "Legal", "A3"],
                "default": "A4"
            },
            "theme": theme_schema
        },
        "required": ["filename", "title"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_size() {
        assert_eq!(parse_page_size("A4"), PdfPageSize::A4);
        assert_eq!(parse_page_size("a4"), PdfPageSize::A4);
        assert_eq!(parse_page_size("Letter"), PdfPageSize::Letter);
        assert_eq!(parse_page_size("letter"), PdfPageSize::Letter);
        assert_eq!(parse_page_size("LEGAL"), PdfPageSize::Legal);
        assert_eq!(parse_page_size("A3"), PdfPageSize::A3);
        assert_eq!(parse_page_size("unknown"), PdfPageSize::A4); // default
    }

    #[test]
    fn test_args_parsing() {
        let args = serde_json::json!({
            "filename": "document.pdf",
            "title": "My Document"
        });

        let parsed: PdfStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.filename, "document.pdf");
        assert_eq!(parsed.title, "My Document");
        assert!(parsed.page_size.is_none());
        assert!(parsed.theme.is_none());
    }

    #[test]
    fn test_args_with_page_size() {
        let args = serde_json::json!({
            "filename": "report.pdf",
            "title": "Report",
            "page_size": "Letter"
        });

        let parsed: PdfStartArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.page_size, Some("Letter".to_string()));
    }

    #[test]
    fn test_args_with_theme_round_trips() {
        let args = serde_json::json!({
            "filename": "styled.pdf",
            "title": "Styled",
            "theme": {
                "typography": {"body_font_size_pt": 13},
                "palette": {"heading": {"r": 0.1, "g": 0.2, "b": 0.8}}
            }
        });

        let parsed: PdfStartArgs = serde_json::from_value(args).unwrap();
        assert!(parsed.theme.is_some());
        let patch = decode_theme_patch(parsed.theme).expect("valid theme accepted");
        assert!(patch.is_some());
    }

    #[test]
    fn decode_theme_patch_rejects_out_of_range() {
        let raw = serde_json::json!({
            "palette": {"heading": {"r": 2.0}}
        });
        let err = decode_theme_patch(Some(raw)).expect_err("rgb > 1 must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("theme.palette.heading.r"),
            "error must name the offending field: {msg}"
        );
    }

    #[test]
    fn decode_theme_patch_rejects_unknown_font() {
        let raw = serde_json::json!({
            "fonts": {"regular": "ComicSans.ttf"}
        });
        let err = decode_theme_patch(Some(raw)).expect_err("unbundled font must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("theme.fonts.regular"));
        assert!(msg.contains("ComicSans.ttf"));
    }

    #[test]
    fn decode_theme_patch_accepts_null() {
        let patch = decode_theme_patch(Some(serde_json::Value::Null)).expect("null is fine");
        assert!(patch.is_none());
    }

    #[test]
    fn decode_theme_patch_accepts_missing() {
        let patch = decode_theme_patch(None).expect("absent is fine");
        assert!(patch.is_none());
    }

    #[test]
    fn sanitize_filename_strips_directory_components() {
        assert_eq!(
            sanitize_filename("../../../etc/cron.d/job.pdf", "pdf"),
            "job.pdf"
        );
    }

    #[test]
    fn sanitize_filename_handles_separators() {
        assert_eq!(sanitize_filename("sub/dir/file.pdf", "pdf"), "file.pdf");
    }

    #[test]
    fn sanitize_filename_handles_empty() {
        assert_eq!(sanitize_filename("", "pdf"), "document.pdf");
    }

    #[test]
    fn sanitize_filename_preserves_normal_names() {
        assert_eq!(sanitize_filename("report.pdf", "pdf"), "report.pdf");
        assert_eq!(sanitize_filename("my-report", "pdf"), "my-report.pdf");
    }

    #[test]
    fn sanitize_filename_handles_dot_dot_only() {
        assert_eq!(sanitize_filename("..", "pdf"), "document.pdf");
    }

    #[test]
    fn test_output_serialization() {
        let output = PdfStartOutput {
            pdf_id: "pdf_123".to_string(),
            filename: "test.pdf".to_string(),
            title: "Test".to_string(),
            page_size: "A4".to_string(),
            status: "Building".to_string(),
            page_count: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            theme_applied: false,
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["pdf_id"], "pdf_123");
        assert_eq!(json["filename"], "test.pdf");
        assert_eq!(json["status"], "Building");
        assert_eq!(json["page_count"], 0);
        assert_eq!(json["theme_applied"], false);
    }
}
