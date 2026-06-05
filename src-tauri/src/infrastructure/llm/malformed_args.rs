//! Shared contract for surfacing malformed tool-call JSON arguments as a
//! recoverable, tool-level error instead of a fatal `LlmError::Parse`.
//!
//! When an LLM provider returns `function.arguments` that `serde_json` cannot
//! parse, the OpenAI-compatible client wraps the failure in a synthetic
//! `ToolCall` whose `arguments` value carries a single sentinel key. The agent
//! runtime recognises this sentinel before dispatch and emits an error-flagged
//! tool result so the model can self-correct on the next step.
//!
//! Keeping the contract in one module prevents the producer (LLM client) and
//! consumer (agent runtime) from drifting on the sentinel shape.
//!
//! See `docs/issues/agent-plan-subtasks-malformed-json/issue.md`.
use serde_json::{json, Value};

/// Top-level key written into `ToolCall.arguments` when the LLM-returned JSON
/// could not be parsed. The `__zk_` prefix avoids collisions with any
/// plausible real tool argument name.
pub const MALFORMED_SENTINEL_KEY: &str = "__zk_malformed_arguments__";

/// Half-width of the diagnostic window extracted around the parser's reported
/// column. The final window is roughly `2 * CONTEXT_WINDOW_RADIUS` chars.
const CONTEXT_WINDOW_RADIUS: usize = 80;

/// Builds the sentinel `arguments` value written onto a synthetic `ToolCall`
/// when JSON parsing fails.
///
/// `raw_arguments` is the exact string returned by the provider.
/// `error` is the `serde_json::Error` produced while parsing it.
#[must_use]
pub fn make_sentinel_value(raw_arguments: &str, error: &serde_json::Error) -> Value {
    let column = error.column();
    let line = error.line();
    let window = extract_window(raw_arguments, column, CONTEXT_WINDOW_RADIUS);

    json!({
        MALFORMED_SENTINEL_KEY: {
            "parserError": error.to_string(),
            "line": line,
            "column": column,
            "windowBefore": window.before,
            "offendingChar": window.at,
            "windowAfter": window.after,
            "rawLength": raw_arguments.chars().count(),
        }
    })
}

/// Returns the marker payload if `arguments` came from [`make_sentinel_value`].
///
/// The returned `Value` is borrowed from `arguments` and carries the
/// diagnostic fields (`parserError`, `column`, `windowBefore`, …).
#[must_use]
pub fn extract_marker(arguments: &Value) -> Option<&Value> {
    arguments
        .as_object()
        .and_then(|obj| obj.get(MALFORMED_SENTINEL_KEY))
}

/// Formats a human-readable window string `"<before>»<at>«<after>"` suitable
/// for logging. Returns `None` if no marker is present.
#[must_use]
pub fn format_window(marker: &Value) -> Option<String> {
    let before = marker.get("windowBefore")?.as_str().unwrap_or("");
    let at = marker.get("offendingChar")?.as_str().unwrap_or("");
    let after = marker.get("windowAfter")?.as_str().unwrap_or("");
    Some(format!("{before}»{at}«{after}"))
}

/// Builds the error-result body that the agent runtime feeds back to the LLM
/// when it sees the sentinel. The body includes diagnostic context so the
/// model can self-correct on the next step.
#[must_use]
pub fn build_error_result(tool_name: &str, marker: &Value) -> Value {
    let parser_error = marker
        .get("parserError")
        .and_then(Value::as_str)
        .unwrap_or("unknown JSON parser error");
    let column = marker.get("column").and_then(Value::as_u64).unwrap_or(0);
    let window = format_window(marker).unwrap_or_default();

    json!({
        "error": format!(
            "Your last `{tool_name}` call had malformed JSON arguments that could not be \
             parsed ({parser_error}). Retry with valid JSON. \
             Context window around the offending character (column {column}): {window}"
        ),
        "reason": "malformed_tool_arguments",
        "tool": tool_name,
        "parserError": parser_error,
        "column": column,
    })
}

struct Window<'a> {
    before: &'a str,
    at: &'a str,
    after: &'a str,
}

