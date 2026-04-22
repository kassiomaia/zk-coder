//! Partial [`Theme`] overrides supplied by the LLM at `pdf_start`.
//!
//! A [`ThemePatch`] mirrors [`Theme`] but every leaf field is optional.
//! The tool layer deserialises the JSON the LLM sends, [`ThemePatch::validate`]
//! enforces safe ranges, then [`Theme::apply_patch`] merges the patch over
//! a base theme: `Some` replaces, `None` leaves the base untouched.
//!
//! Nested structs are also optional, so an LLM can say "just recolor the
//! headings" without spelling out the rest of the palette.
//!
//! The JSON shape matches `resources/pdf/themes/report.toml` exactly, so
//! the bundled report is a copy-pasteable worked example.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::theme::{Decorations, Fonts, PageDefaults, Palette, Rgb, Spacing, Theme, TypographyScale};

/// Allow-listed bundled font filenames.
///
/// The renderer resolves font names against
/// `src-tauri/resources/pdf/fonts/`; any name outside this list would
/// simply fail to load at runtime, so we reject it up front with a
/// descriptive error the LLM can act on.
const BUNDLED_FONTS: &[&str] = &[
    "LiberationSans-Regular.ttf",
    "LiberationSans-Bold.ttf",
    "LiberationSans-Italic.ttf",
    "LiberationMono-Regular.ttf",
    "DejaVuSans.ttf",
];

/// Validation failure for a [`ThemePatch`].
///
/// The `field` string is the dotted JSON path to the offending value so
/// the LLM can locate and correct its input on the next step.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ThemeError {
    /// Numeric value fell outside the allowed range.
    #[error("{field}: value {value} out of range [{min}, {max}]")]
    OutOfRange {
        /// Dotted JSON path of the invalid field.
        field: String,
        /// The value as a string (floats formatted with two decimals).
        value: String,
        /// Inclusive lower bound.
        min: String,
        /// Inclusive upper bound.
        max: String,
    },
    /// Font filename is not in the bundled allow-list.
    #[error("{field}: font '{value}' is not bundled; expected one of {allowed}")]
    UnknownFont {
        /// Dotted JSON path.
        field: String,
        /// The offending font name.
        value: String,
        /// Comma-separated list of accepted names.
        allowed: String,
    },
    /// Numeric value is NaN or infinite.
    #[error("{field}: value must be a finite number")]
    NotFinite {
        /// Dotted JSON path.
        field: String,
    },
}

/// Optional override for [`Rgb`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct RgbPatch {
    /// Red channel.
    #[serde(default)]
    pub r: Option<f32>,
    /// Green channel.
    #[serde(default)]
    pub g: Option<f32>,
    /// Blue channel.
    #[serde(default)]
    pub b: Option<f32>,
}

/// Optional override for [`TypographyScale`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct TypographyPatch {
    /// Body font size, in points. Range `[6.0, 72.0]`.
    #[serde(default)]
    pub body_font_size_pt: Option<f32>,
    /// Heading font size, in points. Range `[8.0, 96.0]`.
    #[serde(default)]
    pub heading_font_size_pt: Option<f32>,
    /// Line leading multiplier. Range `[1.0, 2.5]`.
    #[serde(default)]
    pub line_leading: Option<f32>,
}

/// Optional override for [`Palette`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PalettePatch {
    /// Body text color.
    #[serde(default)]
    pub text: Option<RgbPatch>,
    /// Heading text color.
    #[serde(default)]
    pub heading: Option<RgbPatch>,
    /// Background fill for code blocks.
    #[serde(default)]
    pub code_bg: Option<RgbPatch>,
    /// Foreground for code blocks.
    #[serde(default)]
    pub code_text: Option<RgbPatch>,
    /// Background for callout boxes.
    #[serde(default)]
    pub callout_bg: Option<RgbPatch>,
    /// Border / accent for callout boxes.
    #[serde(default)]
    pub callout_border: Option<RgbPatch>,
    /// Background for table header rows.
    #[serde(default)]
    pub table_header_bg: Option<RgbPatch>,
    /// Table gridline color.
    #[serde(default)]
    pub table_border: Option<RgbPatch>,
}

