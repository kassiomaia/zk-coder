//! API validation and model fetching for LLM providers.
//!
//! This module provides functions to validate API keys and fetch models
//! from various LLM providers (OpenAI, Anthropic, Google).

use reqwest::Client;
use serde::Deserialize;

use crate::nlp::config::{ApiType, FetchModelsResult, Model, ModelCapabilities, ValidationResult};

/// Default timeout for API requests (in seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Creates a configured HTTP client.
fn create_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

// =============================================================================
// OpenAI API
// =============================================================================

/// Validates an OpenAI API key.
///
/// Uses a minimal chat completion request to validate.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_openai_key(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    let client = create_client()?;

    // Handle base_url that might already include the endpoint
    let base = base_url.unwrap_or("https://api.openai.com/v1");
    let url = if base.ends_with("/chat/completions") || base.contains("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base.trim_end_matches('/'))
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();

    if status.as_u16() == 401 {
        return Ok(ValidationResult::failure("Invalid API key"));
    }

    if status.is_success() || status.as_u16() == 400 {
        // 400 might be model not found, but key is valid
        return Ok(ValidationResult::success(
            "API key validated successfully",
        ));
    }

    // Get error details
    let error_body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read error details".to_string());

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_body) {
        if let Some(error_msg) = json["error"]["message"].as_str() {
            return Ok(ValidationResult::failure(format!(
                "OpenAI API error ({}): {}",
                status.as_u16(),
                error_msg
            )));
        }
    }

    Ok(ValidationResult::failure(format!(
        "Validation failed ({}): {}",
        status.as_u16(),
        error_body.chars().take(200).collect::<String>()
    )))
}

/// Fetches available models from OpenAI.
///
/// # Errors
///
/// Returns an error string if the fetch fails.
pub async fn fetch_openai_models(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<FetchModelsResult, String> {
    let client = create_client()?;

    // Handle base_url that might already include the endpoint
    let base = base_url.unwrap_or("https://api.openai.com/v1");
    let url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{}/models", base.trim_end_matches('/'))
    };

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error details".to_string());

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_body) {
            if let Some(error_msg) = json["error"]["message"].as_str() {
                return Ok(FetchModelsResult::failure(format!(
                    "OpenAI API error ({}): {}",
                    status.as_u16(),
                    error_msg
                )));
            }
            if let Some(error_msg) = json["error"].as_str() {
                return Ok(FetchModelsResult::failure(format!(
                    "OpenAI API error ({}): {}",
                    status.as_u16(),
                    error_msg
                )));
            }
        }

        return Ok(FetchModelsResult::failure(format!(
            "API error ({}): {}",
            status.as_u16(),
            error_body.chars().take(200).collect::<String>()
        )));
    }

    let api_response = response
        .json::<OpenAiModelsResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models: Vec<String> = api_response
        .data
        .into_iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            id.contains("gpt") || id.contains("o1") || id.contains("o3")
        })
        .map(|m| m.id)
        .collect();

    Ok(FetchModelsResult::success(models))
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

// =============================================================================
// Google API (OpenAI Compatible)
// =============================================================================

/// Validates a Google API key.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_google_key(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    let client = create_client()?;

    // Handle base_url that might already include the endpoint
    let base = base_url.unwrap_or("https://generativelanguage.googleapis.com/v1beta/openai");
    let url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{}/models", base.trim_end_matches('/'))
    };

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        Ok(ValidationResult::success(
            "API key validated successfully",
        ))
    } else {
        let status = response.status();
        // Try to get error details from response body
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error details".to_string());

        // Parse Google error format if possible
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_body) {
            if let Some(error_msg) = json["error"]["message"].as_str() {
                return Ok(ValidationResult::failure(format!(
                    "Google API error ({}): {}",
                    status.as_u16(),
                    error_msg
                )));
            }
        }

        // Fallback to status + body
        Ok(ValidationResult::failure(format!(
            "Validation failed ({}): {}",
            status.as_u16(),
            error_body.chars().take(200).collect::<String>()
        )))
    }
}

