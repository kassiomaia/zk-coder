//! LLM-supplied styling overlay for a DOCX draft.
//!
//! Mirrors [`DocxTheme`](super::theme::DocxTheme) but every leaf is
//! optional. The tool layer deserialises the JSON, [`DocxThemePatch::validate`]
//! enforces safe ranges, and [`DocxTheme::apply_patch`](super::theme::DocxTheme::apply_patch)
//! merges the patch over a base theme — `Some` replaces, `None` leaves
//! the base untouched.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::theme::PaperSize;

/// Font families the LLM can reference. Restricted to widely-available
/// Office system fonts so the produced document opens in Word without
/// font-substitution surprises.
pub const ALLOWED_FONTS: &[&str] = &[
    "Calibri",
    "Cambria",
    "Arial",
    "Helvetica",
    "Times New Roman",
    "Georgia",
    "Verdana",
    "Tahoma",
    "Courier New",
    "Consolas",
];

/// Validation failure for a [`DocxThemePatch`]. `field` carries the
/// dotted JSON path so the LLM can self-correct.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DocxThemeError {
    /// Numeric value fell outside the allowed range.
    #[error("{field}: value {value} out of range [{min}, {max}]")]
    OutOfRange {
        /// Dotted JSON path of the invalid field.
        field: String,
        /// Offending value (formatted as string).
        value: String,
        /// Inclusive lower bound.
        min: String,
        /// Inclusive upper bound.
        max: String,
    },
    /// Numeric value is NaN or infinite.
    #[error("{field}: value must be a finite number")]
    NotFinite {
        /// Dotted JSON path.
        field: String,
    },
    /// Hex color string is malformed.
    #[error("{field}: '{value}' is not a 6-character RRGGBB hex color")]
    InvalidColor {
        /// Dotted JSON path.
        field: String,
        /// Offending input.
        value: String,
    },
    /// Font family name is not in [`ALLOWED_FONTS`].
    #[error("{field}: font '{value}' is not supported; expected one of {allowed}")]
    UnknownFont {
        /// Dotted JSON path.
        field: String,
        /// Offending font name.
        value: String,
        /// Comma-separated list of accepted names.
        allowed: String,
    },
}

/// Optional RGB override. Accepts `{ "hex": "3B82F6" }` or
/// `{ "hex": "#3B82F6" }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RgbPatch {
    /// RRGGBB hex string, with or without a leading `#`.
    #[serde(default)]
    pub hex: Option<String>,
}

/// Optional override for [`Typography`](super::theme::Typography).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct TypographyPatch {
    /// Body font size, in points. Range `[6, 72]`.
    #[serde(default)]
    pub body_font_size_pt: Option<u16>,
    /// Base heading font size, in points. Range `[8, 96]`.
    #[serde(default)]
    pub heading_font_size_pt: Option<u16>,
    /// Line spacing multiplier. Range `[1.0, 2.5]`.
    #[serde(default)]
    pub line_spacing: Option<f32>,
    /// Per-level heading scale. Range `[0.5, 1.0]`.
    #[serde(default)]
    pub heading_scale: Option<f32>,
}

/// Optional override for [`Palette`](super::theme::Palette).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalettePatch {
    /// Body text color.
    #[serde(default)]
    pub text: Option<RgbPatch>,
    /// Heading color.
    #[serde(default)]
    pub heading: Option<RgbPatch>,
    /// Hyperlink color.
    #[serde(default)]
    pub link: Option<RgbPatch>,
    /// Code foreground.
    #[serde(default)]
    pub code_text: Option<RgbPatch>,
    /// Code background.
    #[serde(default)]
    pub code_bg: Option<RgbPatch>,
    /// Table header row background.
    #[serde(default)]
    pub table_header_bg: Option<RgbPatch>,
    /// Table gridline color.
    #[serde(default)]
    pub table_border: Option<RgbPatch>,
}