/// Optional override for [`Spacing`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct SpacingPatch {
    /// Vertical gap multiplier. Range `[0.0, 5.0]`.
    #[serde(default)]
    pub block_gap: Option<f32>,
    /// Horizontal list indent in mm. Range `[0.0, 40.0]`.
    #[serde(default)]
    pub list_indent_mm: Option<f32>,
    /// Code/callout internal padding in mm. Range `[0.0, 20.0]`.
    #[serde(default)]
    pub block_padding_mm: Option<f32>,
    /// Table cell padding in mm. Range `[0.0, 20.0]`.
    #[serde(default)]
    pub table_cell_padding_mm: Option<f32>,
}

/// Optional override for [`PageDefaults`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PageDefaultsPatch {
    /// Uniform page margin in mm. Range `[5.0, 80.0]`.
    #[serde(default)]
    pub margin_mm: Option<f32>,
}

/// Optional override for [`Decorations`].
///
/// `header` and `footer` accept any string or `null`. Passing `null`
/// explicitly clears the corresponding strip; omitting the field leaves
/// the base theme's template in place.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct DecorationsPatch {
    /// Header template. Wrap the string in explicit `null` to drop it.
    #[serde(
        default,
        deserialize_with = "deserialize_double_optional",
        skip_serializing_if = "DoubleOption::is_unset"
    )]
    pub header: DoubleOption<String>,
    /// Footer template. Same semantics as `header`.
    #[serde(
        default,
        deserialize_with = "deserialize_double_optional",
        skip_serializing_if = "DoubleOption::is_unset"
    )]
    pub footer: DoubleOption<String>,
    /// Skip header/footer on the first page.
    #[serde(default)]
    pub suppress_on_first_page: Option<bool>,
    /// Font size for header/footer text. Range `[6.0, 20.0]`.
    #[serde(default)]
    pub font_size_pt: Option<f32>,
    /// Distance from page edge to baseline, in mm. Range `[2.0, 30.0]`.
    #[serde(default)]
    pub margin_mm: Option<f32>,
}

/// Optional override for [`Fonts`].
///
/// Each filename must be a member of `BUNDLED_FONTS` — callers cannot
/// upload custom fonts in this surface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontsPatch {
    /// Regular weight.
    #[serde(default)]
    pub regular: Option<String>,
    /// Bold weight.
    #[serde(default)]
    pub bold: Option<String>,
    /// Italic / oblique weight.
    #[serde(default)]
    pub italic: Option<String>,
    /// Monospace weight.
    #[serde(default)]
    pub mono: Option<String>,
    /// Fallback chain (each entry must be bundled).
    #[serde(default)]
    pub fallbacks: Option<Vec<String>>,
}

/// Full [`Theme`] patch — the root LLM-facing shape.
///
/// `deny_unknown_fields` is applied top-level and on every nested patch
/// so typos in LLM-supplied JSON — e.g. `heading_size_pt` instead of
/// `heading_font_size_pt` — surface as a clear
/// `ToolError::invalid_args` instead of being silently ignored while
/// `theme_applied: true` lies about the outcome.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct ThemePatch {
    /// Font sizes and leading.
    #[serde(default)]
    pub typography: Option<TypographyPatch>,
    /// Color palette.
    #[serde(default)]
    pub palette: Option<PalettePatch>,
    /// Block spacing.
    #[serde(default)]
    pub spacing: Option<SpacingPatch>,
    /// Page-level defaults.
    #[serde(default)]
    pub page_defaults: Option<PageDefaultsPatch>,
    /// Header / footer templates and styling.
    #[serde(default)]
    pub decorations: Option<DecorationsPatch>,
    /// Font family filenames.
    #[serde(default)]
    pub fonts: Option<FontsPatch>,
}

