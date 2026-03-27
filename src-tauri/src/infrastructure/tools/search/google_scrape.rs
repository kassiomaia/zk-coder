//! Google web search tool (dev/test only).

use crate::infrastructure::tools::constants::ENV_GOOGLE_SCRAPE_ENABLED;
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;
use tauri::AppHandle;

/// Arguments for Google web search.
#[derive(Debug, Deserialize, PartialEq)]
pub struct GoogleSearchArgs {
    /// Search query string.
    pub query: String,
    /// Number of results to return (default: 5, max: 10).
    #[serde(default = "default_num_results")]
    pub num_results: usize,
    /// Language code (default: "en").
    #[serde(default)]
    pub language: Option<String>,
}

fn default_num_results() -> usize {
    5
}

/// Individual search result.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// Result title.
    pub title: String,
    /// Result URL.
    pub url: String,
    /// Result snippet/description.
    pub snippet: String,
    /// Source identifier.
    pub source: String,
}

/// Output from web search.
#[derive(Debug, Serialize)]
pub struct GoogleSearchOutput {
    /// Search query.
    pub query: String,
    /// Number of results returned.
    pub count: usize,
    /// Search results.
    pub results: Vec<SearchResult>,
    /// Warning about scraper stability.
    pub warning: String,
}

/// Dev-only keyless Google web search using HTML scraping (unstable).
pub struct GoogleScrapeTool;