/// Optional override for [`Spacing`](super::theme::Spacing).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpacingPatch {
    /// Space above paragraph, in points. Range `[0, 72]`.
    #[serde(default)]
    pub paragraph_space_before_pt: Option<u16>,
    /// Space below paragraph, in points. Range `[0, 72]`.
    #[serde(default)]
    pub paragraph_space_after_pt: Option<u16>,
    /// Per-level list indent, in points. Range `[0, 144]`.
    #[serde(default)]
    pub list_indent_pt: Option<u16>,
    /// Padding inside table cells, in points. Range `[0, 36]`.
    #[serde(default)]
    pub table_cell_padding_pt: Option<u16>,
}

/// Optional override for [`PageDefaults`](super::theme::PageDefaults).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PagePatch {
    /// Paper size.
    #[serde(default)]
    pub paper: Option<PaperSize>,
    /// Landscape orientation when `true`.
    #[serde(default)]
    pub landscape: Option<bool>,
    /// Top margin in mm. Range `[5, 80]`.
    #[serde(default)]
    pub margin_top_mm: Option<f32>,
    /// Bottom margin in mm. Range `[5, 80]`.
    #[serde(default)]
    pub margin_bottom_mm: Option<f32>,
    /// Left margin in mm. Range `[5, 80]`.
    #[serde(default)]
    pub margin_left_mm: Option<f32>,
    /// Right margin in mm. Range `[5, 80]`.
    #[serde(default)]
    pub margin_right_mm: Option<f32>,
}

/// Optional override for [`Decorations`](super::theme::Decorations).
///
/// `header` and `footer` accept any string or `null`. Passing explicit
/// `null` clears the template; omitting the key leaves the base theme
/// alone. See [`DoubleOption`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorationsPatch {
    /// Header template. Supports tokens `{title}`, `{page}`, `{total}`.
    #[serde(
        default,
        deserialize_with = "deserialize_double_optional",
        skip_serializing_if = "DoubleOption::is_unset"
    )]
    pub header: DoubleOption<String>,
    /// Footer template. Same tokens as `header`.
    #[serde(
        default,
        deserialize_with = "deserialize_double_optional",
        skip_serializing_if = "DoubleOption::is_unset"
    )]
    pub footer: DoubleOption<String>,
    /// Use an empty header/footer on the first page.
    #[serde(default)]
    pub different_first_page: Option<bool>,
}

/// Optional override for [`Fonts`](super::theme::Fonts).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontsPatch {
    /// Body font family. Must be in [`ALLOWED_FONTS`].
    #[serde(default)]
    pub body: Option<String>,
    /// Heading font family.
    #[serde(default)]
    pub heading: Option<String>,
    /// Monospace font family.
    #[serde(default)]
    pub mono: Option<String>,
}

/// LLM-facing root patch. `deny_unknown_fields` everywhere so typos
/// fail loudly instead of being silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct DocxThemePatch {
    /// Font sizes and line spacing.
    #[serde(default)]
    pub typography: Option<TypographyPatch>,
    /// Color slots.
    #[serde(default)]
    pub palette: Option<PalettePatch>,
    /// Paragraph and list spacing.
    #[serde(default)]
    pub spacing: Option<SpacingPatch>,
    /// Page geometry.
    #[serde(default)]
    pub page: Option<PagePatch>,
    /// Header / footer templates.
    #[serde(default)]
    pub decorations: Option<DecorationsPatch>,
    /// Font family names.
    #[serde(default)]
    pub fonts: Option<FontsPatch>,
}

