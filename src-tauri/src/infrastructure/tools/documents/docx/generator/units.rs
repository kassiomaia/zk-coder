//! Unit conversions between the theme's human-friendly units (mm, pt)
//! and the OOXML units `docx_rust` expects on the wire.
//!
//! * `twip`   = 1/20 of a point; used for `w:pgSz`, `w:pgMar`, `w:spacing`.
//! * `half_pt` = 1/2 of a point; used for `w:sz` (font size).
//! * `emu`    = English Metric Unit, 914_400 per inch; used for
//!              image extent (`wp:extent`).

const TWIPS_PER_POINT: f32 = 20.0;
const POINTS_PER_INCH: f32 = 72.0;
const MM_PER_INCH: f32 = 25.4;
const EMU_PER_INCH: f32 = 914_400.0;

/// Millimetres to twentieths of a point (rounded).
#[must_use]
pub fn mm_to_twips(mm: f32) -> i32 {
    let inches = mm / MM_PER_INCH;
    (inches * POINTS_PER_INCH * TWIPS_PER_POINT).round() as i32
}

/// Points to twips.
#[must_use]
pub const fn pt_to_twips(pt: u16) -> i32 {
    (pt as i32) * 20
}

/// Points to half-points (the unit `<w:sz>` accepts).
#[must_use]
pub const fn pt_to_half_pt(pt: u16) -> i32 {
    (pt as i32) * 2
}

/// Millimetres to English Metric Units (rounded).
#[must_use]
pub fn mm_to_emu(mm: f32) -> u64 {
    let inches = mm / MM_PER_INCH;
    (inches * EMU_PER_INCH).round() as u64
}

/// Line-spacing multiplier to the `w:line` integer, paired with
/// `lineRule="auto"` on the wire. Word treats 240 as single spacing,
/// 360 as 1.5, 480 as double.
#[must_use]
pub fn line_spacing_to_line_value(multiplier: f32) -> i32 {
    let base = 240.0 * multiplier;
    if !base.is_finite() || base < 60.0 {
        240
    } else {
        base.round() as i32
    }
}

/// Heading size for a given outline level (1-9). Level 1 uses the
/// base heading size; deeper levels are scaled by `heading_scale`
/// multiplied with the level depth.
#[must_use]
pub fn heading_size_pt(base_pt: u16, scale: f32, level: u8) -> u16 {
    let clamped_level = level.max(1).min(9);
    let steps = f32::from(clamped_level - 1);
    let factor = scale.powf(steps);
    let size = f32::from(base_pt) * factor;
    size.round().clamp(6.0, 96.0) as u16
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a4_margins_convert_to_standard_twips() {
        assert_eq!(mm_to_twips(25.4), 1440);
        assert_eq!(mm_to_twips(10.0), 567);
    }

    #[test]
    fn points_to_twips_matches_twenty_multiplier() {
        assert_eq!(pt_to_twips(12), 240);
        assert_eq!(pt_to_twips(0), 0);
    }

    #[test]
    fn half_points_are_double_the_point_value() {
        assert_eq!(pt_to_half_pt(11), 22);
    }

    #[test]
    fn mm_to_emu_matches_standard_inch_conversion() {
        assert_eq!(mm_to_emu(25.4), 914_400);
    }

    #[test]
    fn line_spacing_converts_to_twenty_fourths() {
        assert_eq!(line_spacing_to_line_value(1.0), 240);
        assert_eq!(line_spacing_to_line_value(1.5), 360);
        assert_eq!(line_spacing_to_line_value(2.0), 480);
    }

    #[test]
    fn line_spacing_handles_edge_values() {
        assert_eq!(line_spacing_to_line_value(f32::NAN), 240);
        assert_eq!(line_spacing_to_line_value(0.1), 240);
    }

    #[test]
    fn heading_sizes_scale_down() {
        let sizes: Vec<u16> = (1..=5).map(|l| heading_size_pt(24, 0.82, l)).collect();
        assert!(sizes.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(sizes[0], 24);
    }
}
