//! Add a page to a PDF document.

use crate::core::documents::pdf::block::ImageFit;
use crate::core::documents::pdf::theme_patch::ThemePatch;
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    documents::shared::{get_draft_store, PdfInsertPosition, PdfPageContent},
    error::{parse_args, ToolError},
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tauri::AppHandle;

/// Arguments for adding a page to a PDF.
#[derive(Debug, Deserialize)]
pub struct PdfAddPageArgs {
    /// PDF ID from `pdf_start`
    pub pdf_id: String,
    /// Page content
    pub content: PdfPageContentInput,
    /// Position to insert (default: "end")
    #[serde(default)]
    pub position: String,
    /// Optional per-page styling patch, merged onto the document theme
    /// at render time. Shape matches the `theme` argument on `pdf_start`.
    /// Only body-level fields (typography, palette, spacing, margins)
    /// take effect — decorations (header/footer) and font families are
    /// fixed at document level.
    #[serde(default)]
    pub theme_override: Option<JsonValue>,
}

/// Page content input for serialization.
///
/// Mirrors [`PdfPageContent`] at the tool boundary. The `html` variant was
/// removed in T1.3 — callers must emit structured Markdown or plain text.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PdfPageContentInput {
    /// Markdown content variant.
    Markdown {
        /// The markdown source.
        content: String,
    },
    /// Plain text content variant.
    Text {
        /// The text content.
        content: String,
    },
    /// Image reference with optional caption and sizing.
    Image {
        /// Path to the image file.
        path: String,
        /// Optional caption.
        #[serde(default)]
        caption: Option<String>,
        /// Optional display width in mm.
        #[serde(default)]
        width_mm: Option<f32>,
        /// Optional display height in mm.
        #[serde(default)]
        height_mm: Option<f32>,
        /// Fit mode when both dimensions are set.
        #[serde(default)]
        fit: ImageFit,
    },
    /// Vertically-centred cover / title page.
    Cover {
        /// Main title.
        title: String,
        /// Optional subtitle.
        #[serde(default)]
        subtitle: Option<String>,
        /// Optional author line.
        #[serde(default)]
        author: Option<String>,
        /// Optional date line.
        #[serde(default)]
        date: Option<String>,
    },
}

impl From<PdfPageContentInput> for PdfPageContent {
    fn from(item: PdfPageContentInput) -> Self {
        match item {
            PdfPageContentInput::Markdown { content } => Self::Markdown { content },
            PdfPageContentInput::Text { content } => Self::Text { content },
            PdfPageContentInput::Image {
                path,
                caption,
                width_mm,
                height_mm,
                fit,
            } => Self::Image {
                path,
                caption,
                width_mm,
                height_mm,
                fit,
            },
            PdfPageContentInput::Cover {
                title,
                subtitle,
                author,
                date,
            } => Self::Cover {
                title,
                subtitle,
                author,
                date,
            },
        }
    }
}

fn parse_position(raw: &str) -> PdfInsertPosition {
    match raw.to_ascii_lowercase().as_str() {
        "start" => PdfInsertPosition::Start,
        _ => PdfInsertPosition::End,
    }
}

/// Decodes and validates the LLM-supplied `theme_override` argument.
///
/// Returns `Ok(None)` when the argument is missing or JSON null.
/// Returns `ToolError::invalid_args` with the dotted JSON path of the
/// offending field on shape or range failures so the LLM can self-correct.
fn decode_theme_override(raw: Option<JsonValue>) -> Result<Option<ThemePatch>, ToolError> {
    let Some(value) = raw else { return Ok(None) };
    if value.is_null() {
        return Ok(None);
    }
    let patch: ThemePatch = serde_json::from_value(value).map_err(|e| {
        ToolError::invalid_args(format!(
            "pdf_add_page: invalid `theme_override` argument: {e}"
        ))
    })?;
    patch
        .validate()
        .map_err(|e| ToolError::invalid_args(format!("pdf_add_page: {e}")))?;
    Ok(Some(patch))
}

