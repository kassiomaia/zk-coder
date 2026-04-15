//! Font cache with primary + fallback chain for glyph lookups.
//!
//! The cache owns the raw bytes of every font the theme declares. Each
//! lookup (`resolve`) walks from the style-selected family through the
//! configured fallbacks and reports the first font that actually has a
//! glyph for the character. Callers use that information to route each
//! code point to the right PDF font in the output and to emit a
//! [`RenderWarning::MissingGlyph`] when nothing covers the code point.

use std::path::{Path, PathBuf};

use ttf_parser::{name_id, Face};

use crate::core::documents::pdf::block::InlineStyle;
use crate::core::documents::pdf::render::RenderError;
use crate::core::documents::pdf::theme::Fonts;

/// Logical slot a font fills in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSlot {
    /// Body / regular text.
    Regular,
    /// Bold weight.
    Bold,
    /// Italic / oblique weight.
    Italic,
    /// Monospaced (inline code).
    Mono,
    /// Fallback chain entry at the given zero-based index.
    Fallback(usize),
}

impl FontSlot {
    /// Stable string label used by tests and diagnostics. Never parsed
    /// by runtime code.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Regular => "regular".to_string(),
            Self::Bold => "bold".to_string(),
            Self::Italic => "italic".to_string(),
            Self::Mono => "mono".to_string(),
            Self::Fallback(i) => format!("fallback_{i}"),
        }
    }
}

/// A font loaded into memory together with its parsed family name.
#[derive(Debug)]
pub struct LoadedFont {
    slot: FontSlot,
    family: String,
    bytes: Vec<u8>,
}

impl LoadedFont {
    /// Builds a [`LoadedFont`] from raw bytes. Reads the family name from
    /// the font's `name` table.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] if `bytes` is not a valid TTF/OTF.
    pub fn from_bytes(slot: FontSlot, bytes: Vec<u8>) -> Result<Self, RenderError> {
        let family = {
            let face = Face::parse(&bytes, 0).map_err(|e| RenderError::Font(e.to_string()))?;
            read_family_name(&face).unwrap_or_else(|| "Unknown".to_string())
        };
        Ok(Self {
            slot,
            family,
            bytes,
        })
    }

    /// Returns the slot this font fills.
    #[must_use]
    pub const fn slot(&self) -> FontSlot {
        self.slot
    }

    /// Returns the family name declared by the font's `name` table.
    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Returns the raw font bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Reparses the face. Cheap; `ttf-parser` faces borrow the bytes.
    ///
    /// # Errors
    /// Returns [`RenderError::Font`] only if the stored bytes stop being
    /// a valid TTF/OTF — which never happens after construction.
    pub fn face(&self) -> Result<Face<'_>, RenderError> {
        Face::parse(&self.bytes, 0).map_err(|e| RenderError::Font(e.to_string()))
    }

    fn has_glyph(&self, c: char) -> bool {
        self.face()
            .ok()
            .and_then(|f| f.glyph_index(c))
            .is_some_and(|gid| gid.0 != 0)
    }
}

/// Loaded fonts grouped by slot plus an ordered fallback chain.
#[derive(Debug)]
pub struct FontCache {
    regular: LoadedFont,
    bold: LoadedFont,
    italic: LoadedFont,
    mono: LoadedFont,
    fallbacks: Vec<LoadedFont>,
}

impl FontCache {
    /// Builds a cache by reading every font file declared in `spec` from
    /// the `resources_root` directory.
    ///
    /// # Errors
    /// Returns [`RenderError::Io`] if any file cannot be read and
    /// [`RenderError::Font`] if any file fails to parse as a TTF/OTF.
    pub fn load(spec: &Fonts, resources_root: &Path) -> Result<Self, RenderError> {
        let regular = load_slot(FontSlot::Regular, resources_root, &spec.regular)?;
        let bold = load_slot(FontSlot::Bold, resources_root, &spec.bold)?;
        let italic = load_slot(FontSlot::Italic, resources_root, &spec.italic)?;
        let mono = load_slot(FontSlot::Mono, resources_root, &spec.mono)?;
        let mut fallbacks = Vec::with_capacity(spec.fallbacks.len());
        for (idx, name) in spec.fallbacks.iter().enumerate() {
            fallbacks.push(load_slot(FontSlot::Fallback(idx), resources_root, name)?);
        }
        Ok(Self {
            regular,
            bold,
            italic,
            mono,
            fallbacks,
        })
    }