/// Fetches available models from Google.
///
/// # Errors
///
/// Returns an error string if the fetch fails.
pub async fn fetch_google_models(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<FetchModelsResult, String> {
    let client = create_client()?;

    // Handle base_url that might already include the endpoint
    let base = base_url.unwrap_or("https://generativelanguage.googleapis.com/v1beta/openai");
    let url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{}/models", base.trim_end_matches('/'))
    };

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        // Try to get error details from response body
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error details".to_string());

        // Parse Google error format if possible
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_body) {
            if let Some(error_msg) = json["error"]["message"].as_str() {
                return Ok(FetchModelsResult::failure(format!(
                    "Google API error ({}): {}",
                    status.as_u16(),
                    error_msg
                )));
            }
        }

        // Fallback to status + body
        return Ok(FetchModelsResult::failure(format!(
            "API error ({}): {}",
            status.as_u16(),
            error_body.chars().take(200).collect::<String>()
        )));
    }

    let api_response = response
        .json::<OpenAiModelsResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models: Vec<Model> = api_response
        .data
        .into_iter()
        .filter(|m| {
            // Filter for Gemini models
            let id = m.id.to_lowercase();
            id.contains("gemini")
        })
        .map(|m| {
            let caps = ModelCapabilities::infer_from_name(&m.id);
            Model::new("google", &m.id, caps)
        })
        .collect();

    let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();

    Ok(FetchModelsResult::success(model_names))
}

// =============================================================================
// Anthropic API
// =============================================================================

/// Validates an Anthropic API key.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_anthropic_key(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    let client = create_client()?;
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com")
    );

    // Anthropic doesn't have a validation endpoint, so we make a minimal request
    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-haiku-20240307",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    // 200 or 400 (rate limit) means key is valid
    // 401 means invalid key
    if status.as_u16() == 401 {
        Ok(ValidationResult::failure("Invalid API key"))
    } else {
        Ok(ValidationResult::success(
            "API key validated successfully",
        ))
    }
}

/// Gets known Anthropic models (Anthropic doesn't have a models endpoint).
///
/// # Errors
///
/// Returns an error string if the operation fails.
pub fn get_anthropic_models() -> FetchModelsResult {
    let models = vec![
        "claude-opus-4-20250514",
        "claude-sonnet-4-20250514",
        "claude-3-5-sonnet-20241022",
        "claude-3-5-haiku-20241022",
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
    ];

    FetchModelsResult::success(models.iter().map(|s| s.to_string()).collect())
}

// =============================================================================
// Unified API
// =============================================================================

/// Validates an API key with the appropriate provider.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_api_key(
    api_type: ApiType,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    match api_type {
        ApiType::OpenAI => validate_openai_key(api_key, base_url).await,
        ApiType::Anthropic => validate_anthropic_key(api_key, base_url).await,
        ApiType::Google => validate_google_key(api_key, base_url).await,
    }
}

/// Tests connection to a custom provider.
///
/// # Errors
///
/// Returns an error string if the connection test fails.
pub async fn test_connection(
    api_type: ApiType,
    api_key: &str,
    base_url: &str,
) -> Result<ValidationResult, String> {
    match api_type {
        ApiType::OpenAI => validate_openai_key(api_key, Some(base_url)).await,
        ApiType::Anthropic => validate_anthropic_key(api_key, Some(base_url)).await,
        ApiType::Google => validate_google_key(api_key, Some(base_url)).await,
    }
}

/// Fetches models from the appropriate provider.
///
/// # Errors
///
/// Returns an error string if the fetch fails.
pub async fn fetch_models(
    api_type: ApiType,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<FetchModelsResult, String> {
    match api_type {
        ApiType::OpenAI => fetch_openai_models(api_key, base_url).await,
        ApiType::Anthropic => Ok(get_anthropic_models()),
        ApiType::Google => fetch_google_models(api_key, base_url).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_anthropic_models() {
        let result = get_anthropic_models();
        assert!(result.success);
        assert!(!result.models.is_empty());
        assert!(result.models.iter().any(|m| m.contains("claude")));
    }

    #[test]
    fn test_infer_capabilities() {
        let caps = ModelCapabilities::infer_from_name("gpt-4-vision-preview");
        assert!(caps.vision);

        let caps = ModelCapabilities::infer_from_name("claude-3-opus");
        assert!(caps.vision);
    }
}