impl ThemePatch {
    /// Returns `Ok(())` if every supplied field is in range; returns the
    /// first [`ThemeError`] otherwise.
    ///
    /// # Errors
    /// Returns a [`ThemeError`] describing the offending JSON path.
    pub fn validate(&self) -> Result<(), ThemeError> {
        if let Some(t) = self.typography.as_ref() {
            check_range("theme.typography.body_font_size_pt", t.body_font_size_pt, 6.0, 72.0)?;
            check_range("theme.typography.heading_font_size_pt", t.heading_font_size_pt, 8.0, 96.0)?;
            check_range("theme.typography.line_leading", t.line_leading, 1.0, 2.5)?;
        }
        if let Some(p) = self.palette.as_ref() {
            check_rgb("theme.palette.text", p.text.as_ref())?;
            check_rgb("theme.palette.heading", p.heading.as_ref())?;
            check_rgb("theme.palette.code_bg", p.code_bg.as_ref())?;
            check_rgb("theme.palette.code_text", p.code_text.as_ref())?;
            check_rgb("theme.palette.callout_bg", p.callout_bg.as_ref())?;
            check_rgb("theme.palette.callout_border", p.callout_border.as_ref())?;
            check_rgb("theme.palette.table_header_bg", p.table_header_bg.as_ref())?;
            check_rgb("theme.palette.table_border", p.table_border.as_ref())?;
        }
        if let Some(s) = self.spacing.as_ref() {
            check_range("theme.spacing.block_gap", s.block_gap, 0.0, 5.0)?;
            check_range("theme.spacing.list_indent_mm", s.list_indent_mm, 0.0, 40.0)?;
            check_range("theme.spacing.block_padding_mm", s.block_padding_mm, 0.0, 20.0)?;
            check_range("theme.spacing.table_cell_padding_mm", s.table_cell_padding_mm, 0.0, 20.0)?;
        }
        if let Some(p) = self.page_defaults.as_ref() {
            check_range("theme.page_defaults.margin_mm", p.margin_mm, 5.0, 80.0)?;
        }
        if let Some(d) = self.decorations.as_ref() {
            check_range("theme.decorations.font_size_pt", d.font_size_pt, 6.0, 20.0)?;
            check_range("theme.decorations.margin_mm", d.margin_mm, 2.0, 30.0)?;
        }
        if let Some(f) = self.fonts.as_ref() {
            check_font("theme.fonts.regular", f.regular.as_deref())?;
            check_font("theme.fonts.bold", f.bold.as_deref())?;
            check_font("theme.fonts.italic", f.italic.as_deref())?;
            check_font("theme.fonts.mono", f.mono.as_deref())?;
            if let Some(fallbacks) = f.fallbacks.as_ref() {
                for (i, name) in fallbacks.iter().enumerate() {
                    check_font(&format!("theme.fonts.fallbacks[{i}]"), Some(name.as_str()))?;
                }
            }
        }
        Ok(())
    }
}

impl Theme {
    /// Merges `patch` onto `self` in place. `Some` replaces, `None`
    /// leaves the corresponding field untouched.
    pub fn apply_patch(&mut self, patch: &ThemePatch) {
        if let Some(t) = patch.typography.as_ref() {
            apply_typography(&mut self.typography, t);
        }
        if let Some(p) = patch.palette.as_ref() {
            apply_palette(&mut self.palette, p);
        }
        if let Some(s) = patch.spacing.as_ref() {
            apply_spacing(&mut self.spacing, s);
        }
        if let Some(p) = patch.page_defaults.as_ref() {
            if let Some(v) = p.margin_mm {
                self.page_defaults = PageDefaults { margin_mm: v };
            }
        }
        if let Some(d) = patch.decorations.as_ref() {
            apply_decorations(&mut self.decorations, d);
        }
        if let Some(f) = patch.fonts.as_ref() {
            apply_fonts(&mut self.fonts, f);
        }
    }

