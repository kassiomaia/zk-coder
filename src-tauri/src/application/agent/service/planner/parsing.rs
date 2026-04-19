//! Response normalization helpers for the planner.
//!
//! LLM providers wrap planner output in inconsistent ways: markdown fences,
//! `<think>` reasoning preambles, prose surrounding the JSON body, etc.
//! These helpers strip those wrappers and extract the JSON object the
//! planner relies on for `serde` parsing.

use log::{info, warn};

/// Returns `true` when the provider's `finish_reason` indicates the
/// response was truncated because it hit the output token cap.
///
/// Values observed across providers (case-insensitive): OpenAI uses
/// `"length"`, Anthropic uses `"max_tokens"`, Google/Gemini uses
/// `"MAX_TOKENS"`. Any other value (including `None` and `"stop"`)
/// means the model finished cleanly and the downstream JSON parser
/// should run as normal.
pub(super) fn finish_reason_indicates_truncation(reason: Option<&str>) -> bool {
    match reason {
        Some(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "length" | "max_tokens")
        }
        None => false,
    }
}

/// Strips markdown code block wrappers from LLM responses.
///
/// LLMs often wrap JSON in ```json ... ``` blocks even when instructed not to.
/// This function extracts the actual content from within such blocks.
pub(super) fn strip_markdown_code_block(content: &str) -> String {
    let content = content.trim();

    // If content is empty, return early
    if content.is_empty() {
        warn!("[Planner] strip_markdown_code_block called with empty content");
        return String::new();
    }

    // Check for ```json or ``` at the start
    if content.starts_with("```") {
        // Find the end of the opening fence line
        let after_opening_fence = if let Some(newline_pos) = content.find('\n') {
            &content[newline_pos + 1..]
        } else {
            // No newline after opening fence, treat entire thing as fence
            warn!("[Planner] Markdown fence found but no newline, returning empty");
            return String::new();
        };

        // Find the closing fence (```)
        let closing_fence = "\n```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping with newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // Also check for closing fence without leading newline
        let closing_fence_no_nl = "```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence_no_nl) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping without newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // No closing fence found, return everything after opening fence
        let result = after_opening_fence.trim().to_string();
        if result.is_empty() && !after_opening_fence.trim().is_empty() {
            warn!("[Planner] No closing fence found but stripping resulted in empty, returning original content");
            return after_opening_fence.to_string();
        }
        return result;
    }

    // Check for closing fence at the end (even without opening fence)
    if let Some(fence_pos) = content.rfind("\n```") {
        let possible_content = &content[..fence_pos];
        // Only strip if there's meaningful content before it
        let trimmed = possible_content.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("```") {
            info!("[Planner] Stripped closing fence from end of content");
            return trimmed.to_string();
        }
    }

    // No markdown wrapper found, return original
    info!("[Planner] No markdown wrapper found, returning original content");
    content.to_string()
}

/// Removes leading `<think>...</think>` blocks from an LLM response.
///
/// Some providers prepend reasoning wrapped in XML-like tags even when the user
/// requested plain JSON output. For planning, that reasoning is transport noise.
pub(super) fn strip_leading_think_blocks(content: &str) -> String {
    let mut remaining = content.trim();

    loop {
        if !remaining.starts_with("<think>") {
            break;
        }

        let Some(end_tag_index) = remaining.find("</think>") else {
            warn!("[Planner] Unterminated <think> block found in planner response");
            break;
        };

        remaining = remaining[end_tag_index + "</think>".len()..].trim_start();
    }

    remaining.to_string()
}