/// Extracts a substring window around a 1-indexed `column` reported by
/// `serde_json`. Works on character boundaries so multibyte input stays valid
/// UTF-8, and tolerates a `column` past the end of `raw` (the parser reports
/// column `N+1` for "unexpected end of input" errors).
fn extract_window(raw: &str, column: usize, radius: usize) -> Window<'_> {
    // serde_json columns are 1-based. Convert to a 0-based *character* index,
    // clamped to the string's char count.
    let char_count = raw.chars().count();
    let at_idx = column.saturating_sub(1).min(char_count);

    let char_indices: Vec<(usize, char)> = raw.char_indices().collect();

    let start_char = at_idx.saturating_sub(radius);
    let end_char = (at_idx + radius).min(char_count);

    let before_start = char_indices.get(start_char).map_or(0, |(byte, _)| *byte);
    let before_end = char_indices
        .get(at_idx)
        .map_or(raw.len(), |(byte, _)| *byte);
    let at_end = char_indices
        .get(at_idx + 1)
        .map_or(raw.len(), |(byte, _)| *byte);
    let after_end = char_indices
        .get(end_char)
        .map_or(raw.len(), |(byte, _)| *byte);

    Window {
        before: &raw[before_start..before_end],
        at: &raw[before_end..at_end],
        after: &raw[at_end..after_end],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_error(raw: &str) -> serde_json::Error {
        serde_json::from_str::<Value>(raw).unwrap_err()
    }

    #[test]
    fn window_captures_offending_char_and_neighbourhood() {
        let raw = r#"{"a": 1, "b": 2, "c": 3 "d": 4}"#; // missing comma before "d"
        let err = parse_error(raw);
        let column = err.column();
        let window = extract_window(raw, column, 5);
        // Offending char is the `"` that starts the unexpected key `"d"`.
        assert_eq!(window.at, "\"");
        assert!(window.before.ends_with("3 "));
        assert!(window.after.starts_with("d\":"));
    }

    #[test]
    fn window_handles_column_past_end_of_input() {
        let raw = "{\"a\": 1"; // unexpected end
        let err = parse_error(raw);
        let window = extract_window(raw, err.column(), 3);
        // Must not panic. The assembled window must be a valid UTF-8
        // substring of `raw`, and the concatenation must never exceed it.
        let assembled = format!("{}{}{}", window.before, window.at, window.after);
        assert!(raw.contains(&assembled));
    }

    #[test]
    fn window_handles_column_far_past_end_without_panic() {
        let raw = r#"{"a": 1}"#;
        // Nothing in `raw` is at column 999; we should clamp gracefully.
        let window = extract_window(raw, 999, 5);
        let assembled = format!("{}{}{}", window.before, window.at, window.after);
        assert!(raw.contains(&assembled));
    }

    #[test]
    fn sentinel_round_trips_through_extract_marker() {
        let raw = r#"{"a": 1 "b": 2}"#;
        let err = parse_error(raw);
        let value = make_sentinel_value(raw, &err);
        let marker = extract_marker(&value).expect("marker present");
        assert!(marker.get("parserError").and_then(Value::as_str).is_some());
        assert_eq!(
            marker.get("column").and_then(Value::as_u64),
            Some(err.column() as u64)
        );
        assert_eq!(
            marker.get("rawLength").and_then(Value::as_u64),
            Some(raw.chars().count() as u64)
        );
    }

    #[test]
    fn extract_marker_returns_none_for_regular_arguments() {
        let value = json!({"path": "/tmp/foo"});
        assert!(extract_marker(&value).is_none());
    }

    #[test]
    fn build_error_result_includes_tool_name_and_column() {
        let raw = r#"{"a": 1 "b": 2}"#;
        let err = parse_error(raw);
        let value = make_sentinel_value(raw, &err);
        let marker = extract_marker(&value).unwrap();
        let body = build_error_result("agent_plan_subtasks", marker);
        let msg = body.get("error").and_then(Value::as_str).unwrap();
        assert!(msg.contains("agent_plan_subtasks"));
        assert!(msg.contains(&format!("column {}", err.column())));
        assert_eq!(
            body.get("reason").and_then(Value::as_str),
            Some("malformed_tool_arguments")
        );
    }

    #[test]
    fn window_preserves_utf8_boundary_with_multibyte_chars() {
        let raw = "{\"role\": \"sênior\", \"forbidden\": [\"^\\\\s+$\" \"TODO\"]}";
        let err = parse_error(raw);
        // Must not panic; returns something UTF-8 valid.
        let window = extract_window(raw, err.column(), 20);
        let _ = format!("{}{}{}", window.before, window.at, window.after);
    }
}
