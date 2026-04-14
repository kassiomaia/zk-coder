//! Small newtype that prevents accidental leaks of sensitive strings through
//! `Debug` / `Display` impls. Used for the Google Custom Search API key.
//!
//! A third-party crate (`secrecy`) would give us zero-on-drop, `Serde`-safe
//! handling, and a broader API, but the surface area we need is tiny: don't
//! print in panic messages or structured logs. Keeping it in-crate avoids a
//! new dependency while still giving us the redaction guarantee.

use std::fmt;

/// Wrapper around a sensitive `String` that redacts itself in `Debug` and
/// `Display` output. Call [`RedactedSecret::expose`] to obtain the underlying
/// value when the caller explicitly needs it (e.g. building the outbound HTTP
/// request).
pub struct RedactedSecret(String);

impl RedactedSecret {
    /// Wraps a sensitive string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the raw inner value. Callers are responsible for ensuring the
    /// returned reference does not escape to logs or the UI.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RedactedSecret(<redacted>)")
    }
}

impl fmt::Display for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl Clone for RedactedSecret {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_never_reveal_the_secret() {
        let s = RedactedSecret::new("super-secret-key");
        assert_eq!(format!("{s}"), "<redacted>");
        assert_eq!(format!("{s:?}"), "RedactedSecret(<redacted>)");
        assert!(!format!("{s}").contains("super"));
        assert!(!format!("{s:?}").contains("super"));
    }

    #[test]
    fn expose_returns_the_original_value() {
        let s = RedactedSecret::new("abc");
        assert_eq!(s.expose(), "abc");
    }

    #[test]
    fn cloning_preserves_the_inner_value() {
        let s = RedactedSecret::new("abc").clone();
        assert_eq!(s.expose(), "abc");
    }
}
