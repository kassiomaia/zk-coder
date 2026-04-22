//! Value objects describing the fully-resolved styling of a DOCX document.
//!
//! A [`DocxTheme`] is the bundled default plus any LLM-supplied
//! [`DocxThemePatch`] merged on top. All leaf fields are required here;
//! optionality lives in the patch type.
//!
//! Units are chosen to match what the generator writes into OOXML:
//! font sizes in whole points, margins in millimetres (converted to
//! twips at render time), colors as `RRGGBB` hex without `#`.

use serde::{Deserialize, Serialize};

use super::theme_patch::{
    DecorationsPatch, DocxThemePatch, DoubleOption, FontsPatch, PagePatch, PalettePatch, RgbPatch,
    SpacingPatch, TypographyPatch,
};

/// Font sizes and line spacing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Typography {
    /// Body font size, in whole points.
    pub body_font_size_pt: u16,
    /// Base heading font size, in whole points. Level 1 uses this
    /// directly; deeper levels scale down.
    pub heading_font_size_pt: u16,
    /// Line spacing multiplier applied to every paragraph.
    pub line_spacing: f32,
    /// Multiplicative step between heading levels (1 = no scaling).
    pub heading_scale: f32,
}

/// Color in `RRGGBB` hex form (no leading `#`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    /// Lower-cased `RRGGBB` hex string.
    pub hex: String,
}

impl Rgb {
    /// Construct from a hex string, upper/lower case and with or
    /// without a leading `#` both accepted. Does not validate;
    /// validation happens at the patch boundary.
    pub fn new<S: Into<String>>(hex: S) -> Self {
        let h = hex.into();
        let trimmed = h.trim_start_matches('#').to_ascii_lowercase();
        Rgb { hex: trimmed }
    }
}

/// Named palette slots mapped to document roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    /// Body text color.
    pub text: Rgb,
    /// Heading color.
    pub heading: Rgb,
    /// Hyperlink color.
    pub link: Rgb,
    /// Inline / block code foreground.
    pub code_text: Rgb,
    /// Inline / block code background shading.
    pub code_bg: Rgb,
    /// Table header row background.
    pub table_header_bg: Rgb,
    /// Table gridline color.
    pub table_border: Rgb,
}

/// Vertical and horizontal rhythm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Spacing {
    /// Space above each paragraph, in points.
    pub paragraph_space_before_pt: u16,
    /// Space below each paragraph, in points.
    pub paragraph_space_after_pt: u16,
    /// Left indent added per list level, in points.
    pub list_indent_pt: u16,
    /// Padding inside table cells, in points.
    pub table_cell_padding_pt: u16,
}

/// Supported paper sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSize {
    /// ISO A4 (210 × 297 mm).
    A4,
    /// US Letter (8.5 × 11 in).
    Letter,
    /// US Legal (8.5 × 14 in).
    Legal,
}

impl PaperSize {
    /// Page width × height in twentieths of a point (twips, the unit
    /// `docx-rust` writes into `<w:pgSz>`).
    #[must_use]
    pub const fn twips_portrait(self) -> (i32, i32) {
        match self {
            PaperSize::A4 => (11906, 16838),
            PaperSize::Letter => (12240, 15840),
            PaperSize::Legal => (12240, 20160),
        }
    }
}

/// Page-level defaults (paper, orientation, margins).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageDefaults {
    /// Paper size.
    pub paper: PaperSize,
    /// `true` for landscape; swaps width/height at render time.
    pub landscape: bool,
    /// Top margin, in millimetres.
    pub margin_top_mm: f32,
    /// Bottom margin, in millimetres.
    pub margin_bottom_mm: f32,
    /// Left margin, in millimetres.
    pub margin_left_mm: f32,
    /// Right margin, in millimetres.
    pub margin_right_mm: f32,
}