impl DocxThemePatch {
    /// Returns `Ok(())` when every supplied field falls within the
    /// allowed range. Returns the first [`DocxThemeError`] otherwise.
    ///
    /// # Errors
    /// Returns a [`DocxThemeError`] describing the offending path.
    pub fn validate(&self) -> Result<(), DocxThemeError> {
        if let Some(t) = self.typography.as_ref() {
            check_u16(
                "theme.typography.body_font_size_pt",
                t.body_font_size_pt,
                6,
                72,
            )?;
            check_u16(
                "theme.typography.heading_font_size_pt",
                t.heading_font_size_pt,
                8,
                96,
            )?;
            check_f32("theme.typography.line_spacing", t.line_spacing, 1.0, 2.5)?;
            check_f32("theme.typography.heading_scale", t.heading_scale, 0.5, 1.0)?;
        }
        if let Some(p) = self.palette.as_ref() {
            check_rgb("theme.palette.text", p.text.as_ref())?;
            check_rgb("theme.palette.heading", p.heading.as_ref())?;
            check_rgb("theme.palette.link", p.link.as_ref())?;
            check_rgb("theme.palette.code_text", p.code_text.as_ref())?;
            check_rgb("theme.palette.code_bg", p.code_bg.as_ref())?;
            check_rgb("theme.palette.table_header_bg", p.table_header_bg.as_ref())?;
            check_rgb("theme.palette.table_border", p.table_border.as_ref())?;
        }
        if let Some(s) = self.spacing.as_ref() {
            check_u16(
                "theme.spacing.paragraph_space_before_pt",
                s.paragraph_space_before_pt,
                0,
                72,
            )?;
            check_u16(
                "theme.spacing.paragraph_space_after_pt",
                s.paragraph_space_after_pt,
                0,
                72,
            )?;
            check_u16("theme.spacing.list_indent_pt", s.list_indent_pt, 0, 144)?;
            check_u16(
                "theme.spacing.table_cell_padding_pt",
                s.table_cell_padding_pt,
                0,
                36,
            )?;
        }
        if let Some(p) = self.page.as_ref() {
            check_f32("theme.page.margin_top_mm", p.margin_top_mm, 5.0, 80.0)?;
            check_f32("theme.page.margin_bottom_mm", p.margin_bottom_mm, 5.0, 80.0)?;
            check_f32("theme.page.margin_left_mm", p.margin_left_mm, 5.0, 80.0)?;
            check_f32("theme.page.margin_right_mm", p.margin_right_mm, 5.0, 80.0)?;
        }
        if let Some(f) = self.fonts.as_ref() {
            check_font("theme.fonts.body", f.body.as_deref())?;
            check_font("theme.fonts.heading", f.heading.as_deref())?;
            check_font("theme.fonts.mono", f.mono.as_deref())?;
        }
        Ok(())
    }
}

fn check_u16(field: &str, value: Option<u16>, min: u16, max: u16) -> Result<(), DocxThemeError> {
    let Some(v) = value else { return Ok(()) };
    if v < min || v > max {
        return Err(DocxThemeError::OutOfRange {
            field: field.to_string(),
            value: v.to_string(),
            min: min.to_string(),
            max: max.to_string(),
        });
    }
    Ok(())
}

fn check_f32(field: &str, value: Option<f32>, min: f32, max: f32) -> Result<(), DocxThemeError> {
    let Some(v) = value else { return Ok(()) };
    if !v.is_finite() {
        return Err(DocxThemeError::NotFinite {
            field: field.to_string(),
        });
    }
    if v < min || v > max {
        return Err(DocxThemeError::OutOfRange {
            field: field.to_string(),
            value: format!("{v:.2}"),
            min: format!("{min:.2}"),
            max: format!("{max:.2}"),
        });
    }
    Ok(())
}

fn check_rgb(prefix: &str, rgb: Option<&RgbPatch>) -> Result<(), DocxThemeError> {
    let Some(rgb) = rgb else { return Ok(()) };
    let Some(hex) = rgb.hex.as_deref() else {
        return Ok(());
    };
    let trimmed = hex.trim_start_matches('#');
    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(DocxThemeError::InvalidColor {
            field: format!("{prefix}.hex"),
            value: hex.to_string(),
        })
    }
}

fn check_font(field: &str, name: Option<&str>) -> Result<(), DocxThemeError> {
    let Some(name) = name else { return Ok(()) };
    if ALLOWED_FONTS.iter().any(|b| b.eq_ignore_ascii_case(name)) {
        Ok(())
    } else {
        Err(DocxThemeError::UnknownFont {
            field: field.to_string(),
            value: name.to_string(),
            allowed: ALLOWED_FONTS.join(", "),
        })
    }
}

