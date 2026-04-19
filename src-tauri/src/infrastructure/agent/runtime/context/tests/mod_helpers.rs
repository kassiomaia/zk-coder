//! Tests for the text-truncation helpers exposed by `context/mod.rs`.

use super::super::truncate_text;

#[test]
fn test_truncate_text_handles_multibyte_boundary() {
    let text = format!("{}{}{}", "a".repeat(499), "─", "b");

    let truncated = truncate_text(&text, 500);

    assert!(truncated.ends_with("..."));
    assert!(truncated.starts_with(&"a".repeat(499)));
}