/// Extracts the first complete top-level JSON object from a response string.
///
/// This tolerates explanatory text before or after the JSON body while avoiding
/// partial-object parsing. It only returns object-shaped JSON because planner
/// responses are required to be objects.
pub(super) fn extract_first_json_object(content: &str) -> Option<String> {
    let bytes = content.as_bytes();
    let mut start_index = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'\\' if in_string => {
                escape = !escape;
            }
            b'"' if !escape => {
                in_string = !in_string;
            }
            _ => {
                escape = false;

                if in_string {
                    continue;
                }

                match byte {
                    b'{' => {
                        if start_index.is_none() {
                            start_index = Some(index);
                        }
                        depth += 1;
                    }
                    b'}' if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            let start = start_index?;
                            return Some(content[start..=index].trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// Normalizes an LLM response into the JSON object expected by the planner.
///
/// The planner accepts provider-specific wrappers such as `<think>` blocks,
/// fenced code blocks, or prose surrounding the JSON payload.
pub(super) fn normalize_planning_json(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("LLM returned empty response. This may indicate a configuration issue with your LLM provider.".to_string());
    }

    let without_think = strip_leading_think_blocks(trimmed);
    let without_markdown = strip_markdown_code_block(&without_think);

    if without_markdown.trim().is_empty() {
        return Err(format!(
            "LLM returned empty response after normalization. Raw response was: '{}'",
            content
        ));
    }

    if let Some(json) = extract_first_json_object(&without_markdown) {
        return Ok(json);
    }

    Err(format!(
        "Could not locate a JSON object in planner response. Normalized response was: '{}'",
        without_markdown
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_code_block_with_json_wrapper() {
        // Test the exact format the LLM returned
        let wrapped = r#"```json
{
  "title": "Test",
  "description": "Test description",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
        assert!(!stripped.contains("json"));
    }

    #[test]
    fn test_strip_markdown_code_block_with_plain_wrapper() {
        let wrapped = r#"```
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
    }

    #[test]
    fn test_strip_markdown_code_block_plain_json() {
        // Plain JSON should pass through unchanged
        let plain = r#"{"title": "Test", "description": "Test"}"#;

        let stripped = strip_markdown_code_block(plain);

        assert_eq!(stripped, plain);
    }

    #[test]
    fn test_strip_markdown_code_block_with_newlines() {
        // Test with extra newlines around the JSON (like the actual LLM response)
        let wrapped = r#"```json
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_markdown_code_block_no_closing_fence() {
        // Some LLMs might forget the closing fence
        let wrapped = r#"```json
{
  "title": "Test"
}"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_leading_think_blocks_removes_reasoning_preamble() {
        let wrapped = r#"<think>Reasoning goes here</think>

{"title":"Test"}"#;

        let stripped = strip_leading_think_blocks(wrapped);

        assert_eq!(stripped, r#"{"title":"Test"}"#);
    }

    #[test]
    fn test_extract_first_json_object_ignores_prefix_and_suffix_text() {
        let content = r#"Some prose before
{"title":"Test","description":"Example","is_non_trivial":false,"subtasks":[]}
Some prose after"#;

        let extracted = extract_first_json_object(content).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn test_normalize_planning_json_handles_think_and_fenced_json() {
        let wrapped = r#"<think>The user said hi, so this is trivial.</think>

```json
{
  "title": "Greeting Response",
  "description": "Acknowledge the user's greeting",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let normalized = normalize_planning_json(wrapped).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&normalized).unwrap();

        assert_eq!(parsed["title"], "Greeting Response");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn finish_reason_truncation_detector_covers_every_provider_spelling() {
        // OpenAI spelling.
        assert!(finish_reason_indicates_truncation(Some("length")));
        // Anthropic spelling.
        assert!(finish_reason_indicates_truncation(Some("max_tokens")));
        // Google / Gemini spelling (upper-case) — normalized lower.
        assert!(finish_reason_indicates_truncation(Some("MAX_TOKENS")));
        // Surrounding whitespace from stray provider quirks.
        assert!(finish_reason_indicates_truncation(Some("  length  ")));
    }

    #[test]
    fn finish_reason_truncation_detector_rejects_clean_finishes() {
        assert!(!finish_reason_indicates_truncation(None));
        assert!(!finish_reason_indicates_truncation(Some("stop")));
        assert!(!finish_reason_indicates_truncation(Some("tool_calls")));
        assert!(!finish_reason_indicates_truncation(Some("end_turn")));
        assert!(!finish_reason_indicates_truncation(Some("")));
    }

    #[test]
    fn test_normalize_planning_json_rejects_missing_json() {
        // Prose that survives `<think>` stripping but contains no JSON object —
        // exercises the "Could not locate a JSON object" branch of
        // `normalize_planning_json`. An input like `<think>...</think>` alone
        // would strip to empty and hit the empty-after-normalization branch
        // instead, which is covered elsewhere.
        let error =
            normalize_planning_json("I'm sorry, I can't help with that request.").unwrap_err();

        assert!(
            error.contains("Could not locate a JSON object"),
            "expected missing-JSON error, got: {error}"
        );
    }
}