/// Header / footer templates and layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decorations {
    /// Header template. `None` renders no header. Tokens `{title}`,
    /// `{page}`, `{total}` are substituted at generate time.
    pub header: Option<String>,
    /// Footer template with the same token set as `header`.
    pub footer: Option<String>,
    /// When `true`, the first page uses an empty header/footer pair.
    pub different_first_page: bool,
}

/// Word-facing font family names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fonts {
    /// Body text font family name (e.g. `Calibri`).
    pub body: String,
    /// Heading font family name.
    pub heading: String,
    /// Monospace font family name for code.
    pub mono: String,
}

/// Fully-resolved theme passed to the generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocxTheme {
    /// Font sizes and line spacing.
    pub typography: Typography,
    /// Color slots.
    pub palette: Palette,
    /// Paragraph and list spacing.
    pub spacing: Spacing,
    /// Page geometry.
    pub page: PageDefaults,
    /// Header / footer templates.
    pub decorations: Decorations,
    /// Font family names.
    pub fonts: Fonts,
}

impl DocxTheme {
    /// Bundled default theme. Balanced for a business report look.
    #[must_use]
    pub fn report_default() -> Self {
        DocxTheme {
            typography: Typography {
                body_font_size_pt: 11,
                heading_font_size_pt: 20,
                line_spacing: 1.15,
                heading_scale: 0.82,
            },
            palette: Palette {
                text: Rgb::new("1f2937"),
                heading: Rgb::new("111827"),
                link: Rgb::new("2563eb"),
                code_text: Rgb::new("1f2937"),
                code_bg: Rgb::new("f3f4f6"),
                table_header_bg: Rgb::new("e5e7eb"),
                table_border: Rgb::new("9ca3af"),
            },
            spacing: Spacing {
                paragraph_space_before_pt: 0,
                paragraph_space_after_pt: 8,
                list_indent_pt: 18,
                table_cell_padding_pt: 4,
            },
            page: PageDefaults {
                paper: PaperSize::A4,
                landscape: false,
                margin_top_mm: 25.0,
                margin_bottom_mm: 25.0,
                margin_left_mm: 25.0,
                margin_right_mm: 25.0,
            },
            decorations: Decorations {
                header: None,
                footer: None,
                different_first_page: false,
            },
            fonts: Fonts {
                body: "Calibri".to_string(),
                heading: "Calibri".to_string(),
                mono: "Consolas".to_string(),
            },
        }
    }

    /// Merge a patch onto `self` in place. `Some` leaves in the patch
    /// replaces the corresponding field; `None` leaves the base
    /// untouched.
    pub fn apply_patch(&mut self, patch: &DocxThemePatch) {
        if let Some(t) = patch.typography.as_ref() {
            apply_typography(&mut self.typography, t);
        }
        if let Some(p) = patch.palette.as_ref() {
            apply_palette(&mut self.palette, p);
        }
        if let Some(s) = patch.spacing.as_ref() {
            apply_spacing(&mut self.spacing, s);
        }
        if let Some(p) = patch.page.as_ref() {
            apply_page(&mut self.page, p);
        }
        if let Some(d) = patch.decorations.as_ref() {
            apply_decorations(&mut self.decorations, d);
        }
        if let Some(f) = patch.fonts.as_ref() {
            apply_fonts(&mut self.fonts, f);
        }
    }

    /// Clone-and-apply convenience used at the render seam.
    #[must_use]
    pub fn with_patch(mut self, patch: &DocxThemePatch) -> Self {
        self.apply_patch(patch);
        self
    }
}

fn apply_typography(base: &mut Typography, patch: &TypographyPatch) {
    if let Some(v) = patch.body_font_size_pt {
        base.body_font_size_pt = v;
    }
    if let Some(v) = patch.heading_font_size_pt {
        base.heading_font_size_pt = v;
    }
    if let Some(v) = patch.line_spacing {
        base.line_spacing = v;
    }
    if let Some(v) = patch.heading_scale {
        base.heading_scale = v;
    }
}

