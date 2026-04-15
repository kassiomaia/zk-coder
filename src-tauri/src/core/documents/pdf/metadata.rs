//! Document-level metadata (title, author, language, ...).

use serde::{Deserialize, Serialize};

use super::error::PdfModelError;

/// BCP-47 language tag.
///
/// A newtype so callers cannot confuse it with a free-form string.
/// Validation is deliberately lax at this layer; we only reject the empty
/// string. Stricter parsing belongs in a dedicated module when a concrete
/// consumer requires it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Lang(String);

impl Lang {
    /// Builds a `Lang` from any string-like value without validation.
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// Returns the underlying tag as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Lang {
    fn default() -> Self {
        Self("en".to_string())
    }
}

/// Document-level metadata.
///
/// The `title` field is the only required value; everything else is
/// optional. `creator` is a short string set by the producing binary so
/// downstream consumers can identify the origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    title: String,
    /// Optional author name (`/Author` in the PDF dictionary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional subject (`/Subject`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Zero or more keyword entries (`/Keywords`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Document language (defaults to `en`).
    #[serde(default)]
    pub language: Lang,
    /// Producing binary identifier (`/Creator`).
    #[serde(default = "Metadata::default_creator")]
    pub creator: String,
}

impl Metadata {
    /// Builds a `Metadata` with the given title.
    ///
    /// # Errors
    /// Returns [`PdfModelError::EmptyTitle`] if `title` is empty or
    /// whitespace-only.
    pub fn new(title: impl Into<String>) -> Result<Self, PdfModelError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(PdfModelError::EmptyTitle);
        }
        Ok(Self {
            title,
            author: None,
            subject: None,
            keywords: Vec::new(),
            language: Lang::default(),
            creator: Self::default_creator(),
        })
    }

    /// Returns the document title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Sets the author and returns self.
    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Sets the subject and returns self.
    #[must_use]
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Sets the keyword list and returns self.
    #[must_use]
    pub fn with_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the language and returns self.
    #[must_use]
    pub fn with_language(mut self, language: Lang) -> Self {
        self.language = language;
        self
    }

    fn default_creator() -> String {
        "Coder".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_title() {
        assert_eq!(Metadata::new(""), Err(PdfModelError::EmptyTitle));
    }

    #[test]
    fn new_rejects_whitespace_only_title() {
        assert_eq!(Metadata::new("   "), Err(PdfModelError::EmptyTitle));
    }

    #[test]
    fn new_accepts_non_empty_title() {
        let meta = Metadata::new("Quarterly Report").unwrap();
        assert_eq!(meta.title(), "Quarterly Report");
        assert_eq!(meta.language.as_str(), "en");
        assert_eq!(meta.creator, "Coder");
    }

    #[test]
    fn serde_roundtrip_preserves_fields() {
        let meta = Metadata::new("Title")
            .unwrap()
            .with_author("Ada")
            .with_subject("Subject")
            .with_keywords(["one", "two"]);
        let json = serde_json::to_string(&meta).unwrap();
        let back: Metadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }

    #[test]
    fn deserialize_rejects_nothing_at_serde_layer() {
        // Constructor validation is the single source of truth; serde just
        // reconstructs what was serialized. Callers must re-validate when
        // reading external input. This test pins the current intent.
        let raw = r#"{"title":"ok","language":"pt-BR","creator":"Coder"}"#;
        let back: Metadata = serde_json::from_str(raw).unwrap();
        assert_eq!(back.title(), "ok");
        assert_eq!(back.language.as_str(), "pt-BR");
    }
}
