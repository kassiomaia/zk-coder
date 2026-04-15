//! Shared infrastructure utilities.

/// Truncates text safely at character boundaries.
#[must_use]
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }

    let truncated = text.chars().take(max_len).collect::<String>();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn truncate_text_keeps_short_inputs_intact() {
        assert_eq!(truncate_text("hello", 10), "hello");
    }

    #[test]
    fn truncate_text_preserves_utf8_boundaries() {
        let text = format!("{}{}{}", "a".repeat(499), "─", "b");
        let truncated = truncate_text(&text, 500);

        assert!(truncated.starts_with(&"a".repeat(499)));
        assert!(truncated.ends_with("..."));
    }
}
