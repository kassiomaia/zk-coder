//! JSON schemas for DOCX tool inputs.
//!
//! Split out of the tool files so the schema mirror (theme + section
//! content) can be reused by `docx_start` and `docx_add_section` without
//! duplication, and so the schemas themselves can be tested.

use serde_json::{json, Value as JsonValue};

use crate::core::documents::docx::theme_patch::ALLOWED_FONTS;

/// JSON schema describing a [`DocxThemePatch`](crate::core::documents::docx::theme_patch::DocxThemePatch).
///
/// Every leaf is optional so the LLM can supply a minimal partial
/// override (e.g. "just recolor the headings"). `additionalProperties:
/// false` everywhere so typos become explicit errors instead of
/// silent drops.
pub fn theme_schema() -> JsonValue {
    json!({
        "type": ["object", "null"],
        "description": "Optional styling overlay. Omit or pass null to use the bundled report theme.",
        "additionalProperties": false,
        "properties": {
            "typography": typography_schema(),
            "palette": palette_schema(),
            "spacing": spacing_schema(),
            "page": page_schema(),
            "decorations": decorations_schema(),
            "fonts": fonts_schema()
        }
    })
}

fn typography_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Font sizes and line spacing.",
        "properties": {
            "body_font_size_pt": { "type": "integer", "minimum": 6, "maximum": 72, "description": "Body font size in points." },
            "heading_font_size_pt": { "type": "integer", "minimum": 8, "maximum": 96, "description": "Base heading size; deeper levels scale down by heading_scale." },
            "line_spacing": { "type": "number", "minimum": 1.0, "maximum": 2.5, "description": "Line-spacing multiplier applied to every paragraph." },
            "heading_scale": { "type": "number", "minimum": 0.5, "maximum": 1.0, "description": "Per-level heading scaling. 0.82 gives a gentle descending series." }
        }
    })
}

fn palette_schema() -> JsonValue {
    let rgb = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "hex": { "type": "string", "description": "RRGGBB hex color, with or without a leading '#'." }
        }
    });
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Named color slots.",
        "properties": {
            "text": rgb,
            "heading": rgb,
            "link": rgb,
            "code_text": rgb,
            "code_bg": rgb,
            "table_header_bg": rgb,
            "table_border": rgb
        }
    })
}

fn spacing_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Paragraph and list spacing, all in points.",
        "properties": {
            "paragraph_space_before_pt": { "type": "integer", "minimum": 0, "maximum": 72 },
            "paragraph_space_after_pt": { "type": "integer", "minimum": 0, "maximum": 72 },
            "list_indent_pt": { "type": "integer", "minimum": 0, "maximum": 144 },
            "table_cell_padding_pt": { "type": "integer", "minimum": 0, "maximum": 36 }
        }
    })
}

fn page_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Page geometry.",
        "properties": {
            "paper": { "type": "string", "enum": ["A4", "Letter", "Legal"] },
            "landscape": { "type": "boolean" },
            "margin_top_mm": { "type": "number", "minimum": 5.0, "maximum": 80.0 },
            "margin_bottom_mm": { "type": "number", "minimum": 5.0, "maximum": 80.0 },
            "margin_left_mm": { "type": "number", "minimum": 5.0, "maximum": 80.0 },
            "margin_right_mm": { "type": "number", "minimum": 5.0, "maximum": 80.0 }
        }
    })
}

fn decorations_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Header / footer templates. Supports tokens {title}, {page}, {total}. Pass explicit null to clear either template.",
        "properties": {
            "header": { "type": ["string", "null"] },
            "footer": { "type": ["string", "null"] },
            "different_first_page": { "type": "boolean" }
        }
    })
}

fn fonts_schema() -> JsonValue {
    let allowed: Vec<JsonValue> = ALLOWED_FONTS.iter().map(|s| json!(s)).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Font family names. Must be one of the bundled Office system fonts.",
        "properties": {
            "body":    { "type": "string", "enum": allowed.clone() },
            "heading": { "type": "string", "enum": allowed.clone() },
            "mono":    { "type": "string", "enum": allowed }
        }
    })
}

/// JSON schema for the polymorphic `content` argument on
/// `docx_add_section`. Split out so the schema is easier to read and
/// reusable if we add other content sinks.
pub fn section_content_schema() -> JsonValue {
    let paragraph_text = paragraph_text_schema();
    let alignment = json!({ "type": "string", "enum": ["left", "center", "right", "justify"] });
    let list_item = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["text"],
        "properties": {
            "text": paragraph_text,
            "level": { "type": "integer", "minimum": 0, "maximum": 8, "description": "Nesting level, 0-8." }
        }
    });
    let paragraph_role = json!({ "type": "string", "enum": ["body", "quote", "code"] });
    json!({
        "description": "Section content. One of: Heading, Paragraph, BulletList, NumberedList, Table, Image, PageBreak, HorizontalRule.",
        "oneOf": [
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "level", "text"],
                "properties": {
                    "type": { "const": "Heading" },
                    "level": { "type": "integer", "minimum": 1, "maximum": 9 },
                    "text": paragraph_text,
                    "alignment": alignment,
                    "page_break_before": { "type": "boolean" }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "text"],
                "properties": {
                    "type": { "const": "Paragraph" },
                    "text": paragraph_text,
                    "alignment": alignment,
                    "role": paragraph_role
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "items"],
                "properties": {
                    "type": { "const": "BulletList" },
                    "items": { "type": "array", "items": list_item }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "items"],
                "properties": {
                    "type": { "const": "NumberedList" },
                    "items": { "type": "array", "items": list_item },
                    "start_at": { "type": "integer", "minimum": 1 }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "headers", "rows"],
                "properties": {
                    "type": { "const": "Table" },
                    "headers": { "type": "array", "items": paragraph_text },
                    "rows":    { "type": "array", "items": { "type": "array", "items": paragraph_text } },
                    "caption": { "type": "string" }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type", "path"],
                "properties": {
                    "type": { "const": "Image" },
                    "path": { "type": "string", "description": "Absolute path to a PNG or JPEG file." },
                    "caption": { "type": "string" },
                    "width_mm": { "type": "number", "minimum": 10.0, "maximum": 300.0, "description": "Render width in millimetres." }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type"],
                "properties": { "type": { "const": "PageBreak" } }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["type"],
                "properties": { "type": { "const": "HorizontalRule" } }
            }
        ]
    })
}

fn paragraph_text_schema() -> JsonValue {
    json!({
        "description": "Either a plain string or an array of runs with inline formatting.",
        "oneOf": [
            { "type": "string" },
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["text"],
                    "properties": {
                        "text":      { "type": "string" },
                        "bold":      { "type": "boolean" },
                        "italic":    { "type": "boolean" },
                        "underline": { "type": "boolean" },
                        "strike":    { "type": "boolean" },
                        "code":      { "type": "boolean" },
                        "color":     { "type": "string", "description": "RRGGBB hex override." },
                        "link":      { "type": "string", "description": "External URL. Wraps the run in a hyperlink." }
                    }
                }
            }
        ]
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn theme_schema_has_all_sections() {
        let s = theme_schema();
        let props = s["properties"].as_object().unwrap();
        for key in ["typography", "palette", "spacing", "page", "decorations", "fonts"] {
            assert!(props.contains_key(key), "missing {key}");
        }
    }

    #[test]
    fn section_content_schema_lists_all_variants() {
        let s = section_content_schema();
        assert_eq!(s["oneOf"].as_array().unwrap().len(), 8);
    }
}