    /// Convenience: clone and apply. Useful at the render seam where we
    /// start from a base (bundled report) and overlay the draft's patch.
    #[must_use]
    pub fn with_patch(mut self, patch: &ThemePatch) -> Self {
        self.apply_patch(patch);
        self
    }
}

fn apply_typography(base: &mut TypographyScale, patch: &TypographyPatch) {
    if let Some(v) = patch.body_font_size_pt {
        base.body_font_size_pt = v;
    }
    if let Some(v) = patch.heading_font_size_pt {
        base.heading_font_size_pt = v;
    }
    if let Some(v) = patch.line_leading {
        base.line_leading = v;
    }
}

fn apply_palette(base: &mut Palette, patch: &PalettePatch) {
    apply_rgb(&mut base.text, patch.text.as_ref());
    apply_rgb(&mut base.heading, patch.heading.as_ref());
    apply_rgb(&mut base.code_bg, patch.code_bg.as_ref());
    apply_rgb(&mut base.code_text, patch.code_text.as_ref());
    apply_rgb(&mut base.callout_bg, patch.callout_bg.as_ref());
    apply_rgb(&mut base.callout_border, patch.callout_border.as_ref());
    apply_rgb(&mut base.table_header_bg, patch.table_header_bg.as_ref());
    apply_rgb(&mut base.table_border, patch.table_border.as_ref());
}

fn apply_rgb(base: &mut Rgb, patch: Option<&RgbPatch>) {
    if let Some(p) = patch {
        if let Some(v) = p.r {
            base.r = v;
        }
        if let Some(v) = p.g {
            base.g = v;
        }
        if let Some(v) = p.b {
            base.b = v;
        }
    }
}

fn apply_spacing(base: &mut Spacing, patch: &SpacingPatch) {
    if let Some(v) = patch.block_gap {
        base.block_gap = v;
    }
    if let Some(v) = patch.list_indent_mm {
        base.list_indent_mm = v;
    }
    if let Some(v) = patch.block_padding_mm {
        base.block_padding_mm = v;
    }
    if let Some(v) = patch.table_cell_padding_mm {
        base.table_cell_padding_mm = v;
    }
}

fn apply_decorations(base: &mut Decorations, patch: &DecorationsPatch) {
    match &patch.header {
        DoubleOption::Unset => {}
        DoubleOption::Null => base.header = None,
        DoubleOption::Set(s) => base.header = Some(s.clone()),
    }
    match &patch.footer {
        DoubleOption::Unset => {}
        DoubleOption::Null => base.footer = None,
        DoubleOption::Set(s) => base.footer = Some(s.clone()),
    }
    if let Some(v) = patch.suppress_on_first_page {
        base.suppress_on_first_page = v;
    }
    if let Some(v) = patch.font_size_pt {
        base.font_size_pt = v;
    }
    if let Some(v) = patch.margin_mm {
        base.margin_mm = v;
    }
}

fn apply_fonts(base: &mut Fonts, patch: &FontsPatch) {
    if let Some(v) = patch.regular.as_ref() {
        base.regular = v.clone();
    }
    if let Some(v) = patch.bold.as_ref() {
        base.bold = v.clone();
    }
    if let Some(v) = patch.italic.as_ref() {
        base.italic = v.clone();
    }
    if let Some(v) = patch.mono.as_ref() {
        base.mono = v.clone();
    }
    if let Some(v) = patch.fallbacks.as_ref() {
        base.fallbacks = v.clone();
    }
}

fn check_range(field: &str, value: Option<f32>, min: f32, max: f32) -> Result<(), ThemeError> {
    let Some(v) = value else { return Ok(()) };
    if !v.is_finite() {
        return Err(ThemeError::NotFinite {
            field: field.to_string(),
        });
    }
    if v < min || v > max {
        return Err(ThemeError::OutOfRange {
            field: field.to_string(),
            value: format!("{v:.2}"),
            min: format!("{min:.2}"),
            max: format!("{max:.2}"),
        });
    }
    Ok(())
}

