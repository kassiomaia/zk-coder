//! Pure helpers that transform provider-supplied strings into the clean shape
//! the LLM and frontend consume.
//!
//! None of these functions touch the network or the database — they stay
//! inside the crate so they can be unit-tested without any Tauri wiring.

use regex::Regex;

/// Maximum snippet length after sanitization (characters, not bytes).
/// Enforces the token-efficiency requirement in `spec.md §5`.
pub const MAX_SNIPPET_CHARS: usize = 300;

/// Query-string parameter prefixes / exact names stripped from result URLs.
/// Keeps the tool from redistributing tracking identifiers.
const TRACKING_PREFIXES: &[&str] = &["utm_"];
const TRACKING_EXACT: &[&str] = &[
    "fbclid", "gclid", "msclkid", "yclid", "mc_cid", "mc_eid", "_hsenc", "_hsmi", "igshid",
];

/// Strips any `<…>` HTML tag from `input`. The result collapses consecutive
/// whitespace so snippet markup like `<b>async</b>\n  traits` becomes a clean
/// single line.
pub fn strip_html(input: &str) -> String {
    // Replace tags with a space rather than empty: this preserves word
    // boundaries for block/line-break tags (`<br/>`, `<p>`, `</div>`) while
    // still removing inline ones like `<b>…</b>`. The whitespace collapse
    // below drops the extras.
    let tag_re = Regex::new(r"<[^>]+>").expect("sanitize: tag regex is a static literal");
    let without_tags = tag_re.replace_all(input, " ").to_string();
    without_tags.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decodes common HTML entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#39;`,
/// `&nbsp;`, `&apos;`) plus numeric decimal (`&#NN;`) and hex (`&#xHH;`) forms.
/// Unknown or malformed entities are passed through unchanged so the original
/// text survives even when we don't recognize an obscure entity.
pub fn decode_entities(input: &str) -> String {
    let mut out = input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    let dec_re = Regex::new(r"&#(\d+);").expect("sanitize: dec regex is a static literal");
    out = dec_re
        .replace_all(&out, |caps: &regex::Captures| {
            caps[1]
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| caps[0].to_string(), |c| c.to_string())
        })
        .to_string();

    let hex_re =
        Regex::new(r"&#x([0-9A-Fa-f]+);").expect("sanitize: hex regex is a static literal");
    hex_re
        .replace_all(&out, |caps: &regex::Captures| {
            u32::from_str_radix(&caps[1], 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| caps[0].to_string(), |c| c.to_string())
        })
        .to_string()
}

/// Applies both transforms and trims the result to [`MAX_SNIPPET_CHARS`]
/// characters. Truncation is character-aware (never splits a multibyte
/// codepoint) and appends an ellipsis when truncation actually happened.
pub fn sanitize_snippet(raw: &str) -> String {
    let cleaned = decode_entities(&strip_html(raw));
    let chars: Vec<char> = cleaned.chars().collect();
    if chars.len() <= MAX_SNIPPET_CHARS {
        cleaned
    } else {
        let mut s: String = chars[..MAX_SNIPPET_CHARS - 1].iter().collect();
        s.push('…');
        s
    }
}

/// Returns the URL with tracking query parameters removed. Preserves path,
/// fragment, and all non-tracking query parameters. Invalid URLs are
/// returned unchanged — callers are responsible for handling the resulting
/// ambiguity (typically via [`source_domain`] returning `None`).
pub fn strip_tracking_params(url: &str) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else {
        return url.to_string();
    };

    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| !is_tracking_param(key))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if kept.is_empty() {
        parsed.set_query(None);
    } else {
        parsed
            .query_pairs_mut()
            .clear()
            .extend_pairs(kept.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }

    parsed.to_string()
}

fn is_tracking_param(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if TRACKING_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return true;
    }
    TRACKING_EXACT.iter().any(|n| lower == *n)
}

/// Extracts the lowercase host portion of an absolute `http(s)` URL.
/// Returns `None` for non-`http(s)` URLs, URLs without a host, and unparseable
/// inputs — the tool drops results with no usable domain.
pub fn source_domain(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    match parsed.scheme() {
        "http" | "https" => parsed.host_str().map(str::to_ascii_lowercase),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_bold_tags() {
        assert_eq!(strip_html("Rust <b>async</b> traits"), "Rust async traits");
    }

    #[test]
    fn strip_html_collapses_whitespace() {
        assert_eq!(strip_html("a<br/>b\n  c"), "a b c");
    }

    #[test]
    fn decode_entities_handles_named_entities() {
        assert_eq!(
            decode_entities("Tom &amp; Jerry &lt;3 &quot;hi&quot;"),
            "Tom & Jerry <3 \"hi\""
        );
    }

    #[test]
    fn decode_entities_handles_numeric_entities() {
        assert_eq!(decode_entities("&#65;&#x42;"), "AB");
    }

    #[test]
    fn decode_entities_passes_unknown_entities_through() {
        // `&foo;` is not a valid entity but we must not destroy it.
        assert_eq!(decode_entities("a &foo; b"), "a &foo; b");
    }

    #[test]
    fn sanitize_snippet_truncates_long_text_with_ellipsis() {
        let long = "a".repeat(MAX_SNIPPET_CHARS + 50);
        let out = sanitize_snippet(&long);
        assert_eq!(out.chars().count(), MAX_SNIPPET_CHARS);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn sanitize_snippet_leaves_short_text_untouched() {
        assert_eq!(sanitize_snippet("<b>hi</b>"), "hi");
    }

    #[test]
    fn strip_tracking_params_removes_utm_and_fbclid() {
        let url = "https://example.com/page?utm_source=x&utm_medium=y&fbclid=abc&keep=1";
        let cleaned = strip_tracking_params(url);
        assert!(cleaned.contains("keep=1"));
        assert!(!cleaned.contains("utm_"));
        assert!(!cleaned.contains("fbclid"));
    }

    #[test]
    fn strip_tracking_params_is_case_insensitive() {
        let url = "https://example.com/?UTM_SOURCE=1&FbClId=2&stay=true";
        let cleaned = strip_tracking_params(url);
        assert!(cleaned.contains("stay=true"));
        assert!(!cleaned.to_lowercase().contains("utm_source"));
        assert!(!cleaned.to_lowercase().contains("fbclid"));
    }

    #[test]
    fn strip_tracking_params_drops_query_string_when_only_tracking_was_present() {
        let url = "https://example.com/page?utm_source=x";
        let cleaned = strip_tracking_params(url);
        assert_eq!(cleaned, "https://example.com/page");
    }

    #[test]
    fn strip_tracking_params_preserves_unparseable_urls() {
        assert_eq!(strip_tracking_params("not a url"), "not a url");
    }

    #[test]
    fn source_domain_lowercases_host() {
        assert_eq!(
            source_domain("https://Svelte.DEV/docs").as_deref(),
            Some("svelte.dev")
        );
    }

    #[test]
    fn source_domain_rejects_non_http_schemes() {
        assert_eq!(source_domain("ftp://example.com"), None);
        assert_eq!(source_domain("javascript:alert(1)"), None);
    }

    #[test]
    fn source_domain_returns_none_for_invalid_url() {
        assert_eq!(source_domain("not a url"), None);
    }
}