impl Tool for GoogleScrapeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("search", "google_scrape"),
            description: "Dev-only keyless Google web search using HTML scraping (unstable).",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results (default: 5, max: 10)",
                        "minimum": 1,
                        "maximum": 10
                    },
                    "language": {
                        "type": "string",
                        "description": "Language code (default: en)"
                    }
                },
                "required": ["query"]
            }),
            requires_workspace: false,
            experimental: true,
            feature_flag: Some(ENV_GOOGLE_SCRAPE_ENABLED),
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            // Check feature flag
            if !crate::infrastructure::tools::constants::is_env_enabled(ENV_GOOGLE_SCRAPE_ENABLED) {
                return Err(ToolError::Disabled(format!(
                    "{}=1 is required for dev/test use",
                    ENV_GOOGLE_SCRAPE_ENABLED
                )));
            }

            let args: GoogleSearchArgs =
                crate::infrastructure::tools::error::parse_args("search_google_scrape", arguments)?;

            let query = args.query.trim().to_string();
            if query.is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let num_results = args.num_results.clamp(1, 10);

            // Build query parameters
            let query_params = vec![
                ("q", query.clone()),
                ("num", num_results.to_string()),
                ("hl", args.language.unwrap_or_else(|| "en".to_string())),
                ("pws", "0".to_string()),
                ("safe", "off".to_string()),
            ];

            // Build HTTP client
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .user_agent(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
                )
                .build()
                .map_err(|e| ToolError::failed(format!("Failed to build HTTP client: {}", e)))?;

            // Execute search
            let response = client
                .get("https://www.google.com/search")
                .query(&query_params)
                .send()
                .await
                .map_err(|e| ToolError::from_reqwest(e))?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ToolError::failed(format!(
                    "Google search failed with status {}: {}",
                    status.as_u16(),
                    body.chars().take(200).collect::<String>()
                )));
            }

            // Parse HTML response
            let html = response
                .text()
                .await
                .map_err(|e| ToolError::failed(format!("Failed to read response body: {}", e)))?;

            let results = GoogleScrapeTool::parse_google_search_results(&html, num_results);

            let output = GoogleSearchOutput {
                query,
                count: results.len(),
                results,
                warning: "Dev/test scraper: unstable markup and ToS-sensitive.".to_string(),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

impl GoogleScrapeTool {
    /// Decode HTML entities.
    fn decode_html_entities(input: &str) -> String {
        input
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&nbsp;", " ")
    }

    /// Strip HTML tags from text.
    fn strip_html_tags(input: &str) -> String {
        let without_tags = Regex::new(r"<[^>]+>").ok().map_or_else(
            || input.to_string(),
            |tag_re| tag_re.replace_all(input, "").to_string(),
        );
        let decoded = Self::decode_html_entities(without_tags.as_ref());
        decoded.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Extract URL from Google's encoded href.
    fn extract_google_result_url(encoded_href_query: &str) -> Option<String> {
        let decoded = Self::decode_html_entities(encoded_href_query);
        let query = if decoded.starts_with("q=") {
            decoded
        } else {
            format!("q={}", decoded)
        };
        let url = reqwest::Url::parse(&format!("https://www.google.com/url?{}", query)).ok()?;
        for (key, value) in url.query_pairs() {
            if key == "q" && (value.starts_with("http://") || value.starts_with("https://")) {
                return Some(value.into_owned());
            }
        }
        None
    }

    /// Parse Google search results from HTML.
    fn parse_google_search_results(html: &str, limit: usize) -> Vec<SearchResult> {
        let anchor_prefix = "<a href=\"/url?q=";
        let mut results = Vec::new();
        let mut cursor = 0;

        while results.len() < limit {
            let Some(anchor_offset) = html[cursor..].find(anchor_prefix) else {
                break;
            };
            let anchor_start = cursor + anchor_offset;
            let href_start = anchor_start + anchor_prefix.len();
            let Some(href_end_rel) = html[href_start..].find('"') else {
                break;
            };
            let href_end = href_start + href_end_rel;
            let href_value = &html[href_start..href_end];
            cursor = href_end.saturating_add(1);

            let Some(url) = Self::extract_google_result_url(href_value) else {
                continue;
            };

            let window_end = (anchor_start + 5000).min(html.len());
            let window = &html[anchor_start..window_end];

            let Some(h3_start_rel) = window.find("<h3") else {
                continue;
            };
            let h3_start = h3_start_rel;
            let Some(h3_open_end_rel) = window[h3_start..].find('>') else {
                continue;
            };
            let title_start = h3_start + h3_open_end_rel + 1;
            let Some(title_end_rel) = window[title_start..].find("</h3>") else {
                continue;
            };
            let title_end = title_start + title_end_rel;
            let title = Self::strip_html_tags(&window[title_start..title_end]);
            if title.is_empty() {
                continue;
            }

            let snippet = window
                .find("VwiC3b")
                .and_then(|class_offset| window[class_offset..].find('>').map(|o| class_offset + o))
                .and_then(|snippet_start| {
                    let content_start = snippet_start + 1;
                    window[content_start..]
                        .find("</div>")
                        .map(|snippet_end_rel| (content_start, content_start + snippet_end_rel))
                })
                .map(|(start, end)| Self::strip_html_tags(&window[start..end]))
                .unwrap_or_default();

            results.push(SearchResult {
                title,
                url,
                snippet,
                source: "google_scrape".to_string(),
            });
        }

        results
    }
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    // === Helper function tests ===

    #[test]
    fn test_decode_html_entities() {
        let input = "Hello &quot;World&quot; &amp; &lt;tag&gt;";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "Hello \"World\" & <tag>");
    }

    #[test]
    fn test_decode_html_entities_nbsp() {
        let input = "Hello&nbsp;World";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "Hello World");
    }

    #[test]
    fn test_decode_html_entities_apostrophe() {
        let input = "Don&#39;t stop believin&#39;";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "Don't stop believin'");
    }

    #[test]
    fn test_decode_html_entities_empty() {
        let input = "";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_decode_html_entities_no_entities() {
        let input = "Plain text without entities";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "Plain text without entities");
    }

    #[test]
    fn test_strip_html_tags() {
        let input = "<p>Hello <strong>World</strong></p>";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "Hello World");
    }

    #[test]
    fn test_strip_html_tags_nested() {
        let input = "<div><span>nested</span> <em>tags</em></div>";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "nested tags");
    }

    #[test]
    fn test_strip_html_tags_with_entities() {
        let input = "<p>Hello &quot;World&quot;</p>";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "Hello \"World\"");
    }

    #[test]
    fn test_strip_html_tags_empty() {
        let input = "";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_html_tags_whitespace_normalization() {
        let input = "<p>Word1</p>  <p>Word2</p>";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "Word1 Word2");
    }

    #[test]
    fn test_strip_html_tags_self_closing() {
        let input = "Text<br/>more<br />text";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "Text more text");
    }

    #[test]
    fn test_extract_google_result_url_valid() {
        let encoded = "https://example.com/path";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(result, Some("https://example.com/path".to_string()));
    }

    #[test]
    fn test_extract_google_result_url_with_q_prefix() {
        let encoded = "q=https://example.com/path";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(result, Some("https://example.com/path".to_string()));
    }

    #[test]
    fn test_extract_google_result_url_with_entities() {
        let encoded = "q=https://example.com/search?q=test&amp;lang=en";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(
            result,
            Some("https://example.com/search?q=test&lang=en".to_string())
        );
    }

    #[test]
    fn test_extract_google_result_url_non_http() {
        // Should filter out non-http URLs
        let encoded = "q=javascript:void(0)";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_google_result_url_empty() {
        let encoded = "";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_google_result_url_invalid() {
        let encoded = "not-a-url";
        let result = GoogleScrapeTool::extract_google_result_url(encoded);
        assert_eq!(result, None);
    }

    // === HTML parsing tests ===

    #[test]
    fn test_parse_google_search_results_empty_html() {
        let html = "";
        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_google_search_results_no_results() {
        let html = "<html><body>No results here</body></html>";
        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_google_search_results_minimal() {
        let html = r#"
            <html>
            <body>
                <a href="/url?q=https://example.com&amp;usg=xxx">
                    <h3>Example Title</h3>
                </a>
            </body>
            </html>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com");
        assert_eq!(results[0].title, "Example Title");
    }

    #[test]
    fn test_parse_google_search_results_with_snippet() {
        let html = r#"
            <a href="/url?q=https://example.com">
                <h3>Example Title</h3>
                <div class="VwiC3b">This is a snippet <em>highlighted</em> text</div>
            </a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("snippet"));
        assert!(results[0].snippet.contains("highlighted"));
    }

    #[test]
    fn test_parse_google_search_results_respects_limit() {
        let html = r#"
            <a href="/url?q=https://example1.com"><h3>Title 1</h3></a>
            <a href="/url?q=https://example2.com"><h3>Title 2</h3></a>
            <a href="/url?q=https://example3.com"><h3>Title 3</h3></a>
            <a href="/url?q=https://example4.com"><h3>Title 4</h3></a>
            <a href="/url?q=https://example5.com"><h3>Title 5</h3></a>
            <a href="/url?q=https://example6.com"><h3>Title 6</h3></a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_parse_google_search_results_empty_title_skipped() {
        let html = r#"
            <a href="/url?q=https://example.com">
                <h3></h3>
            </a>
            <a href="/url?q=https://example2.com">
                <h3>Valid Title</h3>
            </a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        // Empty title should be skipped
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Valid Title");
    }

    #[test]
    fn test_parse_google_search_results_with_html_in_title() {
        let html = r#"
            <a href="/url?q=https://example.com">
                <h3>Title with <em>emphasis</em> and <strong>bold</strong></h3>
            </a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Title with emphasis and bold");
    }

    #[test]
    fn test_parse_google_search_results_malformed_anchor() {
        let html = r#"
            <a href="/url?q=https://example.com">
                <h3>Valid Result</h3>
            </a>
            <a href="invalid-link">
                <h3>Invalid Link</h3>
            </a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        // Should only get valid result
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Valid Result");
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "query": "test search",
            "num_results": 3,
            "language": "en"
        });

        let parsed: GoogleSearchArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.query, "test search");
        assert_eq!(parsed.num_results, 3);
        assert_eq!(parsed.language, Some("en".to_string()));
    }

    #[test]
    fn test_parse_args_defaults() {
        let args = serde_json::json!({
            "query": "test"
        });

        let parsed: GoogleSearchArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.query, "test");
        assert_eq!(parsed.num_results, 5);
        assert_eq!(parsed.language, None);
    }

    #[test]
    fn test_default_num_results() {
        assert_eq!(default_num_results(), 5);
    }

    #[test]
    fn test_parse_args_missing_query() {
        let args = serde_json::json!({
            "num_results": 5
        });

        let parsed: Result<GoogleSearchArgs, _> = serde_json::from_value(args);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_google_search_args_partial_eq() {
        let args1 = GoogleSearchArgs {
            query: "test".to_string(),
            num_results: 5,
            language: Some("en".to_string()),
        };
        let args2 = GoogleSearchArgs {
            query: "test".to_string(),
            num_results: 5,
            language: Some("en".to_string()),
        };
        assert_eq!(args1, args2);
    }

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = GoogleScrapeTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "search_google_scrape");
        assert!(meta.description.contains("dev"));
        assert!(meta.description.contains("unstable"));
        assert!(!meta.requires_workspace);
        assert!(meta.experimental);
        assert!(meta.feature_flag.is_some());
    }

    #[test]
    fn test_tool_id_format() {
        let tool = GoogleScrapeTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "search");
        assert_eq!(meta.id.name, "google_scrape");
    }

    #[test]
    fn test_input_schema() {
        let tool = GoogleScrapeTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));

        // Check num_results constraints
        let num_results_prop = &schema["properties"]["num_results"];
        assert_eq!(num_results_prop["type"], "integer");
        assert_eq!(num_results_prop["minimum"], 1);
        assert_eq!(num_results_prop["maximum"], 10);
    }

    #[test]
    fn test_feature_flag() {
        let tool = GoogleScrapeTool;
        let meta = tool.meta();
        assert_eq!(meta.feature_flag, Some(ENV_GOOGLE_SCRAPE_ENABLED));
    }

    // === Execution tests ===

    #[tokio::test]
    async fn test_execute_missing_query() {
        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_empty_query() {
        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "query": "   "
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_execute_feature_flag_disabled() {
        // Ensure env var is not set
        std::env::remove_var(ENV_GOOGLE_SCRAPE_ENABLED);

        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "query": "test"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("disabled")
                || err.to_string().contains("CODER_LLM_TOOL_GOOGLE_SCRAPE")
        );
    }

    #[tokio::test]
    async fn test_execute_feature_flag_enabled() {
        // Set env var to enable the tool
        std::env::set_var(ENV_GOOGLE_SCRAPE_ENABLED, "1");

        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "query": "test search"
        });

        // We can't fully test execution without a real HTTP server,
        // but we can verify it doesn't fail on feature flag check
        let result = tool.execute(context, args, app_handle, None).await;

        // Will fail on HTTP request, but not on feature flag
        match result {
            Ok(_) => {}
            Err(e) => {
                // Should not be a disabled error
                assert!(!e.to_string().contains("disabled"));
            }
        }

        std::env::remove_var(ENV_GOOGLE_SCRAPE_ENABLED);
    }

    #[tokio::test]
    async fn test_execute_num_results_clamping() {
        std::env::set_var(ENV_GOOGLE_SCRAPE_ENABLED, "1");

        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Test above max
        let args = serde_json::json!({
            "query": "test",
            "num_results": 15
        });

        let _ = tool.execute(context, args, app_handle, None).await;

        std::env::remove_var(ENV_GOOGLE_SCRAPE_ENABLED);
    }

    #[tokio::test]
    async fn test_execute_num_results_below_min() {
        std::env::set_var(ENV_GOOGLE_SCRAPE_ENABLED, "1");

        let tool = GoogleScrapeTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "query": "test",
            "num_results": 0
        });

        let _ = tool.execute(context, args, app_handle, None).await;

        std::env::remove_var(ENV_GOOGLE_SCRAPE_ENABLED);
    }

    // === SearchResult and Output tests ===

    #[test]
    fn test_search_result_fields() {
        let result = SearchResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            snippet: "Test snippet".to_string(),
            source: "test".to_string(),
        };

        assert_eq!(result.title, "Test Title");
        assert_eq!(result.url, "https://example.com");
        assert_eq!(result.snippet, "Test snippet");
        assert_eq!(result.source, "test");
    }

    #[test]
    fn test_google_search_output_serialization() {
        let output = GoogleSearchOutput {
            query: "test query".to_string(),
            count: 2,
            results: vec![
                SearchResult {
                    title: "Title 1".to_string(),
                    url: "https://example1.com".to_string(),
                    snippet: "Snippet 1".to_string(),
                    source: "google_scrape".to_string(),
                },
                SearchResult {
                    title: "Title 2".to_string(),
                    url: "https://example2.com".to_string(),
                    snippet: "Snippet 2".to_string(),
                    source: "google_scrape".to_string(),
                },
            ],
            warning: "Test warning".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["query"], "test query");
        assert_eq!(json["count"], 2);
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
        assert!(json["warning"].as_str().unwrap().contains("warning"));
    }

    #[test]
    fn test_google_search_output_empty_results() {
        let output = GoogleSearchOutput {
            query: "test".to_string(),
            count: 0,
            results: vec![],
            warning: "warning".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["count"], 0);
        assert!(json["results"].as_array().unwrap().is_empty());
    }

    // === Edge case tests ===

    #[test]
    fn test_parse_google_search_results_unicode() {
        let html = r#"
            <a href="/url?q=https://example.com">
                <h3>世界テスト 🌍</h3>
            </a>
        "#;

        let results = GoogleScrapeTool::parse_google_search_results(html, 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("世界"));
    }

    #[test]
    fn test_strip_html_tags_preserves_unicode() {
        let input = "<p>Hello 世界 🌍</p>";
        let output = GoogleScrapeTool::strip_html_tags(input);
        assert_eq!(output, "Hello 世界 🌍");
    }

    #[test]
    fn test_decode_html_entities_unicode() {
        // Test that unicode is preserved through entity decoding
        let input = "世界 🌍 &amp; more";
        let output = GoogleScrapeTool::decode_html_entities(input);
        assert_eq!(output, "世界 🌍 & more");
    }
}