fn check_rgb(prefix: &str, rgb: Option<&RgbPatch>) -> Result<(), ThemeError> {
    let Some(rgb) = rgb else { return Ok(()) };
    check_range(&format!("{prefix}.r"), rgb.r, 0.0, 1.0)?;
    check_range(&format!("{prefix}.g"), rgb.g, 0.0, 1.0)?;
    check_range(&format!("{prefix}.b"), rgb.b, 0.0, 1.0)?;
    Ok(())
}

fn check_font(field: &str, name: Option<&str>) -> Result<(), ThemeError> {
    let Some(name) = name else { return Ok(()) };
    if BUNDLED_FONTS.iter().any(|b| *b == name) {
        Ok(())
    } else {
        Err(ThemeError::UnknownFont {
            field: field.to_string(),
            value: name.to_string(),
            allowed: BUNDLED_FONTS.join(", "),
        })
    }
}

/// Three-state option: JSON `null` clears; missing key leaves the base
/// alone. Serde normally collapses both to `Option::None`, so we keep
/// our own marker.
///
/// Container structs must pair the field with
/// `#[serde(skip_serializing_if = "DoubleOption::is_unset")]` so that
/// `Unset` is omitted from JSON entirely — otherwise a serialize /
/// deserialize round-trip would coerce `Unset` → JSON `null` → `Null`
/// and silently flip the semantics from "leave the base alone" to
/// "explicitly clear".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DoubleOption<T> {
    /// Key was absent from the JSON object.
    #[default]
    Unset,
    /// Key was present with value `null`.
    Null,
    /// Key was present with a real value.
    Set(T),
}

impl<T> DoubleOption<T> {
    /// Returns `true` when this is the `Unset` variant. Used as
    /// `skip_serializing_if` on struct fields so JSON output omits the
    /// key entirely rather than emitting `null`.
    #[must_use]
    pub const fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
}