    /// Builds a cache from already-loaded font payloads. Used by tests
    /// so they do not need to round-trip through the filesystem.
    #[must_use]
    pub const fn from_parts(
        regular: LoadedFont,
        bold: LoadedFont,
        italic: LoadedFont,
        mono: LoadedFont,
        fallbacks: Vec<LoadedFont>,
    ) -> Self {
        Self {
            regular,
            bold,
            italic,
            mono,
            fallbacks,
        }
    }

    /// Selects the style-driven primary font for an inline run.
    ///
    /// Resolution order:
    /// - `code` beats everything else → `mono`
    /// - otherwise `bold + italic` → `bold` (italic-bold is rare in free
    ///   fonts; M2 may add it as a separate slot)
    /// - `bold` alone → `bold`
    /// - `italic` alone → `italic`
    /// - plain → `regular`
    #[must_use]
    pub const fn primary_for(&self, style: &InlineStyle) -> &LoadedFont {
        if style.code {
            return &self.mono;
        }
        if style.bold {
            return &self.bold;
        }
        if style.italic {
            return &self.italic;
        }
        &self.regular
    }

    /// Returns every font loaded by this cache in a stable order.
    #[must_use]
    pub fn all(&self) -> Vec<&LoadedFont> {
        let mut out: Vec<&LoadedFont> = vec![&self.regular, &self.bold, &self.italic, &self.mono];
        for fb in &self.fallbacks {
            out.push(fb);
        }
        out
    }

    /// Number of fonts held in the cache (including fallbacks).
    #[must_use]
    pub fn len(&self) -> usize {
        4 + self.fallbacks.len()
    }

    /// Always `false` in practice — kept so `len` / `is_empty` stay paired
    /// for clippy.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Finds the first font that has a glyph for `c`, starting from the
    /// style-selected primary and walking the fallback chain.
    ///
    /// Returns `None` when no loaded font covers the code point; callers
    /// emit [`RenderWarning::MissingGlyph`] in that case.
    #[must_use]
    pub fn resolve(&self, c: char, style: &InlineStyle) -> Option<&LoadedFont> {
        let primary = self.primary_for(style);
        if primary.has_glyph(c) {
            return Some(primary);
        }
        // Walk the remaining non-fallback slots before the declared
        // fallbacks so e.g. a bold-style code point missing from the
        // bold font can still use the regular weight.
        for candidate in [&self.regular, &self.bold, &self.italic, &self.mono] {
            if std::ptr::eq(candidate, primary) {
                continue;
            }
            if candidate.has_glyph(c) {
                return Some(candidate);
            }
        }
        self.fallbacks.iter().find(|fb| fb.has_glyph(c))
    }
}

/// Builds a [`FontCache`] from the fonts bundled at compile time in
/// `src-tauri/resources/pdf/fonts/`. The set must stay in sync with the
/// defaults declared in `resources/pdf/themes/report.toml`.
///
/// # Errors
/// Returns [`RenderError::Font`] if any baked-in font blob cannot be
/// parsed (which would indicate a broken build).
pub fn bundled_report_cache() -> Result<FontCache, RenderError> {
    const REGULAR: &[u8] =
        include_bytes!("../../../../../../resources/pdf/fonts/LiberationSans-Regular.ttf");
    const BOLD: &[u8] =
        include_bytes!("../../../../../../resources/pdf/fonts/LiberationSans-Bold.ttf");
    const ITALIC: &[u8] =
        include_bytes!("../../../../../../resources/pdf/fonts/LiberationSans-Italic.ttf");
    const DEJAVU: &[u8] =
        include_bytes!("../../../../../../resources/pdf/fonts/DejaVuSans.ttf");

    let regular = LoadedFont::from_bytes(FontSlot::Regular, REGULAR.to_vec())?;
    let bold = LoadedFont::from_bytes(FontSlot::Bold, BOLD.to_vec())?;
    let italic = LoadedFont::from_bytes(FontSlot::Italic, ITALIC.to_vec())?;
    let mono = LoadedFont::from_bytes(FontSlot::Mono, REGULAR.to_vec())?;
    let fallback = LoadedFont::from_bytes(FontSlot::Fallback(0), DEJAVU.to_vec())?;
    Ok(FontCache::from_parts(regular, bold, italic, mono, vec![fallback]))
}