/// Inclusive range for image display dimensions in mm.
///
/// The JSON schema advertises these bounds, but serde doesn't enforce
/// `minimum`/`maximum` keywords — so we validate explicitly at the tool
/// boundary to keep the promise. Zero or negative values would produce
/// nonsense DPI math downstream; oversize values would silently overflow
/// the page.
const IMAGE_DIMENSION_MIN_MM: f32 = 1.0;
const IMAGE_DIMENSION_MAX_MM: f32 = 400.0;

fn validate_image_sizing(content: &PdfPageContentInput) -> Result<(), ToolError> {
    let PdfPageContentInput::Image {
        width_mm,
        height_mm,
        ..
    } = content
    else {
        return Ok(());
    };
    check_image_mm("width_mm", *width_mm)?;
    check_image_mm("height_mm", *height_mm)?;
    Ok(())
}

fn check_image_mm(field: &str, value: Option<f32>) -> Result<(), ToolError> {
    let Some(v) = value else { return Ok(()) };
    if !v.is_finite() {
        return Err(ToolError::invalid_args(format!(
            "pdf_add_page: content.{field} must be a finite number"
        )));
    }
    if v < IMAGE_DIMENSION_MIN_MM || v > IMAGE_DIMENSION_MAX_MM {
        return Err(ToolError::invalid_args(format!(
            "pdf_add_page: content.{field} = {v:.2} out of range [{IMAGE_DIMENSION_MIN_MM:.2}, {IMAGE_DIMENSION_MAX_MM:.2}] mm"
        )));
    }
    Ok(())
}

/// Output from adding a page.
#[derive(Debug, Serialize)]
pub struct PdfAddPageOutput {
    /// Page ID
    pub page_id: String,
    /// PDF ID
    pub pdf_id: String,
    /// Page number (1-based)
    pub page_number: usize,
    /// Total page count
    pub total_pages: usize,
}

/// Tool to add a page to a PDF document.
pub struct PdfAddPageTool;