fn apply_palette(base: &mut Palette, patch: &PalettePatch) {
    apply_rgb(&mut base.text, patch.text.as_ref());
    apply_rgb(&mut base.heading, patch.heading.as_ref());
    apply_rgb(&mut base.link, patch.link.as_ref());
    apply_rgb(&mut base.code_text, patch.code_text.as_ref());
    apply_rgb(&mut base.code_bg, patch.code_bg.as_ref());
    apply_rgb(&mut base.table_header_bg, patch.table_header_bg.as_ref());
    apply_rgb(&mut base.table_border, patch.table_border.as_ref());
}

fn apply_rgb(base: &mut Rgb, patch: Option<&RgbPatch>) {
    if let Some(p) = patch {
        if let Some(hex) = p.hex.as_ref() {
            *base = Rgb::new(hex);
        }
    }
}

fn apply_spacing(base: &mut Spacing, patch: &SpacingPatch) {
    if let Some(v) = patch.paragraph_space_before_pt {
        base.paragraph_space_before_pt = v;
    }
    if let Some(v) = patch.paragraph_space_after_pt {
        base.paragraph_space_after_pt = v;
    }
    if let Some(v) = patch.list_indent_pt {
        base.list_indent_pt = v;
    }
    if let Some(v) = patch.table_cell_padding_pt {
        base.table_cell_padding_pt = v;
    }
}

fn apply_page(base: &mut PageDefaults, patch: &PagePatch) {
    if let Some(v) = patch.paper {
        base.paper = v;
    }
    if let Some(v) = patch.landscape {
        base.landscape = v;
    }
    if let Some(v) = patch.margin_top_mm {
        base.margin_top_mm = v;
    }
    if let Some(v) = patch.margin_bottom_mm {
        base.margin_bottom_mm = v;
    }
    if let Some(v) = patch.margin_left_mm {
        base.margin_left_mm = v;
    }
    if let Some(v) = patch.margin_right_mm {
        base.margin_right_mm = v;
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
    if let Some(v) = patch.different_first_page {
        base.different_first_page = v;
    }
}

fn apply_fonts(base: &mut Fonts, patch: &FontsPatch) {
    if let Some(v) = patch.body.as_ref() {
        base.body = v.clone();
    }
    if let Some(v) = patch.heading.as_ref() {
        base.heading = v.clone();
    }
    if let Some(v) = patch.mono.as_ref() {
        base.mono = v.clone();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn report_default_has_sane_values() {
        let t = DocxTheme::report_default();
        assert_eq!(t.typography.body_font_size_pt, 11);
        assert!(t.typography.line_spacing >= 1.0);
        assert!(t.page.margin_top_mm > 0.0);
        assert_eq!(t.page.paper, PaperSize::A4);
    }

    #[test]
    fn empty_patch_is_noop() {
        let base = DocxTheme::report_default();
        let merged = base.clone().with_patch(&DocxThemePatch::default());
        assert_eq!(base, merged);
    }

    #[test]
    fn partial_patch_replaces_only_specified_fields() {
        let mut base = DocxTheme::report_default();
        let patch = DocxThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(14),
                ..Default::default()
            }),
            ..Default::default()
        };
        base.apply_patch(&patch);
        assert_eq!(base.typography.body_font_size_pt, 14);
        assert_eq!(base.typography.heading_font_size_pt, 20);
    }

    #[test]
    fn rgb_new_normalises_hash_and_case() {
        assert_eq!(Rgb::new("#FFaa00").hex, "ffaa00");
        assert_eq!(Rgb::new("123ABC").hex, "123abc");
    }

    #[test]
    fn paper_size_twips_match_standard_dimensions() {
        assert_eq!(PaperSize::A4.twips_portrait(), (11906, 16838));
        assert_eq!(PaperSize::Letter.twips_portrait(), (12240, 15840));
    }
}