impl<T: Serialize> Serialize for DoubleOption<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // `Unset` is normally skipped via `skip_serializing_if` on the
        // container field; if it somehow reaches this impl (e.g. a
        // caller serialising a bare `DoubleOption`) we still emit
        // `null` rather than panic — the round-trip guarantee only
        // holds for struct fields that apply the skip attribute.
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
    fn empty_patch_is_noop() {
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&ThemePatch::default());
        assert_eq!(base, merged);
    }

    #[test]
    fn typography_patch_overrides_only_supplied_fields() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(14.0),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&patch);
        assert!((merged.typography.body_font_size_pt - 14.0).abs() < f32::EPSILON);
        assert!(
            (merged.typography.heading_font_size_pt - base.typography.heading_font_size_pt).abs()
                < f32::EPSILON
        );
        assert!((merged.typography.line_leading - base.typography.line_leading).abs() < f32::EPSILON);
    }

    #[test]
    fn palette_patch_overrides_only_supplied_channels() {
        let patch = ThemePatch {
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    r: Some(0.1),
                    g: Some(0.2),
                    b: Some(0.8),
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        };
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&patch);
        assert_eq!(merged.palette.heading, Rgb { r: 0.1, g: 0.2, b: 0.8 });
        assert_eq!(merged.palette.text, base.palette.text);
    }

    #[test]
    fn partial_rgb_patch_only_touches_named_channels() {
        let patch = ThemePatch {
            palette: Some(PalettePatch {
                text: Some(RgbPatch {
                    r: Some(0.3),
                    ..RgbPatch::default()
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        };
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&patch);
        assert!((merged.palette.text.r - 0.3).abs() < f32::EPSILON);
        assert_eq!(merged.palette.text.g, base.palette.text.g);
        assert_eq!(merged.palette.text.b, base.palette.text.b);
    }

    #[test]
    fn decorations_null_header_clears_it() {
        let json = serde_json::json!({
            "decorations": {"header": null}
        });
        let patch: ThemePatch = serde_json::from_value(json).unwrap();
        let merged = Theme::default_report().with_patch(&patch);
        assert!(merged.decorations.header.is_none());
    }

    #[test]
    fn decorations_missing_header_keeps_it() {
        let json = serde_json::json!({
            "decorations": {"margin_mm": 15.0}
        });
        let patch: ThemePatch = serde_json::from_value(json).unwrap();
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&patch);
        assert_eq!(merged.decorations.header, base.decorations.header);
        assert!((merged.decorations.margin_mm - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn decorations_patch_round_trips_unset_header_as_absent_not_null() {
        // Build a patch that supplies `margin_mm` but leaves `header`
        // as `Unset`. Serialising + deserialising must not coerce the
        // unset header into a `Null` (which would explicitly clear it).
        let json = serde_json::json!({
            "decorations": {"margin_mm": 15.0}
        });
        let patch: ThemePatch = serde_json::from_value(json).unwrap();
        let serialised = serde_json::to_value(&patch).unwrap();
        // The round-tripped JSON must NOT contain a `header` key at all.
        let decorations = serialised
            .get("decorations")
            .expect("decorations present");
        assert!(
            decorations.get("header").is_none(),
            "Unset header must be omitted, got: {decorations}"
        );
        assert!(
            decorations.get("footer").is_none(),
            "Unset footer must be omitted, got: {decorations}"
        );

        // Round-trip through deserialisation to confirm semantics hold.
        let back: ThemePatch = serde_json::from_value(serialised).unwrap();
        let base_header = Theme::default_report().decorations.header.clone();
        let merged = Theme::default_report().with_patch(&back);
        assert_eq!(
            merged.decorations.header, base_header,
            "round-tripped patch must not have cleared the header"
        );
    }

    #[test]
    fn decorations_patch_preserves_explicit_null_through_round_trip() {
        let json = serde_json::json!({
            "decorations": {"header": null}
        });
        let patch: ThemePatch = serde_json::from_value(json).unwrap();
        let serialised = serde_json::to_value(&patch).unwrap();
        let decorations = serialised.get("decorations").expect("present");
        assert_eq!(
            decorations.get("header"),
            Some(&serde_json::Value::Null),
            "Null header must serialise as explicit JSON null"
        );

        let back: ThemePatch = serde_json::from_value(serialised).unwrap();
        let merged = Theme::default_report().with_patch(&back);
        assert!(
            merged.decorations.header.is_none(),
            "explicit-null header must still clear after round-trip"
        );
    }

    #[test]
    fn decorations_footer_replaces_string() {
        let json = serde_json::json!({
            "decorations": {"footer": "Confidential — {page}"}
        });
        let patch: ThemePatch = serde_json::from_value(json).unwrap();
        let merged = Theme::default_report().with_patch(&patch);
        assert_eq!(
            merged.decorations.footer.as_deref(),
            Some("Confidential — {page}")
        );
    }

    #[test]
    fn fonts_patch_requires_bundled_names() {
        let patch = ThemePatch {
            fonts: Some(FontsPatch {
                regular: Some("LiberationMono-Regular.ttf".to_string()),
                ..FontsPatch::default()
            }),
            ..ThemePatch::default()
        };
        patch.validate().expect("bundled font is accepted");
    }

    #[test]
    fn theme_patch_rejects_unknown_top_level_key() {
        let json = serde_json::json!({
            "typographyy": {"body_font_size_pt": 13}
        });
        let err = serde_json::from_value::<ThemePatch>(json)
            .expect_err("typo in section name must be rejected");
        assert!(
            err.to_string().contains("typographyy"),
            "error should surface the unknown key, got: {err}"
        );
    }

    #[test]
    fn typography_patch_rejects_unknown_nested_key() {
        let json = serde_json::json!({
            "typography": {"heading_size_pt": 16}
        });
        let err = serde_json::from_value::<ThemePatch>(json)
            .expect_err("typo in nested field must be rejected");
        assert!(
            err.to_string().contains("heading_size_pt"),
            "error should surface the misspelled field, got: {err}"
        );
    }

    #[test]
    fn fonts_patch_rejects_unknown_filename() {
        let patch = ThemePatch {
            fonts: Some(FontsPatch {
                regular: Some("ComicSans.ttf".to_string()),
                ..FontsPatch::default()
            }),
            ..ThemePatch::default()
        };
        let err = patch.validate().expect_err("unknown font is rejected");
        match err {
            ThemeError::UnknownFont { field, value, .. } => {
                assert_eq!(field, "theme.fonts.regular");
                assert_eq!(value, "ComicSans.ttf");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_rgb_above_one() {
        let patch = ThemePatch {
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    r: Some(1.5),
                    ..RgbPatch::default()
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        };
        let err = patch.validate().expect_err("rgb > 1 rejected");
        match err {
            ThemeError::OutOfRange { field, .. } => {
                assert_eq!(field, "theme.palette.heading.r");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_body_font_too_small() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(2.0),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        let err = patch.validate().expect_err("font < 6 rejected");
        match err {
            ThemeError::OutOfRange { field, .. } => {
                assert_eq!(field, "theme.typography.body_font_size_pt");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_heading_font_too_large() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                heading_font_size_pt: Some(500.0),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        let err = patch.validate().expect_err("heading > 96 rejected");
        assert!(matches!(err, ThemeError::OutOfRange { .. }));
    }

    #[test]
    fn validate_rejects_line_leading_below_one() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                line_leading: Some(0.5),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        assert!(matches!(
            patch.validate(),
            Err(ThemeError::OutOfRange { .. })
        ));
    }

    #[test]
    fn validate_rejects_margin_too_small() {
        let patch = ThemePatch {
            page_defaults: Some(PageDefaultsPatch {
                margin_mm: Some(1.0),
            }),
            ..ThemePatch::default()
        };
        assert!(matches!(
            patch.validate(),
            Err(ThemeError::OutOfRange { .. })
        ));
    }

    #[test]
    fn validate_rejects_decorations_font_out_of_range() {
        let patch = ThemePatch {
            decorations: Some(DecorationsPatch {
                font_size_pt: Some(30.0),
                ..DecorationsPatch::default()
            }),
            ..ThemePatch::default()
        };
        assert!(matches!(
            patch.validate(),
            Err(ThemeError::OutOfRange { .. })
        ));
    }

    #[test]
    fn validate_rejects_nan() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(f32::NAN),
                ..TypographyPatch::default()
            }),
            ..ThemePatch::default()
        };
        assert!(matches!(patch.validate(), Err(ThemeError::NotFinite { .. })));
    }

    #[test]
    fn json_roundtrip_preserves_shape() {
        let patch = ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(12.5),
                ..TypographyPatch::default()
            }),
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    r: Some(0.1),
                    g: Some(0.2),
                    b: Some(0.8),
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        };
        let json = serde_json::to_string(&patch).unwrap();
        let back: ThemePatch = serde_json::from_str(&json).unwrap();
        assert_eq!(patch, back);
    }

    #[test]
    fn spacing_patch_merges_independent_fields() {
        let patch = ThemePatch {
            spacing: Some(SpacingPatch {
                list_indent_mm: Some(12.0),
                ..SpacingPatch::default()
            }),
            ..ThemePatch::default()
        };
        let base = Theme::default_report();
        let merged = base.clone().with_patch(&patch);
        assert!((merged.spacing.list_indent_mm - 12.0).abs() < f32::EPSILON);
        assert!(
            (merged.spacing.block_gap - base.spacing.block_gap).abs() < f32::EPSILON,
            "untouched field must survive"
        );
    }
}