fn load_slot(
    slot: FontSlot,
    resources_root: &Path,
    relative: &str,
) -> Result<LoadedFont, RenderError> {
    let path: PathBuf = resources_root.join(relative);
    let bytes = std::fs::read(&path)?;
    let family = {
        let face = Face::parse(&bytes, 0).map_err(|e| RenderError::Font(e.to_string()))?;
        read_family_name(&face)
            .unwrap_or_else(|| path.file_stem().map_or_else(|| "Unknown".to_string(), |s| s.to_string_lossy().into_owned()))
    };
    Ok(LoadedFont {
        slot,
        family,
        bytes,
    })
}

/// Reads the `name` table family for `face`. Public so the renderer can
/// keep a single implementation.
#[must_use]
pub fn read_family_name(face: &Face<'_>) -> Option<String> {
    face.names().into_iter().find_map(|n| {
        if n.name_id == name_id::FAMILY {
            n.to_string()
        } else {
            None
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn resources_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdf/fonts")
    }

    fn cache() -> FontCache {
        let spec = Fonts {
            regular: "LiberationSans-Regular.ttf".to_string(),
            bold: "LiberationSans-Bold.ttf".to_string(),
            italic: "LiberationSans-Italic.ttf".to_string(),
            mono: "LiberationSans-Regular.ttf".to_string(),
            fallbacks: vec!["DejaVuSans.ttf".to_string()],
        };
        FontCache::load(&spec, &resources_root()).expect("fonts load")
    }

    #[test]
    fn loads_every_slot_plus_fallback() {
        let c = cache();
        assert_eq!(c.len(), 5);
        assert!(c.regular.family().to_lowercase().contains("liberation"));
        assert!(c.bold.family().to_lowercase().contains("liberation"));
        assert!(c.italic.family().to_lowercase().contains("liberation"));
        assert!(c.fallbacks[0].family().to_lowercase().contains("dejavu"));
    }

    #[test]
    fn primary_for_respects_style_priority() {
        let c = cache();
        let code = InlineStyle {
            code: true,
            ..InlineStyle::PLAIN
        };
        assert_eq!(c.primary_for(&code).slot(), FontSlot::Mono);

        let bold = InlineStyle {
            bold: true,
            ..InlineStyle::PLAIN
        };
        assert_eq!(c.primary_for(&bold).slot(), FontSlot::Bold);

        let italic = InlineStyle {
            italic: true,
            ..InlineStyle::PLAIN
        };
        assert_eq!(c.primary_for(&italic).slot(), FontSlot::Italic);

        assert_eq!(c.primary_for(&InlineStyle::PLAIN).slot(), FontSlot::Regular);
    }

    #[test]
    fn resolve_returns_primary_when_it_has_glyph() {
        let c = cache();
        let got = c.resolve('A', &InlineStyle::PLAIN).expect("'A' in regular");
        assert_eq!(got.slot(), FontSlot::Regular);
    }

    #[test]
    fn resolve_walks_to_fallback_when_primary_missing_glyph() {
        let c = cache();
        // Pick any code point present in the DejaVu fallback but not in
        // Liberation Sans. If the fixture ever evolves (different fonts),
        // adjust this locator rather than hard-coding a code point.
        let only_in_fallback = probe_char_in_second_not_first(
            &c.regular,
            &c.fallbacks[0],
        )
        .expect("fallback must carry at least one glyph absent from primary");
        let picked = c
            .resolve(only_in_fallback, &InlineStyle::PLAIN)
            .expect("fallback must supply the glyph");
        assert!(
            matches!(picked.slot(), FontSlot::Fallback(_)),
            "expected a fallback slot, got {:?}",
            picked.slot()
        );
    }

    #[test]
    fn resolve_returns_none_when_no_font_has_glyph() {
        let c = cache();
        // U+E000 is in the Private Use Area — no mainstream font maps it.
        let pua = '\u{E000}';
        assert!(c.resolve(pua, &InlineStyle::PLAIN).is_none());
    }

    fn probe_char_in_second_not_first(first: &LoadedFont, second: &LoadedFont) -> Option<char> {
        // Walk a broad range that includes box drawing, arrows, math, and
        // emoji presentation selectors. The fallback (DejaVu) covers many
        // of these while Liberation Sans typically does not.
        let ranges: &[std::ops::RangeInclusive<u32>] = &[
            0x2500..=0x257F, // Box Drawing
            0x2700..=0x27BF, // Dingbats
            0x2300..=0x23FF, // Misc Technical
        ];
        for range in ranges {
            for cp in range.clone() {
                if let Some(c) = char::from_u32(cp) {
                    if !first.has_glyph(c) && second.has_glyph(c) {
                        return Some(c);
                    }
                }
            }
        }
        None
    }
}