impl Tool for PdfAddPageTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("pdf", "add_page"),
            description: "Add a page to a PDF document. Use Markdown for rich content — the renderer \
              supports: # headings (H1–H6), **bold**, *italic*, `inline code`, [links](url), \
              unordered lists (- item), ordered lists (1. item), nested lists (indent 2+ spaces), \
              fenced code blocks (```lang ... ```), pipe tables (| H1 | H2 |\\n|---|---|\\n| a | b |), \
              blockquotes (> text → callout box), horizontal rules (--- → page break), \
              and images via the image content type. \
              Prefer 'markdown' type for most content. Use 'image' type only to embed a file from disk. \
              For formal documents, open with a 'cover' content as the first pdf_add_page call — it \
              renders a vertically-centred title page with optional subtitle/author/date. \
              For images, set width_mm and/or height_mm to control display size; fit ('contain' default, \
              preserves aspect ratio; 'stretch' uses both dimensions literally) decides what happens \
              when both are set. \
              Each pdf_add_page call adds one logical page of content; the renderer paginates automatically.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pdf_id": {
                        "type": "string",
                        "description": "PDF ID returned by pdf_start"
                    },
                    "content": {
                        "type": "object",
                        "description": "Page content. Use type='markdown' for rich text; type='text' for plain text; type='image' to embed a file with optional width_mm/height_mm/fit; type='cover' for a vertically-centred title page.",
                        "oneOf": [
                            {
                                "type": "object",
                                "required": ["type", "content"],
                                "properties": {
                                    "type": {"const": "markdown"},
                                    "content": {"type": "string", "description": "Markdown content with headings, lists, tables, code fences, blockquotes, bold/italic, links"}
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "content"],
                                "properties": {
                                    "type": {"const": "text"},
                                    "content": {"type": "string", "description": "Plain unformatted text"}
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "path"],
                                "properties": {
                                    "type": {"const": "image"},
                                    "path": {"type": "string", "description": "Absolute path to a PNG or JPEG image file"},
                                    "caption": {"type": "string", "description": "Optional image caption / alt text"},
                                    "width_mm": {"type": "number", "minimum": 1, "maximum": 400, "description": "Display width in mm. Omit to fit the page; omit with height_mm to derive from aspect."},
                                    "height_mm": {"type": "number", "minimum": 1, "maximum": 400, "description": "Display height in mm. Omit with width_mm to derive from aspect."},
                                    "fit": {"type": "string", "enum": ["contain", "stretch"], "default": "contain", "description": "When both dimensions are set: 'contain' preserves aspect (letterbox); 'stretch' uses both literally (may distort)."}
                                }
                            },
                            {
                                "type": "object",
                                "required": ["type", "title"],
                                "properties": {
                                    "type": {"const": "cover"},
                                    "title": {"type": "string", "description": "Title, rendered centred at heading size."},
                                    "subtitle": {"type": "string", "description": "Optional subtitle, italic, below the title."},
                                    "author": {"type": "string", "description": "Optional author line below the subtitle."},
                                    "date": {"type": "string", "description": "Optional date line below the author."}
                                }
                            }
                        ]
                    },
                    "position": {
                        "type": "string",
                        "enum": ["start", "end"],
                        "default": "end",
                        "description": "Where to insert this page (default: end)"
                    },
                    "theme_override": {
                        "type": ["object", "null"],
                        "additionalProperties": true,
                        "description": "Optional per-page styling patch, same shape as the document-level `theme` on pdf_start. Applies only to the page(s) that explicitly carry it; the renderer automatically resets to the document theme at the next page without an override. Only body-level fields (typography, palette, spacing, page_defaults.margin_mm) take effect — decorations (header/footer) and font family filenames are fixed at document level and cannot be changed mid-render. Pass null or omit the key to keep the document theme for this page."
                    }
                },
                "required": ["pdf_id", "content"]
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
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let args: PdfAddPageArgs = parse_args("pdf_add_page", arguments)?;

            // Validate image sizing BEFORE conversion so we surface the
            // tool-facing field name (content.width_mm / content.height_mm)
            // rather than a deserialized-struct error.
            validate_image_sizing(&args.content)?;

            // Convert content input to internal type
            let content: PdfPageContent = args.content.into();
            let position = parse_position(&args.position);
            let theme_override = decode_theme_override(args.theme_override)?;

            let draft_store = get_draft_store();
            let (page_id, page_number, total_pages) = {
                let mut store = draft_store.lock().map_err(|e| {
                    ToolError::failed(format!("Failed to acquire draft store lock: {e}"))
                })?;

                let page_id = store
                    .add_pdf_page(&args.pdf_id, content, position, theme_override)
                    .map_err(|e| ToolError::failed(format!("Failed to add page: {e}")))?;

                let draft = store
                    .get_pdf_draft(&args.pdf_id)
                    .ok_or_else(|| ToolError::failed(format!("PDF not found: {}", args.pdf_id)))?;
                let total = draft.pages.len();
                let index = draft
                    .pages
                    .iter()
                    .position(|p| p.id == page_id)
                    .unwrap_or(total.saturating_sub(1));
                (page_id, index + 1, total)
            };

            let output = PdfAddPageOutput {
                page_id,
                pdf_id: args.pdf_id,
                page_number,
                total_pages,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_conversion() {
        let input = PdfPageContentInput::Markdown {
            content: "# Title\n\nContent".to_string(),
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Markdown { content } => {
                assert_eq!(content, "# Title\n\nContent");
            }
            _ => panic!("Expected Markdown"),
        }
    }

    #[test]
    fn test_parse_position_defaults_to_end() {
        assert_eq!(parse_position(""), PdfInsertPosition::End);
        assert_eq!(parse_position("end"), PdfInsertPosition::End);
        assert_eq!(parse_position("END"), PdfInsertPosition::End);
        assert_eq!(parse_position("start"), PdfInsertPosition::Start);
        assert_eq!(parse_position("Start"), PdfInsertPosition::Start);
        assert_eq!(parse_position("bogus"), PdfInsertPosition::End);
    }

    #[test]
    fn test_image_content_conversion() {
        let input = PdfPageContentInput::Image {
            path: "/path/to/image.png".to_string(),
            caption: Some("A caption".to_string()),
            width_mm: None,
            height_mm: None,
            fit: ImageFit::Contain,
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Image {
                path,
                caption,
                width_mm,
                height_mm,
                fit,
            } => {
                assert_eq!(path, "/path/to/image.png");
                assert_eq!(caption, Some("A caption".to_string()));
                assert_eq!(width_mm, None);
                assert_eq!(height_mm, None);
                assert_eq!(fit, ImageFit::Contain);
            }
            _ => panic!("Expected Image"),
        }
    }

    #[test]
    fn test_image_content_conversion_preserves_sizing() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: Some(80.0),
            height_mm: Some(40.0),
            fit: ImageFit::Stretch,
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Image {
                width_mm,
                height_mm,
                fit,
                ..
            } => {
                assert_eq!(width_mm, Some(80.0));
                assert_eq!(height_mm, Some(40.0));
                assert_eq!(fit, ImageFit::Stretch);
            }
            _ => panic!("Expected Image"),
        }
    }

    #[test]
    fn validate_image_sizing_accepts_missing_dimensions() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: None,
            height_mm: None,
            fit: ImageFit::Contain,
        };
        validate_image_sizing(&input).expect("no dimensions is valid");
    }

    #[test]
    fn validate_image_sizing_accepts_in_range_dimensions() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: Some(80.0),
            height_mm: Some(40.0),
            fit: ImageFit::Contain,
        };
        validate_image_sizing(&input).expect("in-range dimensions accepted");
    }

    #[test]
    fn validate_image_sizing_rejects_zero_width() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: Some(0.0),
            height_mm: None,
            fit: ImageFit::Contain,
        };
        let err = validate_image_sizing(&input).expect_err("zero width must be rejected");
        assert!(err.to_string().contains("width_mm"));
    }

    #[test]
    fn validate_image_sizing_rejects_oversize_height() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: None,
            height_mm: Some(1000.0),
            fit: ImageFit::Contain,
        };
        let err = validate_image_sizing(&input).expect_err("oversize height must be rejected");
        assert!(err.to_string().contains("height_mm"));
    }

    #[test]
    fn validate_image_sizing_rejects_nan() {
        let input = PdfPageContentInput::Image {
            path: "/logo.png".to_string(),
            caption: None,
            width_mm: Some(f32::NAN),
            height_mm: None,
            fit: ImageFit::Contain,
        };
        let err = validate_image_sizing(&input).expect_err("NaN must be rejected");
        assert!(err.to_string().contains("width_mm"));
    }

    #[test]
    fn validate_image_sizing_ignores_non_image_content() {
        let input = PdfPageContentInput::Markdown {
            content: "# Heading".to_string(),
        };
        validate_image_sizing(&input).expect("non-image content must pass through");
    }

    #[test]
    fn decode_theme_override_accepts_missing() {
        assert!(decode_theme_override(None).unwrap().is_none());
    }

    #[test]
    fn decode_theme_override_accepts_null() {
        assert!(decode_theme_override(Some(serde_json::Value::Null))
            .unwrap()
            .is_none());
    }

    #[test]
    fn decode_theme_override_accepts_valid_patch() {
        let raw = serde_json::json!({
            "typography": {"body_font_size_pt": 14}
        });
        let patch = decode_theme_override(Some(raw)).expect("valid patch accepted");
        assert!(patch.is_some());
    }

    #[test]
    fn decode_theme_override_rejects_out_of_range() {
        let raw = serde_json::json!({
            "typography": {"body_font_size_pt": 200}
        });
        let err = decode_theme_override(Some(raw)).expect_err("out-of-range rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("theme.typography.body_font_size_pt"),
            "error must name the offending field: {msg}"
        );
    }

    #[test]
    fn test_cover_content_conversion() {
        let input = PdfPageContentInput::Cover {
            title: "Report".to_string(),
            subtitle: Some("2026".to_string()),
            author: Some("Ada".to_string()),
            date: None,
        };

        let content: PdfPageContent = input.into();
        match content {
            PdfPageContent::Cover {
                title,
                subtitle,
                author,
                date,
            } => {
                assert_eq!(title, "Report");
                assert_eq!(subtitle.as_deref(), Some("2026"));
                assert_eq!(author.as_deref(), Some("Ada"));
                assert_eq!(date, None);
            }
            _ => panic!("Expected Cover"),
        }
    }
}