/// Three-state option: JSON `null` clears; missing key leaves base.
/// Serde normally collapses both to `Option::None`, so we keep an
/// explicit marker. See the PDF counterpart for the rationale.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DoubleOption<T> {
    /// Key absent from the JSON object.
    #[default]
    Unset,
    /// Key present with value `null`.
    Null,
    /// Key present with a real value.
    Set(T),
}

impl<T> DoubleOption<T> {
    /// `true` when this is the `Unset` variant. Paired with
    /// `skip_serializing_if` so the output omits the key entirely.
    #[must_use]
    pub const fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
}

impl<T: Serialize> Serialize for DoubleOption<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unset | Self::Null => s.serialize_none(),
            Self::Set(v) => v.serialize(s),
        }
    }
}

fn deserialize_double_optional<'de, D, T>(de: D) -> Result<DoubleOption<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt: Option<T> = Option::deserialize(de)?;
    Ok(match opt {
        None => DoubleOption::Null,
        Some(v) => DoubleOption::Set(v),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_patch_validates() {
        DocxThemePatch::default().validate().unwrap();
    }

    #[test]
    fn out_of_range_body_font_size_is_rejected() {
        let patch = DocxThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(500),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = patch.validate().unwrap_err();
        match err {
            DocxThemeError::OutOfRange { field, .. } => {
                assert_eq!(field, "theme.typography.body_font_size_pt");
            }
            _ => panic!("expected OutOfRange"),
        }
    }

    #[test]
    fn invalid_hex_is_rejected_with_dotted_path() {
        let patch = DocxThemePatch {
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    hex: Some("not-a-color".to_string()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = patch.validate().unwrap_err();
        match err {
            DocxThemeError::InvalidColor { field, .. } => {
                assert_eq!(field, "theme.palette.heading.hex");
            }
            _ => panic!("expected InvalidColor"),
        }
    }

    #[test]
    fn hex_with_hash_is_accepted() {
        let patch = DocxThemePatch {
            palette: Some(PalettePatch {
                text: Some(RgbPatch {
                    hex: Some("#3B82F6".to_string()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        patch.validate().unwrap();
    }

    #[test]
    fn unknown_font_rejected() {
        let patch = DocxThemePatch {
            fonts: Some(FontsPatch {
                body: Some("Comic Sans MS".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = patch.validate().unwrap_err();
        assert!(matches!(err, DocxThemeError::UnknownFont { .. }));
    }

    #[test]
    fn allowed_font_accepted_case_insensitively() {
        let patch = DocxThemePatch {
            fonts: Some(FontsPatch {
                heading: Some("calibri".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        patch.validate().unwrap();
    }

    #[test]
    fn unknown_top_level_field_is_rejected_by_serde() {
        let json = serde_json::json!({ "typograhy": { "body_font_size_pt": 12 } });
        let r: Result<DocxThemePatch, _> = serde_json::from_value(json);
        assert!(r.is_err());
    }

    #[test]
    fn double_option_distinguishes_null_from_absent() {
        let absent: DecorationsPatch = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(matches!(absent.header, DoubleOption::Unset));

        let explicit_null: DecorationsPatch =
            serde_json::from_value(serde_json::json!({ "header": null })).unwrap();
        assert!(matches!(explicit_null.header, DoubleOption::Null));

        let set: DecorationsPatch =
            serde_json::from_value(serde_json::json!({ "header": "hello" })).unwrap();
        assert!(matches!(set.header, DoubleOption::Set(_)));
    }

    #[test]
    fn non_finite_line_spacing_rejected() {
        let patch = DocxThemePatch {
            typography: Some(TypographyPatch {
                line_spacing: Some(f32::INFINITY),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = patch.validate().unwrap_err();
        assert!(matches!(err, DocxThemeError::NotFinite { .. }));
    }
}
