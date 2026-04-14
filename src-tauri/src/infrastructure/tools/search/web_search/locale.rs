//! System-locale resolver for the web-search tool's `hl` parameter.
//!
//! Two seams keep this testable:
//! - [`LocaleProvider`] abstracts "where does the host locale come from",
//!   so unit tests can inject a known value without relying on whatever is
//!   set on the machine running them.
//! - [`resolve_hl`] is a pure function that merges an explicit `language`
//!   argument with the provider's result, producing the lowercase BCP-47
//!   tag we hand to Google (or `None` when neither source has a value).

/// Indirection over `sys_locale::get_locale()` so tests never depend on the
/// host's configured locale.
pub trait LocaleProvider: Send + Sync {
    /// Returns the host's current locale as a BCP-47 tag when available.
    fn locale(&self) -> Option<String>;
}

/// Real implementation backed by the `sys-locale` crate.
pub struct SystemLocale;

impl LocaleProvider for SystemLocale {
    fn locale(&self) -> Option<String> {
        sys_locale::get_locale()
    }
}

/// Test helper that returns a fixed locale.
#[cfg(test)]
pub struct FixedLocale(pub Option<String>);

#[cfg(test)]
impl LocaleProvider for FixedLocale {
    fn locale(&self) -> Option<String> {
        self.0.clone()
    }
}

/// Resolves the final `hl` value by merging an explicit argument with the
/// system locale.
///
/// Rules:
/// 1. If `explicit` is `Some`, it wins (after trimming + lowercasing).
/// 2. Otherwise the provider's locale is used (after trimming + lowercasing).
/// 3. An empty-after-trim value from either source is treated as "not set",
///    so we don't end up hashing `hl=""` into cache keys in a way that
///    accidentally differs from `hl` being absent.
#[must_use]
pub fn resolve_hl(explicit: Option<String>, provider: &dyn LocaleProvider) -> Option<String> {
    let explicit_normalized = explicit.as_deref().and_then(normalize);
    if explicit_normalized.is_some() {
        return explicit_normalized;
    }
    provider.locale().as_deref().and_then(normalize)
}

fn normalize(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_system_locale_when_language_is_none() {
        let provider = FixedLocale(Some("en-US".to_string()));
        assert_eq!(resolve_hl(None, &provider).as_deref(), Some("en-us"));
    }

    #[test]
    fn explicit_language_overrides_system_locale() {
        let provider = FixedLocale(Some("en-US".to_string()));
        assert_eq!(
            resolve_hl(Some("pt-BR".to_string()), &provider).as_deref(),
            Some("pt-br")
        );
    }

    #[test]
    fn omits_hl_when_system_locale_cannot_be_determined() {
        let provider = FixedLocale(None);
        assert_eq!(resolve_hl(None, &provider), None);
    }

    #[test]
    fn omits_hl_when_system_locale_is_empty_string() {
        let provider = FixedLocale(Some(String::new()));
        assert_eq!(resolve_hl(None, &provider), None);
    }

    #[test]
    fn omits_hl_when_system_locale_is_whitespace_only() {
        let provider = FixedLocale(Some("   ".to_string()));
        assert_eq!(resolve_hl(None, &provider), None);
    }

    #[test]
    fn normalizes_locale_to_lowercase_bcp47() {
        let provider = FixedLocale(None);
        assert_eq!(
            resolve_hl(Some("EN-US".to_string()), &provider).as_deref(),
            Some("en-us")
        );
    }

    #[test]
    fn trims_explicit_value_before_returning() {
        let provider = FixedLocale(None);
        assert_eq!(
            resolve_hl(Some("  en  ".to_string()), &provider).as_deref(),
            Some("en")
        );
    }

    #[test]
    fn empty_explicit_falls_through_to_provider() {
        // An LLM that deliberately sends `"language": ""` should be treated
        // the same as omitting the field — fall back to the host locale.
        let provider = FixedLocale(Some("de".to_string()));
        assert_eq!(
            resolve_hl(Some(String::new()), &provider).as_deref(),
            Some("de"),
            "empty explicit must not veto the system locale"
        );
    }
}
